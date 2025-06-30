use client_core::error::Result;
use client_core::config::AppConfig;
use client_core::container::DockerManager;
use std::path::PathBuf;

// 子模块声明
mod manager;
mod architecture;
mod image_loader;
mod environment;
mod service_manager;
mod config;
mod health_check;
mod port_manager;
mod error;

// 公共接口导出
pub use manager::DockerServiceManager;
pub use architecture::{Architecture, detect_architecture};
pub use error::{DockerServiceError, DockerServiceResult};
pub use health_check::{ServiceStatus, HealthReport, ContainerStatus};
pub use port_manager::{PortManager, PortMapping, PortConflictReport, PortConflict};

/// Docker 服务管理的主要入口点
pub struct DockerService;

impl DockerService {
    /// 创建 Docker 服务管理器实例
    pub fn new(config: AppConfig, docker_manager: DockerManager) -> Result<DockerServiceManager> {
        let work_dir = docker_manager.get_working_directory()
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