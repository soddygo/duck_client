use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;
use tauri::{AppHandle, Manager, Emitter};
use tokio::sync::RwLock;
use uuid::Uuid;
use tokio_stream::StreamExt;
use duck_cli::{
    init_with_progress, download_with_progress, get_system_info, 
    get_ui_config, update_ui_config, monitor_services
};

// 状态数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateInfo {
    pub state: String,
    pub initialized: bool,
    pub working_directory: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRequirements {
    pub os_supported: bool,
    pub docker_available: bool,
    pub storage_sufficient: bool,
    pub available_space_gb: f64,
    pub required_space_gb: f64,
    pub platform_specific: PlatformSpecificChecks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSpecificChecks {
    pub docker_desktop_installed: bool,
    pub wsl_enabled: bool, // Windows only
    pub homebrew_docker: bool, // macOS only
    pub docker_group_member: bool, // Linux only
}

// 全局状态管理
pub struct AppGlobalState {
    pub current_tasks: RwLock<HashMap<String, TaskHandle>>,
    pub working_directory: RwLock<Option<PathBuf>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHandle {
    pub task_id: String,
    pub task_type: String,
    pub status: String,
    pub progress: f64,
}

impl Default for AppGlobalState {
    fn default() -> Self {
        Self {
            current_tasks: RwLock::new(HashMap::new()),
            working_directory: RwLock::new(None),
        }
    }
}

// ================== 应用状态管理命令 ==================

#[tauri::command]
pub async fn get_app_state(
    app_handle: AppHandle,
) -> Result<AppStateInfo, String> {
    let state = app_handle.state::<AppGlobalState>();
    let working_dir = state.working_directory.read().await;
    
    let (current_work_dir, has_user_set_dir) = if let Some(dir) = working_dir.as_ref() {
        (dir.clone(), true)
    } else {
        (get_default_work_directory(), false)
    };
    
    let initialized = current_work_dir.join("config.toml").exists() && 
                     current_work_dir.join("history.db").exists();
    
    Ok(AppStateInfo {
        state: if initialized { "READY".to_string() } else { "UNINITIALIZED".to_string() },
        initialized,
        working_directory: if has_user_set_dir || initialized { 
            Some(current_work_dir.to_string_lossy().to_string()) 
        } else { 
            None 
        },
        last_error: None,
    })
}

#[tauri::command]
pub async fn set_working_directory(
    app_handle: AppHandle,
    directory: String,
) -> Result<(), String> {
    let path = PathBuf::from(directory);
    
    // 如果目录不存在，尝试创建它
    if !path.exists() {
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("无法创建工作目录: {}", e))?;
    }
    
    // 检查是否为有效目录
    if !path.is_dir() {
        return Err("指定的路径不是有效的目录".to_string());
    }
    
    let state = app_handle.state::<AppGlobalState>();
    let mut working_dir = state.working_directory.write().await;
    *working_dir = Some(path);
    
    Ok(())
}

// 获取默认工作目录
fn get_default_work_directory() -> PathBuf {
    if let Some(home_dir) = dirs::home_dir() {
        home_dir.join("Documents").join("DuckClient")
    } else {
        // 如果无法获取home目录，使用当前目录
        PathBuf::from("./DuckClient")
    }
}

// ================== 系统检查命令 ==================

#[tauri::command]
pub async fn check_system_requirements(
    directory: Option<String>,
) -> Result<SystemRequirements, String> {
    let system_info = get_system_info();
    
    // 检查存储空间
    let available_space_gb = if let Some(dir) = directory {
        let _path = PathBuf::from(dir);
        system_info.disk_space.available as f64 / (1024.0 * 1024.0 * 1024.0)
    } else {
        system_info.disk_space.available as f64 / (1024.0 * 1024.0 * 1024.0)
    };
    
    let required_space_gb = 60.0; // 60GB最小要求
    
    // 平台特定检查
    let platform_specific = PlatformSpecificChecks {
        docker_desktop_installed: system_info.docker_version.is_some(),
        wsl_enabled: check_wsl_status(),
        homebrew_docker: check_homebrew_docker(),
        docker_group_member: check_docker_group_membership(),
    };
    
    Ok(SystemRequirements {
        os_supported: matches!(system_info.os.as_str(), "windows" | "macos" | "linux"),
        docker_available: system_info.docker_version.is_some(),
        storage_sufficient: available_space_gb >= required_space_gb,
        available_space_gb,
        required_space_gb,
        platform_specific,
    })
}

// ================== 初始化命令 ==================

#[tauri::command]
pub async fn init_client_with_progress(
    app_handle: AppHandle,
    working_dir: String,
) -> Result<String, String> {
    let task_id = Uuid::new_v4().to_string();
    let path = PathBuf::from(working_dir);
    
    // 确保工作目录存在，如果不存在则创建
    if !path.exists() {
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("无法创建工作目录 {}: {}", path.display(), e))?;
    }
    
    // 更新全局状态中的工作目录
    let state = app_handle.state::<AppGlobalState>();
    {
        let mut working_directory = state.working_directory.write().await;
        *working_directory = Some(path.clone());
    }
    
    // 保存任务信息
    {
        let mut tasks = state.current_tasks.write().await;
        tasks.insert(task_id.clone(), TaskHandle {
            task_id: task_id.clone(),
            task_type: "initialization".to_string(),
            status: "starting".to_string(),
            progress: 0.0,
        });
    }
    
    let app_handle_clone = app_handle.clone();
    let task_id_clone = task_id.clone();
    
    // 启动初始化任务
    let cleanup_handle = app_handle_clone.clone();
    let cleanup_task_id = task_id_clone.clone();
    
    tokio::spawn(async move {
        let app_handle_for_progress = app_handle_clone.clone();
        let task_id_for_progress = task_id_clone.clone();
        let app_handle_for_completion = app_handle_clone.clone();
        let task_id_for_completion = task_id_clone.clone();
        
        let result = init_with_progress(&path, move |progress| {
            let task_id = task_id_for_progress.clone();
            let app_handle = app_handle_for_progress.clone();
            
            // 发送进度事件到前端
            let _ = app_handle.emit("init-progress", InitProgressEvent {
                task_id: task_id.clone(),
                stage: progress.stage,
                message: progress.message,
                percentage: progress.percentage,
                current_step: progress.current_step,
                total_steps: progress.total_steps,
            });
            
            // 更新任务状态
            tokio::spawn(async move {
                if let Some(state) = app_handle.try_state::<AppGlobalState>() {
                    let mut tasks = state.current_tasks.write().await;
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.progress = progress.percentage;
                        task.status = if progress.percentage >= 100.0 {
                            "completed".to_string()
                        } else {
                            "running".to_string()
                        };
                    }
                }
            });
        }).await;
        
        // 立即处理结果并发送完成事件，避免在await之后使用result
        let (success, error_message) = match result {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        
        // 发送完成事件
        let _ = app_handle_for_completion.emit("init-completed", InitCompletedEvent {
            task_id: task_id_for_completion,
            success,
            error: error_message,
        });
    });
    
    // 单独的清理任务
    tokio::spawn(async move {
        // 等待一小段时间让主任务完成
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // 清理任务
        if let Some(state) = cleanup_handle.try_state::<AppGlobalState>() {
            let mut tasks = state.current_tasks.write().await;
            tasks.remove(&cleanup_task_id);
        }
    });
    
    Ok(task_id)
}

#[tauri::command]
pub async fn download_package_with_progress(
    app_handle: AppHandle,
    url: String,
    target_dir: String,
) -> Result<String, String> {
    let task_id = Uuid::new_v4().to_string();
    let target_path = PathBuf::from(target_dir);
    
    let app_handle_clone = app_handle.clone();
    let task_id_clone = task_id.clone();
    
    // 启动下载任务
    tokio::spawn(async move {
        let app_handle_for_progress = app_handle_clone.clone();
        let task_id_for_progress = task_id_clone.clone();
        let app_handle_for_completion = app_handle_clone.clone();
        let task_id_for_completion = task_id_clone.clone();
        
        let result = download_with_progress(&url, &target_path, move |progress| {
            let _ = app_handle_for_progress.emit("download-progress", DownloadProgressEvent {
                task_id: task_id_for_progress.clone(),
                file_name: progress.file_name,
                downloaded_bytes: progress.downloaded_bytes,
                total_bytes: progress.total_bytes,
                download_speed: progress.download_speed,
                eta_seconds: progress.eta_seconds,
                percentage: progress.percentage,
                status: format!("{:?}", progress.status),
            });
        }).await;
        
        // 立即处理结果并发送完成事件，避免在await之后使用result
        let (success, error_message) = match result {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        
        // 发送完成事件
        let _ = app_handle_for_completion.emit("download-completed", DownloadCompletedEvent {
            task_id: task_id_for_completion,
            success,
            error: error_message,
        });
    });
    
    Ok(task_id)
}

// ================== 服务管理命令 ==================

#[tauri::command]
pub async fn get_services_status() -> Result<Vec<ServiceStatusInfo>, String> {
    let mut services_stream = monitor_services().await;
    let mut services = Vec::new();
    
    // 收集当前服务状态（非流式，一次性获取）
    for _ in 0..10 { // 最多等待10个服务状态更新
        if let Ok(service) = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            async { 
                services_stream.next().await 
            }
        ).await {
            if let Some(service_status) = service {
                services.push(ServiceStatusInfo {
                    name: service_status.name,
                    status: service_status.status,
                    health: service_status.health,
                    uptime_seconds: service_status.uptime,
                    cpu_usage: service_status.cpu_usage,
                    memory_usage_mb: service_status.memory_usage / (1024 * 1024),
                    ports: service_status.ports,
                });
            }
        } else {
            break;
        }
    }
    
    if services.is_empty() {
        // 如果没有获取到实时数据，返回模拟数据
        services.push(ServiceStatusInfo {
            name: "检查服务状态".to_string(),
            status: "正在检查...".to_string(),
            health: "unknown".to_string(),
            uptime_seconds: None,
            cpu_usage: 0.0,
            memory_usage_mb: 0,
            ports: vec![],
        });
    }
    
    Ok(services)
}

#[tauri::command]
pub async fn start_services_monitoring(app_handle: AppHandle) -> Result<(), String> {
    let app_handle_clone = app_handle.clone();
    
    tokio::spawn(async move {
        let mut services_stream = monitor_services().await;
        
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

// ================== 配置管理命令 ==================

#[tauri::command]
pub async fn get_ui_configuration() -> Result<serde_json::Value, String> {
    get_ui_config().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_ui_configuration(config: serde_json::Value) -> Result<(), String> {
    update_ui_config(config).await.map_err(|e| e.to_string())
}

// ================== 任务管理命令 ==================

#[tauri::command]
pub async fn get_current_tasks(
    app_handle: AppHandle,
) -> Result<Vec<TaskHandle>, String> {
    let state = app_handle.state::<AppGlobalState>();
    let tasks = state.current_tasks.read().await;
    Ok(tasks.values().cloned().collect())
}

#[tauri::command]
pub async fn cancel_task(
    app_handle: AppHandle,
    task_id: String,
) -> Result<(), String> {
    let state = app_handle.state::<AppGlobalState>();
    let mut tasks = state.current_tasks.write().await;
    
    if let Some(task) = tasks.get_mut(&task_id) {
        task.status = "cancelled".to_string();
        // 这里可以添加实际的任务取消逻辑
        tasks.remove(&task_id);
        Ok(())
    } else {
        Err("任务不存在".to_string())
    }
}

// ================== 事件数据结构 ==================

#[derive(Debug, Clone, Serialize)]
struct InitProgressEvent {
    task_id: String,
    stage: String,
    message: String,
    percentage: f64,
    current_step: usize,
    total_steps: usize,
}

#[derive(Debug, Clone, Serialize)]
struct InitCompletedEvent {
    task_id: String,
    success: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DownloadProgressEvent {
    task_id: String,
    file_name: String,
    downloaded_bytes: u64,
    total_bytes: u64,
    download_speed: f64,
    eta_seconds: u64,
    percentage: f64,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
struct DownloadCompletedEvent {
    task_id: String,
    success: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceStatusInfo {
    pub name: String,
    pub status: String,
    pub health: String,
    pub uptime_seconds: Option<u64>,
    pub cpu_usage: f64,
    pub memory_usage_mb: u64,
    pub ports: Vec<String>,
}

// ================== 平台特定函数 ==================

#[cfg(target_os = "windows")]
fn check_wsl_status() -> bool {
    use std::process::Command;
    Command::new("wsl")
        .arg("--status")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn check_wsl_status() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn check_homebrew_docker() -> bool {
    use std::process::Command;
    Command::new("brew")
        .args(&["list", "docker"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn check_homebrew_docker() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn check_docker_group_membership() -> bool {
    use std::process::Command;
    Command::new("groups")
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .contains("docker")
        })
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn check_docker_group_membership() -> bool {
    false
}

// ================== 新增的平台和系统检查命令 ==================

#[tauri::command]
pub async fn get_platform() -> Result<String, String> {
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    };
    
    Ok(platform.to_string())
}

#[derive(serde::Serialize)]
pub struct StorageInfo {
    pub path: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub available_space_gb: f64,
    pub required_space_gb: f64,
    pub sufficient: bool,
}

#[tauri::command]
pub async fn check_system_storage() -> Result<StorageInfo, String> {
    let system_info = get_system_info();
    
    let total_bytes = system_info.disk_space.total;
    let available_bytes = system_info.disk_space.available;
    let used_bytes = total_bytes - available_bytes;
    let available_space_gb = available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let required_space_gb = 60.0;
    
    Ok(StorageInfo {
        path: "系统磁盘".to_string(),
        total_bytes,
        available_bytes,
        used_bytes,
        available_space_gb,
        required_space_gb,
        sufficient: available_space_gb >= required_space_gb,
    })
}

#[tauri::command]
pub async fn check_storage_space(path: String) -> Result<StorageInfo, String> {
    let path_buf = PathBuf::from(&path);
    
    // 确保路径存在，如果不存在则创建父目录用于检测
    let check_path = if path_buf.exists() {
        path_buf
    } else {
        // 如果路径不存在，使用父目录或根目录进行检测
        path_buf.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| {
                #[cfg(target_os = "windows")]
                { PathBuf::from("C:\\") }
                #[cfg(not(target_os = "windows"))]
                { PathBuf::from("/") }
            })
    };
    
    let required_bytes = 60u64 * 1024 * 1024 * 1024; // 60GB
    
    // 使用跨平台的方法获取磁盘空间
    #[cfg(target_os = "windows")]
    {
        use std::ffi::CString;
        use std::os::windows::ffi::OsStrExt;
        use winapi::um::fileapi::GetDiskFreeSpaceExW;
        use winapi::shared::minwindef::BOOL;
        
        let path_wide: Vec<u16> = check_path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mut free_bytes_available = 0u64;
        let mut total_number_of_bytes = 0u64;
        let mut total_number_of_free_bytes = 0u64;
        
        let result = unsafe {
            GetDiskFreeSpaceExW(
                path_wide.as_ptr(),
                &mut free_bytes_available,
                &mut total_number_of_bytes,
                &mut total_number_of_free_bytes,
            )
        };
        
        if result == 0 {
            return Err("无法获取磁盘空间信息".to_string());
        }
        
        let total_bytes = total_number_of_bytes;
        let available_bytes = free_bytes_available;
        let used_bytes = total_bytes - available_bytes;
        let available_space_gb = available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let required_space_gb = required_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let sufficient = available_bytes >= required_bytes;
        
        Ok(StorageInfo {
            path,
            total_bytes,
            available_bytes,
            used_bytes,
            available_space_gb,
            required_space_gb,
            sufficient,
        })
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use libc::{statvfs, c_char};
        
        let path_c = CString::new(check_path.to_string_lossy().as_bytes())
            .map_err(|e| format!("路径转换失败: {}", e))?;
        
        let mut stat: statvfs = unsafe { std::mem::zeroed() };
        let result = unsafe { statvfs(path_c.as_ptr(), &mut stat) };
        
        if result != 0 {
            return Err("无法获取磁盘空间信息".to_string());
        }
        
        let total_bytes = (stat.f_blocks as u64) * (stat.f_frsize as u64);
        let available_bytes = (stat.f_bavail as u64) * (stat.f_frsize as u64);
        let used_bytes = total_bytes - available_bytes;
        let available_space_gb = available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let required_space_gb = required_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let sufficient = available_bytes >= required_bytes;
        
        Ok(StorageInfo {
            path,
            total_bytes,
            available_bytes,
            used_bytes,
            available_space_gb,
            required_space_gb,
            sufficient,
        })
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::ffi::CString;
        use libc::{statvfs, c_char};
        
        let path_c = CString::new(check_path.to_string_lossy().as_bytes())
            .map_err(|e| format!("路径转换失败: {}", e))?;
        
        let mut stat: statvfs = unsafe { std::mem::zeroed() };
        let result = unsafe { statvfs(path_c.as_ptr(), &mut stat) };
        
        if result != 0 {
            return Err("无法获取磁盘空间信息".to_string());
        }
        
        let total_bytes = (stat.f_blocks as u64) * (stat.f_frsize as u64);
        let available_bytes = (stat.f_bavail as u64) * (stat.f_frsize as u64);
        let used_bytes = total_bytes - available_bytes;
        let available_space_gb = available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let required_space_gb = required_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let sufficient = available_bytes >= required_bytes;
        
        Ok(StorageInfo {
            path,
            total_bytes,
            available_bytes,
            used_bytes,
            available_space_gb,
            required_space_gb,
            sufficient,
        })
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        // 对于其他平台，返回错误
        Err("当前平台不支持磁盘空间检测".to_string())
    }
}

#[tauri::command]
pub async fn open_file_manager(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    
    if !path_buf.exists() {
        return Err("指定的路径不存在".to_string());
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("无法打开文件管理器: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("无法打开文件管理器: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("无法打开文件管理器: {}", e))?;
    }
    
    Ok(())
} 