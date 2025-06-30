use std::path::PathBuf;

/// Docker 服务状态
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Unknown,
}

/// Docker 服务信息
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub status: ServiceStatus,
    pub image: String,
    pub ports: Vec<String>,
}

/// 服务配置（从docker-compose.yml解析）
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub restart: Option<String>,
}

/// Docker 服务管理器
#[derive(Debug, Clone)]
pub struct DockerManager {
    pub(crate) compose_file: PathBuf,
} 