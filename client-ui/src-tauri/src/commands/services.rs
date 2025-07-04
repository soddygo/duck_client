use tauri::{command, AppHandle, Emitter};
use client_core::{
    config::AppConfig,
    container::DockerManager,
};
use super::types::{ServiceInfo, ServiceStatusInfo};
use tokio_stream::StreamExt;
use duck_cli::monitor_services as duck_monitor_services;

/// 获取服务状态
#[command]
pub async fn get_services_status() -> Result<Vec<ServiceInfo>, String> {
    use client_core::constants::config::get_config_file_path;
    
    // 加载配置
    let config_path = get_config_file_path();
    let config = AppConfig::load_from_file(&config_path)
        .map_err(|_| "配置文件未找到，请先运行初始化".to_string())?;
    
    // 创建Docker管理器
    let docker_manager = DockerManager::new(std::path::Path::new(&config.docker.compose_file))
        .map_err(|e| format!("Docker管理器初始化失败: {}", e))?;
    
    // 获取服务状态
    match docker_manager.get_services_status().await {
        Ok(services) => {
            let service_infos: Vec<ServiceInfo> = services.into_iter().map(|service| {
                ServiceInfo {
                    name: service.name,
                    status: match service.status {
                        client_core::container::ServiceStatus::Running => "运行中".to_string(),
                        client_core::container::ServiceStatus::Stopped => "已停止".to_string(),
                        client_core::container::ServiceStatus::Unknown => "未知".to_string(),
                    },
                    uptime: None, // Docker服务信息中没有直接的uptime，可以扩展
                    ports: service.ports,
                }
            }).collect();
            
            Ok(service_infos)
        },
        Err(_) => {
            // 如果获取失败，返回空列表而不是错误
            Ok(vec![])
        }
    }
}

/// 启动服务状态监控
#[command]
pub async fn start_services_monitoring(app_handle: AppHandle) -> Result<(), String> {
    let app_handle_clone = app_handle.clone();
    
    tokio::spawn(async move {
        let mut services_stream = duck_monitor_services().await;
        
        while let Some(service_status) = services_stream.next().await {
            let service_info = ServiceStatusInfo {
                name: service_status.name,
                status: service_status.status,
                health: service_status.health,
                uptime_seconds: service_status.uptime,
                cpu_usage: service_status.cpu_usage,
                memory_usage_mb: service_status.memory_usage / (1024 * 1024),
                ports: service_status.ports,
            };
            
            let _ = app_handle_clone.emit("service-status-update", service_info);
        }
    });
    
    Ok(())
}

/// 启动服务
#[command]
pub async fn start_services() -> Result<String, String> {
    use client_core::constants::config::get_config_file_path;
    
    let config_path = get_config_file_path();
    let config = AppConfig::load_from_file(&config_path)
        .map_err(|_| "配置文件未找到，请先运行初始化".to_string())?;
    
    let docker_manager = DockerManager::new(std::path::Path::new(&config.docker.compose_file))
        .map_err(|e| format!("Docker管理器初始化失败: {}", e))?;
    
    docker_manager.start_services()
        .await
        .map_err(|e| format!("启动服务失败: {}", e))?;
    
    Ok("服务启动成功".to_string())
}

/// 停止服务
#[command]
pub async fn stop_services() -> Result<String, String> {
    use client_core::constants::config::get_config_file_path;
    
    let config_path = get_config_file_path();
    let config = AppConfig::load_from_file(&config_path)
        .map_err(|_| "配置文件未找到，请先运行初始化".to_string())?;
    
    let docker_manager = DockerManager::new(std::path::Path::new(&config.docker.compose_file))
        .map_err(|e| format!("Docker管理器初始化失败: {}", e))?;
    
    docker_manager.stop_services()
        .await
        .map_err(|e| format!("停止服务失败: {}", e))?;
    
    Ok("服务停止成功".to_string())
}

/// 重启服务
#[command]
pub async fn restart_services() -> Result<String, String> {
    use client_core::constants::config::get_config_file_path;
    
    let config_path = get_config_file_path();
    let config = AppConfig::load_from_file(&config_path)
        .map_err(|_| "配置文件未找到，请先运行初始化".to_string())?;
    
    let docker_manager = DockerManager::new(std::path::Path::new(&config.docker.compose_file))
        .map_err(|e| format!("Docker管理器初始化失败: {}", e))?;
    
    docker_manager.restart_services()
        .await
        .map_err(|e| format!("重启服务失败: {}", e))?;
    
    Ok("服务重启成功".to_string())
} 