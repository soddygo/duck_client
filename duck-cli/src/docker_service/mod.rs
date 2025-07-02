use client_core::config::AppConfig;
use client_core::container::DockerManager;
use client_core::error::Result;

// 子模块声明
mod architecture;
mod config;
mod directory_permissions;
mod environment;
mod error;
mod health_check;
mod image_loader;
mod manager;
mod port_manager;
mod script_permissions;
mod service_manager;

// 公共接口导出
pub use architecture::{Architecture, detect_architecture};
#[allow(unused_imports)]
pub use config::DockerServiceConfig;
#[allow(unused_imports)]
pub use environment::EnvironmentChecker;
#[allow(unused_imports)]
pub use error::{DockerServiceError, DockerServiceResult};
#[allow(unused_imports)]
pub use health_check::{ContainerStatus, HealthReport, ServiceStatus};
#[allow(unused_imports)]
pub use image_loader::{ImageInfo, ImageLoader, ImageType, LoadResult, TagResult};
pub use manager::DockerServiceManager;
#[allow(unused_imports)]
pub use port_manager::{PortConflict, PortConflictReport, PortManager, PortMapping};
#[allow(unused_imports)]
pub use service_manager::ServiceManager;

/// Docker 服务管理的主要入口点
pub struct DockerService;

impl DockerService {
    /// 创建 Docker 服务管理器实例
    #[allow(clippy::new_ret_no_self)]
    pub fn new(config: AppConfig, docker_manager: DockerManager) -> Result<DockerServiceManager> {
        let work_dir = docker_manager
            .get_working_directory()
            .ok_or_else(|| client_core::DuckError::Custom("无法确定 Docker 工作目录".to_string()))?
            .to_path_buf();

        Ok(DockerServiceManager::new(config, docker_manager, work_dir))
    }
}

/// 便捷函数：检测系统架构
pub fn get_system_architecture() -> Architecture {
    detect_architecture()
}

/// 便捷函数：获取架构对应的镜像后缀
pub fn get_architecture_suffix(arch: Architecture) -> &'static str {
    match arch {
        Architecture::Amd64 => "amd64",
        Architecture::Arm64 => "arm64",
    }
}
