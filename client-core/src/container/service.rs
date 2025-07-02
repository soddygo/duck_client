use super::types::{DockerManager, ServiceInfo, ServiceStatus};
use crate::constants::timeout;
use crate::{DuckError, Result};
use ducker::docker::{container::DockerContainer, util::new_local_docker_connection};
use tracing::{error, info};

impl DockerManager {
    /// 启动所有服务
    pub async fn start_services(&self) -> Result<()> {
        self.check_prerequisites().await?;

        let output = self.run_compose_command(&["up", "-d"]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DuckError::Docker(format!("启动服务失败: {stderr}")));
        }

        // 等待服务启动并验证状态
        self.verify_services_started(None).await?;

        Ok(())
    }

    /// 停止所有服务
    pub async fn stop_services(&self) -> Result<()> {
        self.check_prerequisites().await?;

        let output = self.run_compose_command(&["down"]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DuckError::Docker(format!("停止服务失败: {stderr}")));
        }

        Ok(())
    }

    /// 重启所有服务
    pub async fn restart_services(&self) -> Result<()> {
        self.stop_services().await?;
        self.start_services().await?;
        Ok(())
    }

    /// 重启单个服务
    pub async fn restart_service(&self, service_name: &str) -> Result<()> {
        self.check_prerequisites().await?;

        // 先停止指定服务
        let output = self.run_compose_command(&["stop", service_name]).await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DuckError::Docker(format!(
                "停止服务 {service_name} 失败: {stderr}"
            )));
        }

        // 再启动指定服务
        let output = self.run_compose_command(&["start", service_name]).await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DuckError::Docker(format!(
                "启动服务 {service_name} 失败: {stderr}"
            )));
        }

        Ok(())
    }

    /// 获取服务状态 - 使用 ducker 库实现，只返回docker-compose中定义的服务
    pub async fn get_services_status(&self) -> Result<Vec<ServiceInfo>> {
        self.check_prerequisites().await?;

        info!("使用 ducker 库获取容器状态...");

        // 1. 获取docker-compose.yml中定义的服务名称
        let compose_services = self.get_compose_service_names().await?;
        info!("docker-compose.yml 中定义的服务: {:?}", compose_services);

        // 2. 获取所有容器信息
        let containers = self.get_all_containers_with_ducker().await?;
        info!("系统中发现 {} 个容器", containers.len());

        // 3. 只保留与compose服务匹配的容器
        let mut matched_services = Vec::new();
        let mut compose_services_found = std::collections::HashSet::new();

        for container in containers {
            // 检查该容器是否属于任何compose服务
            for service_name in &compose_services {
                if self.is_service_name_match(&container.names, service_name) {
                    let service_info =
                        self.convert_docker_container_to_service_info(container.clone());
                    matched_services.push(service_info);
                    compose_services_found.insert(service_name.clone());
                    break; // 避免重复匹配
                }
            }
        }

        // 4. 为未找到的compose服务创建"已停止"状态的条目
        // 如果在compose文件中定义但没有找到对应容器，说明服务已停止
        for service_name in &compose_services {
            if !compose_services_found.contains(service_name) {
                matched_services.push(ServiceInfo {
                    name: service_name.clone(),
                    status: crate::container::ServiceStatus::Stopped, // 改为Stopped而不是Unknown
                    image: "未启动".to_string(), // 更准确的描述
                    ports: Vec::new(),
                });
            }
        }

        info!(
            "匹配到 {}/{} 个compose服务容器",
            compose_services_found.len(),
            compose_services.len()
        );

        Ok(matched_services)
    }

    /// 获取所有容器状态（包括非compose容器）- 保留原有功能
    pub async fn get_all_containers_status(&self) -> Result<Vec<ServiceInfo>> {
        self.check_prerequisites().await?;

        info!("使用 ducker 库获取所有容器状态...");

        // 获取所有容器信息
        let containers = self.get_all_containers_with_ducker().await?;

        // 转换为 ServiceInfo 格式
        let services = containers
            .into_iter()
            .map(|container| self.convert_docker_container_to_service_info(container))
            .collect();

        Ok(services)
    }

    /// 使用 ducker 库获取所有容器信息
    async fn get_all_containers_with_ducker(&self) -> Result<Vec<DockerContainer>> {
        match new_local_docker_connection("/var/run/docker.sock", None).await {
            Ok(docker) => match DockerContainer::list(&docker).await {
                Ok(containers) => {
                    info!("ducker 成功获取到 {} 个容器", containers.len());
                    Ok(containers)
                }
                Err(e) => {
                    error!("ducker 获取容器列表失败: {}", e);
                    Err(DuckError::Docker(format!("获取容器列表失败: {e}")))
                }
            },
            Err(e) => {
                error!("ducker 连接 Docker 失败: {}", e);
                Err(DuckError::Docker(format!("连接 Docker 失败: {e}")))
            }
        }
    }

    /// 将 DockerContainer 转换为 ServiceInfo
    fn convert_docker_container_to_service_info(&self, container: DockerContainer) -> ServiceInfo {
        let status = if container.running {
            ServiceStatus::Running
        } else {
            // 根据状态字符串进一步判断
            match container.status.to_lowercase().as_str() {
                s if s.contains("exited") => ServiceStatus::Stopped,
                s if s.contains("created") => ServiceStatus::Stopped,
                s if s.contains("restarting") => ServiceStatus::Unknown,
                s if s.contains("paused") => ServiceStatus::Stopped,
                s if s.contains("dead") => ServiceStatus::Stopped,
                _ => ServiceStatus::Unknown,
            }
        };

        // 解析端口映射
        let ports = if container.ports.is_empty() {
            Vec::new()
        } else {
            container
                .ports
                .split(", ")
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string())
                .collect()
        };

        ServiceInfo {
            name: container.names.clone(),
            status,
            image: container.image.clone(),
            ports,
        }
    }

    /// 获取服务状态 - 保留旧方法作为备选
    pub async fn get_services_status_with_compose(&self) -> Result<Vec<ServiceInfo>> {
        self.check_prerequisites().await?;

        let output = self
            .run_compose_command(&["ps", "--format", "json"])
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DuckError::Docker(format!("获取服务状态失败: {stderr}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_service_info(&stdout)
    }

    /// 检查单个服务是否正在运行 - 使用 ducker 实现
    pub async fn is_service_running(&self, service_name: &str) -> Result<bool> {
        let services = self.get_services_status().await?;

        for service in services {
            if self.is_service_name_match(&service.name, service_name) {
                return Ok(service.status == ServiceStatus::Running);
            }
        }

        Ok(false)
    }

    /// 判断容器是否属于指定的compose服务
    /// 使用docker-compose的容器命名规则进行匹配
    fn is_service_name_match(&self, container_name: &str, service_name: &str) -> bool {
        // 生成可能的容器名称模式
        let patterns = self.generate_compose_container_patterns(service_name);

        let container_lower = container_name.to_lowercase();

        // 检查容器名称是否匹配任何模式
        for pattern in patterns {
            let pattern_lower = pattern.to_lowercase();

            // 精确匹配
            if container_lower == pattern_lower {
                return true;
            }

            // 前缀匹配（处理有额外后缀的情况）
            if container_lower.starts_with(&pattern_lower) {
                return true;
            }
        }

        // 如果没有匹配到，检查是否是包含关系
        let service_lower = service_name.to_lowercase();
        if container_lower.contains(&service_lower) {
            return true;
        }

        false
    }

    /// 获取特定服务的详细信息
    pub async fn get_service_detail(&self, service_name: &str) -> Result<Option<ServiceInfo>> {
        let services = self.get_services_status().await?;

        for service in services {
            if self.is_service_name_match(&service.name, service_name) {
                return Ok(Some(service));
            }
        }

        Ok(None)
    }

    /// 获取所有正在运行的服务
    pub async fn get_running_services(&self) -> Result<Vec<ServiceInfo>> {
        let services = self.get_services_status().await?;
        Ok(services
            .into_iter()
            .filter(|service| service.status == ServiceStatus::Running)
            .collect())
    }

    /// 获取所有失败的服务
    pub async fn get_failed_services(&self) -> Result<Vec<ServiceInfo>> {
        let services = self.get_services_status().await?;
        Ok(services
            .into_iter()
            .filter(|service| service.status == ServiceStatus::Stopped)
            .collect())
    }

    /// 获取服务日志
    pub async fn get_logs(&self, service_name: Option<&str>, lines: Option<u32>) -> Result<String> {
        self.check_prerequisites().await?;

        let mut args = vec!["logs"];
        let lines_str;
        if let Some(n) = lines {
            args.push("--tail");
            lines_str = n.to_string();
            args.push(&lines_str);
        }
        if let Some(service) = service_name {
            args.push(service);
        }

        let output = self.run_compose_command(&args).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DuckError::Docker(format!("获取日志失败: {stderr}")));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// 检查所有服务的健康状况
    pub async fn check_services_health(&self) -> Result<()> {
        let services = self.get_services_status().await?;

        if services.is_empty() {
            return Err(DuckError::Docker("没有找到任何服务".to_string()));
        }

        let mut unhealthy_services = Vec::new();
        for service in services {
            if service.status != ServiceStatus::Running {
                unhealthy_services.push(service.name);
            }
        }

        if !unhealthy_services.is_empty() {
            return Err(DuckError::Docker(format!(
                "部分服务未在运行: {}",
                unhealthy_services.join(", ")
            )));
        }

        Ok(())
    }

    /// 等待并验证服务启动（使用默认超时时间）
    pub async fn wait_for_services_started(&self) -> Result<()> {
        self.verify_services_started(None).await
    }

    /// 等待并验证服务启动（用于部署场景，使用更长的超时时间）
    pub async fn wait_for_services_started_after_deploy(&self) -> Result<()> {
        self.verify_services_started(Some(timeout::DEPLOY_START_TIMEOUT))
            .await
    }

    /// 等待并验证服务启动（使用自定义超时时间）
    pub async fn wait_for_services_started_with_timeout(&self, timeout_secs: u64) -> Result<()> {
        self.verify_services_started(Some(timeout_secs)).await
    }

    /// 验证服务启动状态（启动后等待并检查实际状态）
    ///
    /// # 参数
    /// * `custom_timeout` - 自定义超时时间（秒），如果为None则使用默认的SERVICE_START_TIMEOUT
    async fn verify_services_started(&self, custom_timeout: Option<u64>) -> Result<()> {
        use tokio::time::{Duration, sleep};

        // 使用统一的常量配置
        let max_wait_time =
            Duration::from_secs(custom_timeout.unwrap_or(timeout::SERVICE_START_TIMEOUT));
        let check_interval = Duration::from_secs(timeout::SERVICE_CHECK_INTERVAL);
        let max_attempts = max_wait_time.as_secs() / check_interval.as_secs();

        for attempt in 1..=max_attempts {
            tracing::debug!("验证服务状态，第 {} 次尝试", attempt);

            // 获取当前服务状态
            match self.get_services_status().await {
                Ok(services) => {
                    if services.is_empty() {
                        tracing::warn!("没有找到任何服务，可能compose文件没有定义服务");
                        return Ok(()); // 允许空服务情况
                    }

                    // 检查是否有必须运行的服务
                    let mut failed_services = Vec::new();
                    let mut pending_services = Vec::new();

                    for service in &services {
                        match service.status {
                            ServiceStatus::Running => {
                                // 服务正在运行，很好
                                tracing::debug!("服务 {} 运行正常", service.name);
                            }
                            ServiceStatus::Stopped => {
                                // 检查这是否是一次性任务服务
                                if self
                                    .is_oneshot_service(&service.name)
                                    .await
                                    .unwrap_or(false)
                                {
                                    tracing::debug!(
                                        "服务 {} 是一次性任务，已正常退出",
                                        service.name
                                    );
                                } else {
                                    failed_services.push(service.name.clone());
                                }
                            }
                            ServiceStatus::Unknown => {
                                pending_services.push(service.name.clone());
                            }
                        }
                    }

                    // 如果没有失败的服务且没有待定的服务，说明启动成功
                    if failed_services.is_empty() && pending_services.is_empty() {
                        tracing::info!("所有服务启动验证成功");
                        return Ok(());
                    }

                    // 如果有失败的服务，记录但继续等待（可能需要更多时间）
                    if !failed_services.is_empty() {
                        tracing::warn!("服务启动失败: {}", failed_services.join(", "));
                    }

                    if !pending_services.is_empty() {
                        tracing::debug!("等待服务启动: {}", pending_services.join(", "));
                    }

                    // 如果是最后一次尝试，返回错误
                    if attempt == max_attempts {
                        let mut error_msg = String::new();
                        if !failed_services.is_empty() {
                            error_msg.push_str(&format!(
                                "启动失败的服务: {}",
                                failed_services.join(", ")
                            ));
                        }
                        if !pending_services.is_empty() {
                            if !error_msg.is_empty() {
                                error_msg.push_str("; ");
                            }
                            error_msg.push_str(&format!(
                                "启动超时的服务: {}",
                                pending_services.join(", ")
                            ));
                        }
                        return Err(DuckError::Docker(format!("服务启动验证失败: {error_msg}")));
                    }
                }
                Err(e) => {
                    tracing::warn!("获取服务状态失败: {}", e);
                    if attempt == max_attempts {
                        return Err(DuckError::Docker(format!("无法获取服务状态: {e}")));
                    }
                }
            }

            // 等待下次检查
            sleep(check_interval).await;
        }

        Ok(())
    }

    /// 解析服务信息
    pub(crate) fn parse_service_info(&self, json_output: &str) -> Result<Vec<ServiceInfo>> {
        let mut services = Vec::new();

        // 如果输出为空，返回空列表
        if json_output.trim().is_empty() {
            return Ok(services);
        }

        // 尝试按行解析 JSON
        for line in json_output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(service_json) => {
                    let name = service_json["Name"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string();

                    let state = service_json["State"].as_str().unwrap_or("unknown");

                    let status = match state {
                        "running" => ServiceStatus::Running,
                        "exited" | "stopped" => ServiceStatus::Stopped,
                        _ => ServiceStatus::Unknown,
                    };

                    let image = service_json["Image"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string();

                    let ports = service_json["Ports"]
                        .as_str()
                        .unwrap_or("")
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    services.push(ServiceInfo {
                        name,
                        status,
                        image,
                        ports,
                    });
                }
                Err(e) => {
                    tracing::warn!("解析服务 JSON 失败: {}, 内容: {}", e, line);
                }
            }
        }

        Ok(services)
    }
}
