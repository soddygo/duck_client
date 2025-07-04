use tauri::command;
use std::path::PathBuf;
use client_core::config::AppConfig;
use super::types::VersionInfo;

/// 获取版本信息
#[command]
pub async fn get_version_info() -> Result<VersionInfo, String> {
    // 客户端版本从编译时获取
    let client_version = format!("v{}", env!("CARGO_PKG_VERSION"));
    
    // 服务版本从配置文件获取
    let config_path = PathBuf::from("config.toml");
    let service_version = if config_path.exists() {
        match AppConfig::load_from_file(&config_path) {
            Ok(config) => config.versions.docker_service,
            Err(_) => "未知".to_string(),
        }
    } else {
        "未配置".to_string()
    };
    
    Ok(VersionInfo {
        client_version,
        service_version,
    })
} 