use serde::{Deserialize, Serialize};
use tauri::command;
use std::path::PathBuf;
use std::env;
use std::fs;

// 导入新的命令模块
mod commands;
use commands::*;

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
    work_dir: String,
    backup_path: String,
    cache_path: String,
    docker_path: String,
    config_exists: bool,
    backup_exists: bool,
    cache_exists: bool,
    docker_exists: bool,
    total_size_mb: f64,
    is_initialized: bool,
}

// duck-cli 配置文件结构（简化版本）
#[derive(Debug, Serialize, Deserialize)]
struct DuckConfig {
    versions: Versions,
    backup: BackupConfig,
    cache: CacheConfig,
    docker: DockerConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct Versions {
    docker_service: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupConfig {
    storage_dir: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheConfig {
    cache_dir: String,
    download_dir: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DockerConfig {
    compose_file: String,
}

// 获取存储的工作目录路径
fn get_stored_work_dir() -> Option<PathBuf> {
    // 尝试从应用数据目录读取存储的工作目录
    if let Some(app_data_dir) = dirs::data_dir() {
        let settings_file = app_data_dir.join("duck_client").join("work_dir.txt");
        if let Ok(content) = fs::read_to_string(settings_file) {
            let path = PathBuf::from(content.trim());
            if path.exists() {
                return Some(path);
            }
        }
    }
    None
}

// 保存工作目录路径
fn save_work_dir(work_dir: &PathBuf) -> Result<(), String> {
    if let Some(app_data_dir) = dirs::data_dir() {
        let duck_client_dir = app_data_dir.join("duck_client");
        fs::create_dir_all(&duck_client_dir)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
        
        let settings_file = duck_client_dir.join("work_dir.txt");
        fs::write(settings_file, work_dir.to_string_lossy().as_bytes())
            .map_err(|e| format!("Failed to save work directory: {}", e))?;
    }
    Ok(())
}

// 读取 duck-cli 配置文件
fn read_duck_config(work_dir: &PathBuf) -> Result<DuckConfig, String> {
    let config_file = work_dir.join("config.toml");
    if !config_file.exists() {
        return Err("配置文件 config.toml 不存在".to_string());
    }
    
    let content = fs::read_to_string(config_file)
        .map_err(|e| format!("读取配置文件失败: {}", e))?;
    
    let config: DuckConfig = toml::from_str(&content)
        .map_err(|e| format!("解析配置文件失败: {}", e))?;
    
    Ok(config)
}

// 获取数据目录信息
#[command]
async fn get_data_directory() -> Result<DataDirectoryInfo, String> {
    let work_dir = get_stored_work_dir().ok_or("未设置工作目录")?;
    
    // 检查基础文件存在性
    let config_exists = work_dir.join("config.toml").exists();
    let is_initialized = config_exists && work_dir.join("history.db").exists();
    
    let (backup_path, cache_path, docker_path) = if config_exists {
        // 尝试读取配置文件获取准确路径
        match read_duck_config(&work_dir) {
            Ok(config) => {
                let backup_path = if config.backup.storage_dir.starts_with("./") {
                    work_dir.join(&config.backup.storage_dir[2..])
                } else {
                    PathBuf::from(&config.backup.storage_dir)
                };
                
                let cache_path = if config.cache.cache_dir.starts_with("./") {
                    work_dir.join(&config.cache.cache_dir[2..])
                } else {
                    PathBuf::from(&config.cache.cache_dir)
                };
                
                let docker_path = if config.docker.compose_file.starts_with("./") {
                    work_dir.join(&config.docker.compose_file[2..]).parent().unwrap().to_path_buf()
                } else {
                    work_dir.join("docker")
                };
                
                (backup_path, cache_path, docker_path)
            }
            Err(_) => {
                // 配置文件解析失败，使用默认路径
                (
                    work_dir.join("backups"),
                    work_dir.join("cacheDuckData"),
                    work_dir.join("docker")
                )
            }
        }
    } else {
        // 配置文件不存在，使用默认路径
        (
            work_dir.join("backups"),
            work_dir.join("cacheDuckData"),
            work_dir.join("docker")
        )
    };
    
    // 检查目录是否存在
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
        work_dir: work_dir.to_string_lossy().to_string(),
        backup_path: backup_path.to_string_lossy().to_string(),
        cache_path: cache_path.to_string_lossy().to_string(),
        docker_path: docker_path.to_string_lossy().to_string(),
        config_exists,
        backup_exists,
        cache_exists,
        docker_exists,
        total_size_mb,
        is_initialized,
    })
}

// 设置工作目录
#[command]
async fn set_work_directory(_app: tauri::AppHandle, path: String) -> Result<(), String> {
    let work_dir = PathBuf::from(path);
    
    if !work_dir.exists() {
        return Err("选择的目录不存在".to_string());
    }
    
    if !work_dir.is_dir() {
        return Err("选择的路径不是目录".to_string());
    }
    
    // 保存工作目录设置
    save_work_dir(&work_dir)?;
    
    Ok(())
}

// 选择工作目录
#[command]
async fn select_work_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use std::sync::{Arc, Mutex};
    use std::sync::mpsc;
    
    let (tx, rx) = mpsc::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));
    
    tauri_plugin_dialog::DialogExt::dialog(&app)
        .file()
        .pick_folder(move |folder_path| {
            if let Ok(sender) = tx.lock() {
                if let Some(sender) = sender.as_ref() {
                    let _ = sender.send(folder_path);
                }
            }
        });
    
    match rx.recv() {
        Ok(Some(folder_path)) => {
            let path = folder_path.to_string();
            // 设置工作目录
            set_work_directory(app, path.clone()).await?;
            Ok(Some(path))
        }
        _ => Ok(None),
    }
}

// 打开工作目录
#[command]
async fn open_data_directory() -> Result<(), String> {
    let work_dir = get_stored_work_dir().ok_or("未设置工作目录")?;
    open_directory(&work_dir)
}

// 打开备份目录
#[command]
async fn open_backup_directory() -> Result<(), String> {
    let data_info = get_data_directory().await?;
    let backup_path = PathBuf::from(data_info.backup_path);
    
    // 如果备份目录不存在，创建它
    if !backup_path.exists() {
        fs::create_dir_all(&backup_path)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;
    }
    
    open_directory(&backup_path)
}

// 打开缓存目录
#[command]
async fn open_cache_directory() -> Result<(), String> {
    let data_info = get_data_directory().await?;
    let cache_path = PathBuf::from(data_info.cache_path);
    
    if !cache_path.exists() {
        return Err("缓存目录不存在，请先运行 duck-cli upgrade 下载服务包".to_string());
    }
    
    open_directory(&cache_path)
}

// 通用的打开目录函数
fn open_directory(path: &PathBuf) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }
    
    Ok(())
}

// 清理缓存
#[command]
async fn clear_cache() -> Result<String, String> {
    let work_dir = get_stored_work_dir().ok_or("未设置工作目录")?;
    
    // 执行 duck-cli 清理缓存命令（我们稍后会添加这个命令）
    let output = std::process::Command::new("duck-cli")
        .arg("cache")
        .arg("clear")
        .current_dir(&work_dir)
        .output()
        .map_err(|e| format!("执行清理命令失败: {}", e))?;
    
    if output.status.success() {
        Ok("缓存清理完成".to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("清理缓存失败: {}", error))
    }
}

// 初始化客户端
#[command]
async fn init_client() -> Result<String, String> {
    let work_dir = get_stored_work_dir().ok_or("未设置工作目录")?;
    
    let output = std::process::Command::new("duck-cli")
        .arg("init")
        .arg("--force")
        .current_dir(&work_dir)
        .output()
        .map_err(|e| format!("执行初始化命令失败: {}", e))?;
    
    if output.status.success() {
        Ok("客户端初始化完成".to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("初始化失败: {}", error))
    }
}

// 自动部署服务
#[command]
async fn auto_deploy_service(port: Option<u16>) -> Result<String, String> {
    let work_dir = get_stored_work_dir().ok_or("未设置工作目录")?;
    
    let mut cmd = std::process::Command::new("duck-cli");
    cmd.arg("auto-upgrade-deploy")
        .arg("run")
        .current_dir(&work_dir);
    
    // 如果指定了端口，添加端口参数
    if let Some(p) = port {
        cmd.arg("--port").arg(p.to_string());
    }
    
    let output = cmd.output()
        .map_err(|e| format!("执行自动部署命令失败: {}", e))?;
    
    if output.status.success() {
        Ok("自动部署完成".to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("自动部署失败: {}", error))
    }
}

// 初始化并自动部署（一键完成）
#[command]
async fn init_and_deploy(port: Option<u16>) -> Result<String, String> {
    let work_dir = get_stored_work_dir().ok_or("未设置工作目录")?;
    
    // 1. 首先初始化
    let init_output = std::process::Command::new("duck-cli")
        .arg("init")
        .arg("--force")
        .current_dir(&work_dir)
        .output()
        .map_err(|e| format!("执行初始化命令失败: {}", e))?;
    
    if !init_output.status.success() {
        let error = String::from_utf8_lossy(&init_output.stderr);
        return Err(format!("初始化失败: {}", error));
    }
    
    // 2. 然后自动部署
    let mut deploy_cmd = std::process::Command::new("duck-cli");
    deploy_cmd.arg("auto-upgrade-deploy")
        .arg("run")
        .current_dir(&work_dir);
    
    // 如果指定了端口，添加端口参数
    if let Some(p) = port {
        deploy_cmd.arg("--port").arg(p.to_string());
    }
    
    let deploy_output = deploy_cmd.output()
        .map_err(|e| format!("执行自动部署命令失败: {}", e))?;
    
    if deploy_output.status.success() {
        Ok("初始化和自动部署完成".to_string())
    } else {
        let error = String::from_utf8_lossy(&deploy_output.stderr);
        Err(format!("自动部署失败: {}", error))
    }
}

// 计算目录大小的辅助函数
fn calculate_directory_size(dir: &PathBuf) -> Result<f64, Box<dyn std::error::Error>> {
    let mut total_size = 0u64;
    
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
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
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .manage(AppGlobalState::default()) // 注册全局状态
        .invoke_handler(tauri::generate_handler![
            // 新的UI优化命令
            get_app_state,
            set_working_directory,
            check_system_requirements,
            init_client_with_progress,
            download_package_with_progress,
            get_services_status,
            start_services_monitoring,
            get_ui_configuration,
            update_ui_configuration,
            get_current_tasks,
            cancel_task,
            // 新增的平台和系统检查命令
            get_platform,
            check_storage_space,
            check_system_storage,
            open_file_manager,
            
            // 保留原有命令（兼容性）
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
            set_work_directory,
            select_work_directory,
            open_data_directory,
            open_backup_directory,
            open_cache_directory,
            clear_cache,
            init_client,
            auto_deploy_service,
            init_and_deploy,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
