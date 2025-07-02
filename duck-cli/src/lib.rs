// 私有模块声明
mod app;
mod cli;
mod commands;
mod docker_service;
mod docker_utils;
mod init;
pub mod project_info; // 公开项目信息模块
mod utils;

// 通过 pub use 精确控制对外暴露的接口
pub use app::CliApp;
pub use cli::{Cli, Commands};
pub use commands::{run_status_details, show_client_version}; // 导出status相关函数
pub use docker_service::{
    ContainerStatus, DockerService, DockerServiceManager, get_architecture_suffix,
    get_system_architecture,
};
pub use init::run_init;
pub use utils::setup_logging;
