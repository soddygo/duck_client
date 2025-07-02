use serde::{Deserialize, Serialize};
use tauri::command;
use std::path::PathBuf;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub status: String,
    pub containers: i32,
    pub uptime: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    pub client_version: String,
    pub service_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub size: i64,
}

#[derive(serde::Serialize)]
struct DataDirectoryInfo {
    path: String,
    backup_path: String,
    cache_path: String,
    docker_path: String,
    exists: bool,
    backup_exists: bool,
    cache_exists: bool,
    docker_exists: bool,
    total_size_mb: f64,
}

// 获取数据目录信息（基于当前工作目录）
#[command]
async fn get_data_directory() -> Result<DataDirectoryInfo, String> {
    let current_dir = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    // 各个子目录路径
    let backup_path = current_dir.join("backups");
    let cache_path = current_dir.join("cacheDuckData");
    let docker_path = current_dir.join("docker");
    
    // 检查目录是否存在
    let exists = current_dir.exists();
    let backup_exists = backup_path.exists();
    let cache_exists = cache_path.exists();
    let docker_exists = docker_path.exists();
    
    // 计算总大小
    let mut total_size_mb = 0.0;
    if backup_exists {
        total_size_mb += calculate_directory_size(&backup_path).unwrap_or(0.0);
    }
    if cache_exists {
        total_size_mb += calculate_directory_size(&cache_path).unwrap_or(0.0);
    }
    if docker_exists {
        total_size_mb += calculate_directory_size(&docker_path).unwrap_or(0.0);
    }
    
    Ok(DataDirectoryInfo {
        path: current_dir.to_string_lossy().to_string(),
        backup_path: backup_path.to_string_lossy().to_string(),
        cache_path: cache_path.to_string_lossy().to_string(),
        docker_path: docker_path.to_string_lossy().to_string(),
        exists,
        backup_exists,
        cache_exists,
        docker_exists,
        total_size_mb,
    })
}

// 打开数据目录
#[command]
async fn open_data_directory() -> Result<(), String> {
    let current_dir = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    // 使用系统默认程序打开目录
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&current_dir)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&current_dir)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&current_dir)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }
    
    Ok(())
}

// 打开备份目录
#[command]
async fn open_backup_directory() -> Result<(), String> {
    let current_dir = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    let backup_path = current_dir.join("backups");
    
    // 如果备份目录不存在，创建它
    if !backup_path.exists() {
        std::fs::create_dir_all(&backup_path)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&backup_path)
            .spawn()
            .map_err(|e| format!("Failed to open backup directory: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&backup_path)
            .spawn()
            .map_err(|e| format!("Failed to open backup directory: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&backup_path)
            .spawn()
            .map_err(|e| format!("Failed to open backup directory: {}", e))?;
    }
    
    Ok(())
}

// 打开缓存目录
#[command]
async fn open_cache_directory() -> Result<(), String> {
    let current_dir = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    let cache_path = current_dir.join("cacheDuckData");
    
    if !cache_path.exists() {
        return Err("缓存目录不存在，请先运行 duck-cli upgrade 下载服务包".to_string());
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&cache_path)
            .spawn()
            .map_err(|e| format!("Failed to open cache directory: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&cache_path)
            .spawn()
            .map_err(|e| format!("Failed to open cache directory: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&cache_path)
            .spawn()
            .map_err(|e| format!("Failed to open cache directory: {}", e))?;
    }
    
    Ok(())
}

// 计算目录大小的辅助函数
fn calculate_directory_size(dir: &PathBuf) -> Result<f64, Box<dyn std::error::Error>> {
    let mut total_size = 0u64;
    
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                total_size += calculate_directory_size(&path)? as u64;
            } else {
                total_size += entry.metadata()?.len();
            }
        }
    }
    
    // 转换为MB
    Ok(total_size as f64 / 1024.0 / 1024.0)
}

// 获取服务状态
#[command]
async fn get_service_status() -> Result<ServiceStatus, String> {
    Ok(ServiceStatus {
        status: "running".to_string(),
        containers: 5,
        uptime: "2小时30分钟".to_string(),
    })
}

// 启动服务
#[command]
async fn start_service() -> Result<(), String> {
    Ok(())
}

// 停止服务
#[command]
async fn stop_service() -> Result<(), String> {
    Ok(())
}

// 重启服务
#[command]
async fn restart_service() -> Result<(), String> {
    Ok(())
}

// 检查更新
#[command]
async fn check_updates() -> Result<VersionInfo, String> {
    Ok(VersionInfo {
        client_version: "1.0.10".to_string(),
        service_version: "1.2.0".to_string(),
    })
}

// 执行升级
#[command]
async fn perform_upgrade(_full: bool, _force: bool) -> Result<(), String> {
    Ok(())
}

// 创建备份
#[command]
async fn create_backup() -> Result<BackupInfo, String> {
    Ok(BackupInfo {
        id: 3,
        name: "backup_20240120_154500".to_string(),
        created_at: "2024-01-20 15:45:00".to_string(),
        size: 162529280, // ~155MB
    })
}

// 列出备份
#[command]
async fn list_backups() -> Result<Vec<BackupInfo>, String> {
    Ok(vec![
        BackupInfo {
            id: 1,
            name: "backup_20240120_103000".to_string(),
            created_at: "2024-01-20 10:30:00".to_string(),
            size: 157286400, // ~150MB
        },
        BackupInfo {
            id: 2,
            name: "backup_20240119_020000".to_string(),
            created_at: "2024-01-19 02:00:00".to_string(),
            size: 145829888, // ~139MB
        },
    ])
}

// 从备份恢复
#[command]
async fn restore_backup(_backup_id: i64, _force: bool) -> Result<(), String> {
    Ok(())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_service_status,
            start_service,
            stop_service,
            restart_service,
            check_updates,
            perform_upgrade,
            create_backup,
            list_backups,
            restore_backup,
            get_data_directory,
            open_data_directory,
            open_backup_directory,
            open_cache_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
