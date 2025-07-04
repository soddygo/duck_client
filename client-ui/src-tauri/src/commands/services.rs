use tauri::{command, AppHandle, Emitter, Manager};
use client_core::{
    config::AppConfig,
    container::DockerManager,
};
use super::types::{ServiceInfo, ServiceStatusInfo, AppGlobalState};
use tokio_stream::StreamExt;
use duck_cli::{monitor_services as duck_monitor_services, DockerService};
use tracing::{info, error, warn};
use std::time::Instant;
use serde_json;

/// 获取服务状态
#[command]
pub async fn get_services_status(app_handle: AppHandle) -> Result<Vec<ServiceInfo>, String> {
    // 1. 获取用户设置的工作目录
    let state = app_handle.state::<AppGlobalState>();
    let working_dir = state.working_directory.read().await;
    
    let base_dir = working_dir.as_ref().ok_or("请先选择工作目录")?;
    
    // 2. 临时切换到用户选择的工作目录
    let original_dir = std::env::current_dir().map_err(|e| format!("获取当前目录失败: {}", e))?;
    std::env::set_current_dir(base_dir).map_err(|e| format!("切换到工作目录失败: {}", e))?;
    
    // 执行获取状态（确保在结束时恢复目录）
    let result = async {
        // 3. 加载配置
        let config = AppConfig::find_and_load_config()
            .map_err(|_| "配置文件未找到，请先运行初始化".to_string())?;
        
        // 4. 检查docker-compose.yml是否存在
        let docker_compose_path = base_dir.join("docker").join("docker-compose.yml");
        if !docker_compose_path.exists() {
            return Ok(vec![]);
        }
        
        // 5. 创建Docker管理器
        let docker_manager = DockerManager::new(&docker_compose_path)
            .map_err(|e| format!("Docker管理器初始化失败: {}", e))?;
        
        // 6. 创建DockerService实例（使用和CLI相同的逻辑）
        let docker_service = DockerService::new(config, docker_manager)
            .map_err(|e| format!("DockerService初始化失败: {}", e))?;
        
        // 7. 获取详细的服务状态报告
        match docker_service.get_service_status().await {
            Ok(report) => {
                let service_infos: Vec<ServiceInfo> = report.containers.into_iter().map(|container| {
                    let status = match container.status {
                        duck_cli::ContainerStatus::Running => "运行中".to_string(),
                        duck_cli::ContainerStatus::Stopped => "已停止".to_string(),
                        duck_cli::ContainerStatus::Starting => "启动中".to_string(),
                        duck_cli::ContainerStatus::Unhealthy => "不健康".to_string(),
                        duck_cli::ContainerStatus::Completed => "已完成".to_string(),
                        duck_cli::ContainerStatus::Unknown => "未知".to_string(),
                    };
                    
                    ServiceInfo {
                        name: container.name,
                        status,
                        uptime: container.uptime,
                        ports: container.ports,
                    }
                }).collect();
                
                Ok(service_infos)
            },
            Err(e) => {
                warn!("获取服务状态失败: {}", e);
                Ok(vec![])
            }
        }
    }.await;
    
    // 恢复原始工作目录
    let _ = std::env::set_current_dir(original_dir);
    
    result
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
pub async fn start_services(app_handle: AppHandle) -> Result<String, String> {
    let start_time = Instant::now();
    
    // 1. 获取用户设置的工作目录
    let state = app_handle.state::<AppGlobalState>();
    let working_dir = state.working_directory.read().await;
    
    let base_dir = working_dir.as_ref().ok_or("请先选择工作目录")?;
    
    // 2. 临时切换到用户选择的工作目录
    let original_dir = std::env::current_dir().map_err(|e| format!("获取当前目录失败: {}", e))?;
    std::env::set_current_dir(base_dir).map_err(|e| format!("切换到工作目录失败: {}", e))?;
    
    // 执行启动服务（确保在结束时恢复目录）
    let result = async {
        // 3. 获取全局数据库管理器
        let db_manager = state.get_or_init_db_manager(base_dir).await?;
        
        // 4. 记录用户操作开始
        let action_id = db_manager.record_user_action(
            "START_SERVICES",
            "启动Docker服务",
            None
        ).await.map_err(|e| format!("记录用户操作失败: {}", e))?;
        
        info!("▶️ 启动 Docker 服务...");
        
        // 5. 加载配置
        let config = AppConfig::find_and_load_config()
            .map_err(|_| "配置文件未找到，请先运行初始化".to_string())?;
        
        // 6. 检查docker-compose.yml是否存在
        let docker_compose_path = base_dir.join("docker").join("docker-compose.yml");
        if !docker_compose_path.exists() {
            error!("❌ Docker Compose文件不存在: {}", docker_compose_path.display());
            error!("   工作目录: {}", base_dir.display());
            error!("   请确保已完成服务部署或选择正确的工作目录");
            
            // 发送特殊事件，引导用户进行初始化
            let _ = app_handle.emit("require-initialization", serde_json::json!({
                "working_directory": base_dir.to_string_lossy(),
                "reason": "Docker服务文件不存在，需要重新部署"
            }));
            
            return Err("Docker服务文件不存在。请先完成服务部署，或检查工作目录是否正确。如果是空目录，请重新进行初始化。".to_string());
        }
        
        // 7. 创建Docker管理器
        let docker_manager = DockerManager::new(&docker_compose_path)
            .map_err(|e| format!("Docker管理器初始化失败: {}", e))?;
        
        // 8. 创建DockerService实例（使用和CLI相同的逻辑）
        let mut docker_service = DockerService::new(config, docker_manager)
            .map_err(|e| format!("DockerService初始化失败: {}", e))?;
        
        // 9. 启动服务
        match docker_service.start_services().await {
            Ok(_) => {
                info!("✅ Docker 服务启动成功!");
                
                // 记录用户操作完成
                let duration = start_time.elapsed().as_secs() as i32;
                db_manager.complete_user_action(
                    action_id,
                    "SUCCESS",
                    Some("Docker服务启动成功".to_string()),
                    Some(duration)
                ).await.map_err(|e| format!("完成用户操作记录失败: {}", e))?;
                
                Ok("服务启动成功".to_string())
            }
            Err(e) => {
                error!("❌ Docker 服务启动失败: {}", e);
                
                // 记录用户操作失败
                let duration = start_time.elapsed().as_secs() as i32;
                db_manager.complete_user_action(
                    action_id,
                    "FAILED",
                    Some(format!("Docker服务启动失败: {}", e)),
                    Some(duration)
                ).await.map_err(|e| format!("完成用户操作记录失败: {}", e))?;
                
                Err(format!("启动服务失败: {}", e))
            }
        }
    }.await;
    
    // 恢复原始工作目录
    let _ = std::env::set_current_dir(original_dir);
    
    result
}

/// 停止服务
#[command]
pub async fn stop_services(app_handle: AppHandle) -> Result<String, String> {
    let start_time = Instant::now();
    
    // 1. 获取用户设置的工作目录
    let state = app_handle.state::<AppGlobalState>();
    let working_dir = state.working_directory.read().await;
    
    let base_dir = working_dir.as_ref().ok_or("请先选择工作目录")?;
    
    // 2. 临时切换到用户选择的工作目录
    let original_dir = std::env::current_dir().map_err(|e| format!("获取当前目录失败: {}", e))?;
    std::env::set_current_dir(base_dir).map_err(|e| format!("切换到工作目录失败: {}", e))?;
    
    // 执行停止服务（确保在结束时恢复目录）
    let result = async {
        // 3. 获取全局数据库管理器
        let db_manager = state.get_or_init_db_manager(base_dir).await?;
        
        // 4. 记录用户操作开始
        let action_id = db_manager.record_user_action(
            "STOP_SERVICES",
            "停止Docker服务",
            None
        ).await.map_err(|e| format!("记录用户操作失败: {}", e))?;
        
        info!("⏹️ 停止 Docker 服务...");
        
        // 5. 加载配置
        let config = AppConfig::find_and_load_config()
            .map_err(|_| "配置文件未找到，请先运行初始化".to_string())?;
        
        // 6. 检查docker-compose.yml是否存在
        let docker_compose_path = base_dir.join("docker").join("docker-compose.yml");
        if !docker_compose_path.exists() {
            error!("❌ Docker Compose文件不存在: {}", docker_compose_path.display());
            error!("   工作目录: {}", base_dir.display());
            error!("   服务可能未部署，或工作目录不正确");
            
            // 发送特殊事件，引导用户进行初始化
            let _ = app_handle.emit("require-initialization", serde_json::json!({
                "working_directory": base_dir.to_string_lossy(),
                "reason": "Docker服务文件不存在，无法停止服务"
            }));
            
            return Err("Docker服务文件不存在，服务可能未部署。如果是空目录，请重新进行初始化。".to_string());
        }
        
        // 7. 创建Docker管理器
        let docker_manager = DockerManager::new(&docker_compose_path)
            .map_err(|e| format!("Docker管理器初始化失败: {}", e))?;
        
        // 8. 创建DockerService实例（使用和CLI相同的逻辑）
        let docker_service = DockerService::new(config, docker_manager)
            .map_err(|e| format!("DockerService初始化失败: {}", e))?;
        
        // 9. 停止服务
        match docker_service.stop_services().await {
            Ok(_) => {
                info!("✅ Docker 服务已停止");
                
                // 记录用户操作完成
                let duration = start_time.elapsed().as_secs() as i32;
                db_manager.complete_user_action(
                    action_id,
                    "SUCCESS",
                    Some("Docker服务停止成功".to_string()),
                    Some(duration)
                ).await.map_err(|e| format!("完成用户操作记录失败: {}", e))?;
                
                Ok("服务停止成功".to_string())
            }
            Err(e) => {
                error!("❌ Docker 服务停止失败: {}", e);
                
                // 记录用户操作失败
                let duration = start_time.elapsed().as_secs() as i32;
                db_manager.complete_user_action(
                    action_id,
                    "FAILED",
                    Some(format!("Docker服务停止失败: {}", e)),
                    Some(duration)
                ).await.map_err(|e| format!("完成用户操作记录失败: {}", e))?;
                
                Err(format!("停止服务失败: {}", e))
            }
        }
    }.await;
    
    // 恢复原始工作目录
    let _ = std::env::set_current_dir(original_dir);
    
    result
}

/// 重启服务
#[command]
pub async fn restart_services(app_handle: AppHandle) -> Result<String, String> {
    let start_time = Instant::now();
    
    // 1. 获取用户设置的工作目录
    let state = app_handle.state::<AppGlobalState>();
    let working_dir = state.working_directory.read().await;
    
    let base_dir = working_dir.as_ref().ok_or("请先选择工作目录")?;
    
    // 2. 临时切换到用户选择的工作目录
    let original_dir = std::env::current_dir().map_err(|e| format!("获取当前目录失败: {}", e))?;
    std::env::set_current_dir(base_dir).map_err(|e| format!("切换到工作目录失败: {}", e))?;
    
    // 执行重启服务（确保在结束时恢复目录）
    let result = async {
        // 3. 获取全局数据库管理器
        let db_manager = state.get_or_init_db_manager(base_dir).await?;
        
        // 4. 记录用户操作开始
        let action_id = db_manager.record_user_action(
            "RESTART_SERVICES",
            "重启Docker服务",
            None
        ).await.map_err(|e| format!("记录用户操作失败: {}", e))?;
        
        info!("🔄 重启 Docker 服务...");
        
        // 5. 加载配置
        let config = AppConfig::find_and_load_config()
            .map_err(|_| "配置文件未找到，请先运行初始化".to_string())?;
        
        // 6. 检查docker-compose.yml是否存在
        let docker_compose_path = base_dir.join("docker").join("docker-compose.yml");
        if !docker_compose_path.exists() {
            error!("❌ Docker Compose文件不存在: {}", docker_compose_path.display());
            error!("   工作目录: {}", base_dir.display());
            error!("   请确保已完成服务部署或选择正确的工作目录");
            
            // 发送特殊事件，引导用户进行初始化
            let _ = app_handle.emit("require-initialization", serde_json::json!({
                "working_directory": base_dir.to_string_lossy(),
                "reason": "Docker服务文件不存在，无法重启服务"
            }));
            
            return Err("Docker服务文件不存在。请先完成服务部署，或检查工作目录是否正确。如果是空目录，请重新进行初始化。".to_string());
        }
        
        // 7. 创建Docker管理器
        let docker_manager = DockerManager::new(&docker_compose_path)
            .map_err(|e| format!("Docker管理器初始化失败: {}", e))?;
        
        // 8. 创建DockerService实例（使用和CLI相同的逻辑）
        let mut docker_service = DockerService::new(config, docker_manager)
            .map_err(|e| format!("DockerService初始化失败: {}", e))?;
        
        // 9. 重启服务
        match docker_service.restart_services().await {
            Ok(_) => {
                info!("✅ Docker 服务重启成功!");
                
                // 记录用户操作完成
                let duration = start_time.elapsed().as_secs() as i32;
                db_manager.complete_user_action(
                    action_id,
                    "SUCCESS",
                    Some("Docker服务重启成功".to_string()),
                    Some(duration)
                ).await.map_err(|e| format!("完成用户操作记录失败: {}", e))?;
                
                Ok("服务重启成功".to_string())
            }
            Err(e) => {
                error!("❌ Docker 服务重启失败: {}", e);
                
                // 记录用户操作失败
                let duration = start_time.elapsed().as_secs() as i32;
                db_manager.complete_user_action(
                    action_id,
                    "FAILED",
                    Some(format!("Docker服务重启失败: {}", e)),
                    Some(duration)
                ).await.map_err(|e| format!("完成用户操作记录失败: {}", e))?;
                
                Err(format!("重启服务失败: {}", e))
            }
        }
    }.await;
    
    // 恢复原始工作目录
    let _ = std::env::set_current_dir(original_dir);
    
    result
} 