use tauri::{command, AppHandle, Manager};
use std::path::PathBuf;
use client_core::config::AppConfig;
use client_core::constants::version::version_info::DEFAULT_DOCKER_SERVICE_VERSION;
use super::types::{VersionInfo, AppGlobalState};

/// 获取版本信息
#[command]
pub async fn get_version_info(app_handle: AppHandle) -> Result<VersionInfo, String> {
    // 客户端版本从编译时获取
    let client_version = format!("v{}", env!("CARGO_PKG_VERSION"));
    
    // 服务版本从配置文件获取，如果不存在则使用默认值
    let state = app_handle.state::<AppGlobalState>();
    let working_dir = state.working_directory.read().await;
    
    let service_version = if let Some(base_dir) = working_dir.as_ref() {
        let config_path = base_dir.join("data").join("config.toml");
        if config_path.exists() {
            match AppConfig::load_from_file(&config_path) {
                Ok(config) => config.versions.docker_service,
                Err(_) => format!("v{}", DEFAULT_DOCKER_SERVICE_VERSION),
            }
        } else {
            format!("v{}", DEFAULT_DOCKER_SERVICE_VERSION)
        }
    } else {
        format!("v{}", DEFAULT_DOCKER_SERVICE_VERSION)
    };
    
    Ok(VersionInfo {
        client_version,
        service_version,
    })
} 