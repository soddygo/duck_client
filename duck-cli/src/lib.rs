// 私有模块声明
mod app;
mod cli;
mod commands;
mod docker_utils;
mod init;
pub mod project_info;  // 公开项目信息模块
mod utils;
mod docker_service;

// 通过 pub use 精确控制对外暴露的接口
pub use app::CliApp;
pub use cli::{Cli, Commands};
pub use init::run_init;
pub use utils::setup_logging;
pub use docker_service::{DockerService, DockerServiceManager, ContainerStatus, get_system_architecture, get_architecture_suffix}; 