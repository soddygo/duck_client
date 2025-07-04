use tauri::{command, AppHandle, Manager};
use std::path::PathBuf;
use super::types::{AppGlobalState, AppStateInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryInfo {
    pub path: String,
    pub exists: bool,
    pub is_initialized: bool,
    pub available_space_gb: f64,
    pub error: Option<String>,
}

/// 获取应用状态
#[command]
pub async fn get_app_state(
    app_handle: AppHandle,
) -> Result<AppStateInfo, String> {
    let state = app_handle.state::<AppGlobalState>();
    
    // 首先检查是否有设置的工作目录
    let working_dir = state.working_directory.read().await;
    let current_work_dir = if let Some(dir) = working_dir.as_ref() {
        dir.clone()
    } else {
        // 如果没有设置工作目录，尝试从数据库加载
        drop(working_dir); // 释放读锁
        
        // 尝试从数据库加载保存的工作目录
        let _ = state.load_working_directory_from_db().await;
        
        // 重新获取工作目录
        let working_dir = state.working_directory.read().await;
        working_dir.as_ref().unwrap_or(&get_default_work_directory()).clone()
    };
    
    let initialized = current_work_dir.join("data").join("config.toml").exists() && 
                     current_work_dir.join("data").join("duck_client.db").exists();
    
    Ok(AppStateInfo {
        state: if initialized { "READY".to_string() } else { "UNINITIALIZED".to_string() },
        initialized,
        working_directory: Some(current_work_dir.to_string_lossy().to_string()),
        last_error: None,
    })
}

/// 设置工作目录
#[command]
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
    
    // 设置新的工作目录
    let mut working_dir = state.working_directory.write().await;
    *working_dir = Some(path.clone());
    drop(working_dir); // 释放写锁
    
    // ✅ 保存工作目录设置到数据库中（持久化）
    state.save_working_directory_to_db(&path).await
        .map_err(|e| format!("保存工作目录设置失败: {}", e))?;
    
    // ✅ 重置数据库管理器，确保使用新目录的数据库
    state.reset_db_manager().await;
    
    Ok(())
}

/// 获取工作目录
#[command]
pub async fn get_working_directory() -> Result<String, String> {
    use std::env;
    
    match env::current_dir() {
        Ok(path) => Ok(path.to_string_lossy().to_string()),
        Err(e) => Err(format!("获取工作目录失败: {}", e)),
    }
}

/// 重设工作目录
#[command]
pub async fn reset_working_directory(new_path: String) -> Result<String, String> {
    use std::env;
    use std::path::Path;
    
    let path = Path::new(&new_path);
    if !path.exists() {
        return Err("指定的目录不存在".to_string());
    }
    
    env::set_current_dir(path)
        .map_err(|e| format!("切换工作目录失败: {}", e))?;
    
    Ok("工作目录已切换".to_string())
}

/// 打开目录
#[command]
pub async fn open_directory(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }
    
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

/// 初始化应用状态（应用启动时调用）
#[command]
pub async fn initialize_app_state(
    app_handle: AppHandle,
) -> Result<(), String> {
    let state = app_handle.state::<AppGlobalState>();
    
    // 尝试从数据库加载保存的工作目录
    state.load_working_directory_from_db().await
        .map_err(|e| format!("加载工作目录设置失败: {}", e))?;
    
    Ok(())
} 