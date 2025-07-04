use tauri::{command, AppHandle, Emitter};
use std::path::PathBuf;
use client_core::{
    config::AppConfig,
    database::Database,
    api::ApiClient,
    container::DockerManager,
    backup::BackupManager,
    upgrade::UpgradeManager,
};
use super::types::{UpgradeInfo, UpgradeCompletedEvent};

/// 检查可用的升级版本
#[command]
pub async fn check_upgrade_available() -> Result<UpgradeInfo, String> {
    use client_core::constants::config::get_config_file_path;
    
    // 加载配置
    let config_path = get_config_file_path();
    let config = AppConfig::load_from_file(&config_path)
        .map_err(|e| format!("加载配置失败: {}", e))?;
    
    // 创建API客户端
    let api_client = ApiClient::new(None);
    
    // 检查版本
    let current_version = &config.versions.docker_service;
    let version_result = api_client.check_docker_version(current_version)
        .await
        .map_err(|e| format!("检查版本失败: {}", e))?;
    
    // 估算下载大小和时间（从服务清单获取）
    let (download_size, estimated_time) = if version_result.has_update {
        match api_client.get_docker_service_manifest().await {
            Ok(manifest) => {
                let size_mb = manifest.packages.full.size as f64 / 1024.0 / 1024.0;
                let size_str = format!("{:.1} MB", size_mb);
                let time_estimate = if size_mb > 100.0 { "5-10分钟" } else { "2-5分钟" };
                (Some(size_str), Some(time_estimate.to_string()))
            },
            Err(_) => (Some("约100MB".to_string()), Some("5-10分钟".to_string()))
        }
    } else {
        (None, None)
    };
    
    Ok(UpgradeInfo {
        current_version: version_result.current_version,
        latest_version: version_result.latest_version,
        has_update: version_result.has_update,
        release_notes: version_result.release_notes,
        download_size,
        estimated_time,
    })
}

/// 开始升级下载
#[command]
pub async fn start_upgrade_download() -> Result<String, String> {
    use client_core::constants::config::{get_config_file_path, get_database_path};
    
    // 加载配置和创建管理器
    let config_path = get_config_file_path();
    let config = AppConfig::load_from_file(&config_path)
        .map_err(|e| format!("加载配置失败: {}", e))?;
    
    let database = Database::connect(&get_database_path())
        .await
        .map_err(|e| format!("连接数据库失败: {}", e))?;
    
    let api_client = ApiClient::new(None);
    
    let docker_manager = DockerManager::new(std::path::Path::new(&config.docker.compose_file))
        .map_err(|e| format!("初始化Docker管理器失败: {}", e))?;
    
    let backup_manager = BackupManager::new(
        PathBuf::from(&config.backup.storage_dir),
        database.clone(),
        docker_manager.clone()
    ).map_err(|e| format!("初始化备份管理器失败: {}", e))?;
    
    let mut upgrade_manager = UpgradeManager::new(
        config.clone(),
        config_path,
        docker_manager,
        backup_manager,
        api_client,
        database,
    );
    
    // 执行升级
    let options = client_core::upgrade::UpgradeOptions {
        skip_backup: false,
        force: false,
        use_incremental: false,
        backup_dir: None,
        download_only: false,
    };
    
    upgrade_manager.upgrade_service(options, None)
        .await
        .map_err(|e| format!("升级失败: {}", e))?;
    
    Ok("升级完成".to_string())
}

/// 模拟升级进度更新（用于测试）
#[command]
pub async fn simulate_upgrade_progress(app_handle: AppHandle) -> Result<(), String> {
    let total_steps = 10;
    
    for i in 1..=total_steps {
        let progress = (i as f64 / total_steps as f64) * 100.0;
        let stage = match i {
            1..=2 => "准备下载",
            3..=7 => "下载中",
            8..=9 => "安装中", 
            10 => "完成",
            _ => "未知"
        };
        
        // 发送进度事件到前端
        app_handle.emit("upgrade-progress", serde_json::json!({
            "progress": progress,
            "stage": stage,
            "message": format!("正在{}... ({}/{})", stage, i, total_steps)
        })).map_err(|e| format!("发送事件失败: {}", e))?;
        
        // 模拟延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    // 发送完成事件
    app_handle.emit("upgrade-completed", UpgradeCompletedEvent {
        success: true,
        version: "1.2.3".to_string(),
        message: "升级成功完成".to_string(),
    }).map_err(|e| format!("发送完成事件失败: {}", e))?;
    
    Ok(())
} 