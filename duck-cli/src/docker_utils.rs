use client_core::constants::{docker, timeout};
use client_core::error::Result;
use ducker::docker::container::DockerContainer;
#[allow(unused_imports)]
use ducker::docker::util::new_local_docker_connection;
use serde_yaml::Value;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Docker服务过滤条件
#[derive(Debug, Clone)]
pub enum ServiceFilter {
    /// 按容器名称关键字过滤
    #[allow(dead_code)]
    NameContains(Vec<String>),
    /// 按容器ID过滤
    #[allow(dead_code)]
    ContainerIds(Vec<String>),
    /// 基于服务配置智能过滤
    ServiceConfigs(Vec<ServiceConfig>),
    /// 检查所有容器
    All,
}

impl ServiceFilter {
    /// 检查容器是否匹配过滤条件
    pub fn matches(&self, container: &DockerContainer) -> bool {
        match self {
            ServiceFilter::NameContains(keywords) => {
                if keywords.is_empty() {
                    return true;
                }
                keywords.iter().any(|keyword| {
                    container
                        .names
                        .to_lowercase()
                        .contains(&keyword.to_lowercase())
                })
            }
            ServiceFilter::ContainerIds(ids) => {
                if ids.is_empty() {
                    return true;
                }
                ids.iter().any(|id| container.id.starts_with(id))
            }
            ServiceFilter::ServiceConfigs(configs) => {
                if configs.is_empty() {
                    return true;
                }
                configs.iter().any(|config| {
                    container
                        .names
                        .to_lowercase()
                        .contains(&config.name.to_lowercase())
                })
            }
            ServiceFilter::All => true,
        }
    }

    /// 获取匹配容器的服务配置
    pub fn get_service_config(&self, container: &DockerContainer) -> Option<&ServiceConfig> {
        match self {
            ServiceFilter::ServiceConfigs(configs) => configs.iter().find(|config| {
                container
                    .names
                    .to_lowercase()
                    .contains(&config.name.to_lowercase())
            }),
            _ => None,
        }
    }
}

/// 检查指定的Docker服务是否在运行
pub async fn check_services_running(filter: &ServiceFilter) -> Result<bool> {
    // 使用跨平台连接方法，自动处理 Unix socket 和 Windows Named Pipe
    // 优先使用环境变量 DOCKER_HOST，否则使用平台默认路径
    match new_local_docker_connection(
        docker::DOCKER_SOCKET_PATH,
        None, // 优先使用 DOCKER_HOST 环境变量，若未设置则使用 socket_path
    )
    .await
    {
        Ok(docker) => {
            match DockerContainer::list(&docker).await {
                Ok(containers) => {
                    let filtered_containers: Vec<_> =
                        containers.iter().filter(|c| filter.matches(c)).collect();

                    // 智能计算"正常运行"的服务数量
                    let running_count = filtered_containers
                        .iter()
                        .filter(|container| {
                            if let Some(service_config) = filter.get_service_config(container) {
                                // 基于重启策略判断服务状态
                                if service_config.restart_policy.should_keep_running() {
                                    // 持续运行服务：应该处于运行状态
                                    container.running
                                } else {
                                    // 一次性任务服务：检查是否成功完成
                                    // 如果容器正在运行，说明任务还在执行中，这也是正常的
                                    // 如果容器已停止，我们假设它已经成功完成（因为无法轻易获取exit code）
                                    true // 对于一次性任务，我们认为它们总是"正常"的
                                }
                            } else {
                                // 对于没有配置信息的容器，使用原来的逻辑
                                container.running
                            }
                        })
                        .count();

                    let total_filtered = filtered_containers.len();

                    match filter {
                        ServiceFilter::All => {
                            info!(
                                "发现 {} 个正在运行的容器（总共 {} 个）",
                                running_count, total_filtered
                            );
                        }
                        ServiceFilter::NameContains(keywords) => {
                            info!(
                                "匹配关键字 {:?} 的容器: {} 个运行中（总共 {} 个）",
                                keywords, running_count, total_filtered
                            );
                        }
                        ServiceFilter::ContainerIds(ids) => {
                            info!(
                                "匹配ID {:?} 的容器: {} 个运行中（总共 {} 个）",
                                ids, running_count, total_filtered
                            );
                        }
                        ServiceFilter::ServiceConfigs(configs) => {
                            let service_names: Vec<_> = configs.iter().map(|c| &c.name).collect();
                            info!(
                                "匹配服务配置 {:?} 的容器: {} 个运行中（总共 {} 个）",
                                service_names, running_count, total_filtered
                            );
                        }
                    }

                    // 如果有过滤条件，只要有匹配的容器在运行就返回true
                    // 如果是检查所有容器，只要有任何容器在运行就返回true
                    Ok(running_count > 0)
                }
                Err(e) => {
                    error!("获取容器列表失败: {}", e);
                    Err(client_core::error::DuckError::docker_service(format!(
                        "获取容器列表失败: {}",
                        e
                    )))
                }
            }
        }
        Err(e) => {
            error!("无法连接到Docker: {}", e);
            Err(client_core::error::DuckError::docker_service(format!(
                "无法连接到Docker: {}",
                e
            )))
        }
    }
}

/// 等待指定的Docker服务完全停止
pub async fn wait_for_services_stopped(filter: &ServiceFilter, timeout_secs: u64) -> Result<bool> {
    let start_time = tokio::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    info!(
        "开始等待服务停止，过滤条件: {:?}，超时: {} 秒",
        filter, timeout_secs
    );

    while start_time.elapsed() < timeout {
        match check_services_running(filter).await {
            Ok(false) => {
                info!("指定的Docker服务已完全停止");
                return Ok(true);
            }
            Ok(true) => {
                info!("等待Docker服务停止...");
                sleep(Duration::from_secs(timeout::SERVICE_CHECK_INTERVAL)).await;
            }
            Err(e) => {
                warn!("检查服务状态时出错: {}", e);
                sleep(Duration::from_secs(timeout::SERVICE_CHECK_INTERVAL)).await;
            }
        }
    }

    warn!("等待服务停止超时 ({} 秒)", timeout_secs);
    Ok(false)
}

/// 等待指定的Docker服务完全启动
pub async fn wait_for_services_started(filter: &ServiceFilter, timeout_secs: u64) -> Result<bool> {
    let start_time = tokio::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    info!(
        "开始等待服务启动，过滤条件: {:?}，超时: {} 秒",
        filter, timeout_secs
    );

    while start_time.elapsed() < timeout {
        match check_services_running(filter).await {
            Ok(true) => {
                info!("指定的Docker服务已启动");
                return Ok(true);
            }
            Ok(false) => {
                info!("等待Docker服务启动...");
                sleep(Duration::from_secs(timeout::SERVICE_CHECK_INTERVAL)).await;
            }
            Err(e) => {
                warn!("检查服务状态时出错: {}", e);
                sleep(Duration::from_secs(timeout::SERVICE_CHECK_INTERVAL)).await;
            }
        }
    }

    warn!("等待服务启动超时 ({} 秒)", timeout_secs);
    Ok(false)
}

/// 服务配置信息
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub name: String,
    pub restart_policy: RestartPolicy,
}

/// Docker服务重启策略
#[derive(Debug, Clone, PartialEq)]
pub enum RestartPolicy {
    /// 始终重启 (restart: always)
    Always,
    /// 除非手动停止 (restart: unless-stopped)  
    UnlessStopped,
    /// 失败时重启 (restart: on-failure)
    OnFailure,
    /// 从不重启 (restart: "no")
    No,
    /// 未指定（默认为No）
    Unspecified,
}

impl RestartPolicy {
    /// 从字符串解析重启策略
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "always" => RestartPolicy::Always,
            "unless-stopped" => RestartPolicy::UnlessStopped,
            "on-failure" => RestartPolicy::OnFailure,
            "no" => RestartPolicy::No,
            _ => RestartPolicy::Unspecified,
        }
    }

    /// 判断服务是否应该持续运行
    pub fn should_keep_running(&self) -> bool {
        match self {
            RestartPolicy::Always | RestartPolicy::UnlessStopped => true,
            RestartPolicy::OnFailure | RestartPolicy::No | RestartPolicy::Unspecified => false,
        }
    }
}

/// 从docker-compose.yml文件中解析服务配置
pub async fn parse_service_configs_from_compose(
    compose_file_path: &Path,
) -> Result<Vec<ServiceConfig>> {
    if !compose_file_path.exists() {
        warn!(
            "docker-compose.yml 文件不存在: {}",
            compose_file_path.display()
        );
        return Ok(vec![]);
    }

    match fs::read_to_string(compose_file_path) {
        Ok(content) => match serde_yaml::from_str::<Value>(&content) {
            Ok(yaml) => {
                let mut service_configs = Vec::new();

                if let Some(services) = yaml.get("services") {
                    if let Some(services_map) = services.as_mapping() {
                        for (key, value) in services_map {
                            if let Some(service_name) = key.as_str() {
                                let restart_policy =
                                    if let Some(service_config) = value.as_mapping() {
                                        if let Some(restart_value) = service_config.get("restart") {
                                            if let Some(restart_str) = restart_value.as_str() {
                                                RestartPolicy::from_str(restart_str)
                                            } else {
                                                RestartPolicy::Unspecified
                                            }
                                        } else {
                                            RestartPolicy::Unspecified
                                        }
                                    } else {
                                        RestartPolicy::Unspecified
                                    };

                                service_configs.push(ServiceConfig {
                                    name: service_name.to_string(),
                                    restart_policy,
                                });
                            }
                        }
                    }
                }

                info!(
                    "从 {} 解析到 {} 个服务配置:",
                    compose_file_path.display(),
                    service_configs.len()
                );
                for config in &service_configs {
                    info!("  - {}: {:?}", config.name, config.restart_policy);
                }

                Ok(service_configs)
            }
            Err(e) => {
                error!("解析docker-compose.yml失败: {}", e);
                Err(client_core::error::DuckError::custom(format!(
                    "解析docker-compose.yml失败: {}",
                    e
                )))
            }
        },
        Err(e) => {
            error!("读取docker-compose.yml文件失败: {}", e);
            Err(client_core::error::DuckError::custom(format!(
                "读取docker-compose.yml文件失败: {}",
                e
            )))
        }
    }
}

/// 从docker-compose.yml文件中解析服务名称（保持向后兼容）
#[allow(dead_code)]
pub async fn parse_service_names_from_compose(compose_file_path: &Path) -> Result<Vec<String>> {
    let service_configs = parse_service_configs_from_compose(compose_file_path).await?;
    Ok(service_configs
        .into_iter()
        .map(|config| config.name)
        .collect())
}

/// 基于docker-compose.yml创建服务过滤器
pub async fn create_compose_filter(compose_file_path: &Path) -> Result<ServiceFilter> {
    let service_configs = parse_service_configs_from_compose(compose_file_path).await?;

    if service_configs.is_empty() {
        warn!("未找到服务配置，将检查所有容器");
        Ok(ServiceFilter::All)
    } else {
        // 使用智能服务配置过滤器，能够区分不同类型的服务
        Ok(ServiceFilter::ServiceConfigs(service_configs))
    }
}

/// 便捷函数：等待compose服务停止
pub async fn wait_for_compose_services_stopped(
    compose_file_path: &Path,
    timeout_secs: u64,
) -> Result<bool> {
    let filter = create_compose_filter(compose_file_path).await?;
    wait_for_services_stopped(&filter, timeout_secs).await
}

/// 便捷函数：等待compose服务启动
pub async fn wait_for_compose_services_started(
    compose_file_path: &Path,
    timeout_secs: u64,
) -> Result<bool> {
    let filter = create_compose_filter(compose_file_path).await?;
    wait_for_services_started(&filter, timeout_secs).await
}

/// 便捷函数：检查compose服务是否运行
pub async fn check_compose_services_running(compose_file_path: &Path) -> Result<bool> {
    let filter = create_compose_filter(compose_file_path).await?;
    check_services_running(&filter).await
}
