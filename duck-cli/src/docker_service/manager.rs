use crate::docker_service::architecture::{Architecture, detect_architecture};
use crate::docker_service::error::{DockerServiceError, DockerServiceResult};
use crate::docker_service::health_check::{HealthChecker, HealthReport};
use crate::docker_service::image_loader::{ImageLoader, LoadResult, TagResult};
use crate::docker_service::port_manager::{PortConflictReport, PortManager};
use client_core::config::AppConfig;
use client_core::constants::timeout;
use client_core::container::DockerManager;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info, warn};

/// Docker 服务管理器
pub struct DockerServiceManager {
    #[allow(dead_code)]
    config: AppConfig,
    docker_manager: DockerManager,
    work_dir: PathBuf,
    architecture: Architecture,
    image_loader: ImageLoader,
    health_checker: HealthChecker,
    port_manager: PortManager,
}

impl DockerServiceManager {
    /// 创建新的 Docker 服务管理器
    pub fn new(config: AppConfig, docker_manager: DockerManager, work_dir: PathBuf) -> Self {
        let architecture = detect_architecture();

        // 由于 DockerManager 实现了 Clone，我们可以安全地克隆它
        let image_loader = ImageLoader::new(docker_manager.clone(), work_dir.clone())
            .expect("Failed to create image loader");
        let health_checker = HealthChecker::new(docker_manager.clone());

        Self {
            config,
            docker_manager,
            work_dir,
            architecture,
            image_loader,
            health_checker,
            port_manager: PortManager::new(),
        }
    }

    /// 获取当前系统架构
    pub fn get_architecture(&self) -> Architecture {
        self.architecture
    }

    /// 获取工作目录
    pub fn get_work_dir(&self) -> &PathBuf {
        &self.work_dir
    }

    /// 执行完整的服务部署流程
    pub async fn deploy_services(&mut self) -> DockerServiceResult<()> {
        info!("开始 Docker 服务部署流程");

        // 1. 环境检查
        self.check_environment().await?;

        // 2. 设置必要目录
        self.setup_directories().await?;

        // 3. 加载镜像并获取映射信息
        let load_result = self.load_images().await?;

        // 4. 使用ducker验证并设置镜像标签（推荐方法）
        self.setup_image_tags_with_ducker_validation(&load_result.image_mappings).await?;

        // 5. 启动服务
        self.start_services().await?;

        info!("Docker 服务部署完成");
        Ok(())
    }

    /// 环境检查
    pub async fn check_environment(&self) -> DockerServiceResult<()> {
        info!("检查 Docker 环境...");

        // 检查 Docker 是否安装和运行
        self.docker_manager
            .check_docker_status()
            .await
            .map_err(|e| DockerServiceError::EnvironmentCheck(e.to_string()))?;

        // 检查工作目录
        if !self.work_dir.exists() {
            return Err(DockerServiceError::EnvironmentCheck(format!(
                "工作目录不存在: {}",
                self.work_dir.display()
            )));
        }

        // 检查镜像目录
        let images_dir = self.work_dir.join("images");
        if !images_dir.exists() {
            return Err(DockerServiceError::EnvironmentCheck(format!(
                "镜像目录不存在: {}",
                images_dir.display()
            )));
        }

        // 检查 docker-compose.yml
        let compose_file = self.work_dir.join("docker-compose.yml");
        if !compose_file.exists() {
            return Err(DockerServiceError::EnvironmentCheck(format!(
                "Docker Compose 配置文件不存在: {}",
                compose_file.display()
            )));
        }

        info!("环境检查通过");
        Ok(())
    }

    /// 设置必要目录
    pub async fn setup_directories(&self) -> DockerServiceResult<()> {
        info!("创建必要目录...");

        let directories = [
            "data",
            "data/mysql",
            "data/redis",
            "data/milvus",
            "data/milvus/data",
            "data/milvus/etcd",
            "logs",
            "logs/agent",
            "logs/mysql",
            "logs/redis",
            "logs/milvus",
            "upload",
            "config",
            "backups",
        ];

        for dir in directories {
            let dir_path = self.work_dir.join(dir);
            if !dir_path.exists() {
                info!("创建目录: {}", dir_path.display());
                tokio::fs::create_dir_all(&dir_path).await.map_err(|e| {
                    DockerServiceError::FileSystem(format!(
                        "创建目录失败 {}: {}",
                        dir_path.display(),
                        e
                    ))
                })?;
            }
        }

        info!("目录设置完成");
        Ok(())
    }

    /// 加载 Docker 镜像
    pub async fn load_images(&self) -> DockerServiceResult<LoadResult> {
        info!("开始加载 Docker 镜像...");
        let result = self.image_loader.load_all_images().await?;

        if !result.is_all_successful() {
            warn!(
                "部分镜像加载失败: 成功 {}, 失败 {}",
                result.success_count(),
                result.failure_count()
            );
        }

        Ok(result)
    }

    /// 基于实际镜像映射设置标签
    pub async fn setup_image_tags_with_mappings(&self, image_mappings: &[(String, String)]) -> DockerServiceResult<TagResult> {
        info!("开始设置镜像标签...");
        let result = self.image_loader.setup_image_tags_with_mappings(image_mappings).await?;

        if !result.is_all_successful() {
            warn!(
                "部分标签设置失败: 成功 {}, 失败 {}",
                result.success_count(),
                result.failure_count()
            );
        }

        Ok(result)
    }

    /// 基于 ducker 验证镜像后再设置标签（推荐使用）
    pub async fn setup_image_tags_with_ducker_validation(&self, image_mappings: &[(String, String)]) -> DockerServiceResult<TagResult> {
        info!("开始验证并设置镜像标签...");
        let result = self.image_loader.setup_image_tags_with_validation(image_mappings).await?;

        if !result.is_all_successful() {
            warn!(
                "部分标签设置失败: 成功 {}, 失败 {}",
                result.success_count(),
                result.failure_count()
            );
        }

        Ok(result)
    }

    /// 使用 ducker 列出当前系统中的所有镜像
    pub async fn list_docker_images_with_ducker(&self) -> DockerServiceResult<Vec<String>> {
        info!("使用 ducker 获取镜像列表...");
        self.image_loader.list_images_with_ducker().await
    }

    /// 设置镜像标签（传统方法）
    pub async fn setup_image_tags(&self) -> DockerServiceResult<TagResult> {
        info!("开始设置镜像标签...");
        let result = self.image_loader.setup_image_tags().await?;

        if !result.is_all_successful() {
            warn!(
                "部分标签设置失败: 成功 {}, 失败 {}",
                result.success_count(),
                result.failure_count()
            );
        }

        Ok(result)
    }

    /// 启动所有服务
    pub async fn start_services(&mut self) -> DockerServiceResult<()> {
        info!("启动 Docker Compose 服务...");

        // 1. 检查端口冲突
        self.check_port_conflicts().await?;

        // 切换到工作目录
        let current_dir =
            std::env::current_dir().map_err(|e| DockerServiceError::FileSystem(e.to_string()))?;

        std::env::set_current_dir(&self.work_dir)
            .map_err(|e| DockerServiceError::FileSystem(e.to_string()))?;

        // 使用 DockerManager 启动服务
        let result = self.docker_manager.start_services().await;

        // 恢复原始目录
        std::env::set_current_dir(current_dir)
            .map_err(|e| DockerServiceError::FileSystem(e.to_string()))?;

        match result {
            Ok(_) => {
                info!("服务启动命令执行成功");

                // 等待服务就绪
                info!("等待服务启动完成...");
                let timeout = Duration::from_secs(timeout::HEALTH_CHECK_TIMEOUT);
                let check_interval = Duration::from_secs(timeout::HEALTH_CHECK_INTERVAL);

                match self
                    .health_checker
                    .wait_for_services_ready(timeout, check_interval)
                    .await
                {
                    Ok(report) => {
                        info!("所有服务已成功启动!");
                        self.print_service_status(&report).await;
                    }
                    Err(e) => {
                        warn!("等待服务启动超时或失败: {}", e);
                        // 即使超时也显示当前状态
                        if let Ok(report) = self.health_checker.check_health().await {
                            self.print_service_status(&report).await;
                        }
                    }
                }

                Ok(())
            }
            Err(e) => {
                error!("服务启动失败: {}", e);
                Err(DockerServiceError::ServiceManagement(e.to_string()))
            }
        }
    }

    /// 停止所有服务
    pub async fn stop_services(&self) -> DockerServiceResult<()> {
        info!("停止 Docker Compose 服务...");

        // 切换到工作目录
        let current_dir =
            std::env::current_dir().map_err(|e| DockerServiceError::FileSystem(e.to_string()))?;

        std::env::set_current_dir(&self.work_dir)
            .map_err(|e| DockerServiceError::FileSystem(e.to_string()))?;

        // 使用 DockerManager 停止服务
        let result = self.docker_manager.stop_services().await;

        // 恢复原始目录
        std::env::set_current_dir(current_dir)
            .map_err(|e| DockerServiceError::FileSystem(e.to_string()))?;

        match result {
            Ok(_) => {
                info!("服务已成功停止");
                Ok(())
            }
            Err(e) => {
                error!("服务停止失败: {}", e);
                Err(DockerServiceError::ServiceManagement(e.to_string()))
            }
        }
    }

    /// 重启所有服务
    pub async fn restart_services(&mut self) -> DockerServiceResult<()> {
        info!("重启 Docker Compose 服务...");

        // 先停止服务
        self.stop_services().await?;

        // 等待一下确保服务完全停止
        tokio::time::sleep(Duration::from_secs(timeout::RESTART_INTERVAL)).await;

        // 重新启动服务（包括镜像加载）
        self.deploy_services().await
    }

    /// 重启单个容器
    pub async fn restart_container(&self, container_name: &str) -> DockerServiceResult<()> {
        info!("重启容器: {}", container_name);

        // 切换到工作目录
        let current_dir =
            std::env::current_dir().map_err(|e| DockerServiceError::FileSystem(e.to_string()))?;

        std::env::set_current_dir(&self.work_dir)
            .map_err(|e| DockerServiceError::FileSystem(e.to_string()))?;

        // 重启指定容器
        let result = self.docker_manager.restart_service(container_name).await;

        // 恢复原始目录
        std::env::set_current_dir(current_dir)
            .map_err(|e| DockerServiceError::FileSystem(e.to_string()))?;

        match result {
            Ok(_) => {
                info!("容器 {} 重启成功", container_name);
                Ok(())
            }
            Err(e) => {
                error!("容器 {} 重启失败: {}", container_name, e);
                Err(DockerServiceError::ServiceManagement(e.to_string()))
            }
        }
    }

    /// 获取服务状态
    pub async fn get_service_status(&self) -> DockerServiceResult<HealthReport> {
        self.health_checker.check_health().await
    }

    /// 执行健康检查
    pub async fn health_check(&self) -> DockerServiceResult<HealthReport> {
        self.health_checker.check_health().await
    }

    /// 获取服务状态摘要
    pub async fn get_status_summary(&self) -> DockerServiceResult<String> {
        self.health_checker.get_status_summary().await
    }

    /// 打印服务状态信息
    async fn print_service_status(&self, report: &HealthReport) {
        info!("=== 服务状态概览 ===");
        info!("整体状态: {}", report.overall_status.display_name());
        info!(
            "运行中容器: {}/{}",
            report.running_count, report.total_count
        );

        if !report.containers.is_empty() {
            info!("容器详情:");
            for container in &report.containers {
                info!(
                    "  • {} - {} ({})",
                    container.name,
                    container.status.display_name(),
                    container.image
                );
            }
        }

        if !report.errors.is_empty() {
            warn!("错误信息:");
            for error in &report.errors {
                warn!("  • {}", error);
            }
        }

        // 显示访问信息
        if report.overall_status.is_healthy() {
            info!("=== 服务访问信息 ===");
            info!("• 前端页面: http://localhost:80");
            info!("• 后端API: http://localhost:8080");
            info!("• 服务管理完成，可以开始使用!");
        }
    }

    /// 检查特定容器状态
    pub async fn check_container_status(
        &self,
        container_name: &str,
    ) -> DockerServiceResult<crate::docker_service::health_check::ContainerInfo> {
        self.health_checker
            .check_container_status(container_name)
            .await
    }

    /// 检查端口冲突
    async fn check_port_conflicts(&mut self) -> DockerServiceResult<()> {
        let compose_file = self.work_dir.join("docker-compose.yml");

        if !compose_file.exists() {
            warn!("docker-compose.yml 文件不存在，跳过端口冲突检查");
            return Ok(());
        }

        info!("🔍 开始检查端口冲突...");

        match self
            .port_manager
            .smart_check_compose_port_conflicts(&compose_file)
            .await
        {
            Ok(report) => {
                if report.has_conflicts {
                    error!("❌ 发现端口冲突，无法启动服务");
                    self.port_manager.print_smart_conflict_report(&report);
                    return Err(DockerServiceError::PortManagement(format!(
                        "发现 {} 个端口冲突，请解决后重试",
                        report.conflicted_ports.len()
                    )));
                } else {
                    info!("✅ 端口检查通过，没有发现冲突");
                    if report.total_checked > 0 {
                        info!("总计检查了 {} 个端口映射", report.total_checked);
                    }
                }
            }
            Err(e) => {
                warn!("端口冲突检查失败: {}，将继续启动服务", e);
                // 端口检查失败不应该阻止服务启动，只是警告
            }
        }

        Ok(())
    }

    /// 手动检查端口冲突（供外部调用）
    pub async fn check_port_conflicts_report(&mut self) -> DockerServiceResult<PortConflictReport> {
        let compose_file = self.work_dir.join("docker-compose.yml");
        self.port_manager
            .smart_check_compose_port_conflicts(&compose_file)
            .await
    }
}
