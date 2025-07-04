use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::{RwLock, Mutex};
use std::sync::Arc;
use client_core::{
    config::AppConfig,
    database::Database,
    constants::config,
    container::DockerManager,
    authenticated_client::AuthenticatedClient,
    db::DuckDbManager,
};

// ================== 通用数据结构 ==================

#[derive(Serialize)]
pub struct VersionInfo {
    pub client_version: String,
    pub service_version: String,
}

#[derive(Serialize)]
pub struct UpgradeInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_notes: Option<String>,
    pub download_size: Option<String>,
    pub estimated_time: Option<String>,
}

#[derive(Serialize)]
pub struct ServiceInfo {
    pub name: String,
    pub status: String,
    pub uptime: Option<String>,
    pub ports: Vec<String>,
}

#[derive(Serialize)]
pub struct ActivityLogEntry {
    pub timestamp: String,
    pub action: String,
    pub status: String,
    pub details: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct UpgradeCompletedEvent {
    pub success: bool,
    pub version: String,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub language: String,
    pub auto_refresh: bool,
    pub refresh_interval: u32,
}

// ================== 系统检查相关结构 ==================

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

// ================== 应用状态管理 ==================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateInfo {
    pub state: String,
    pub initialized: bool,
    pub working_directory: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHandle {
    pub task_id: String,
    pub task_type: String,
    pub status: String,
    pub progress: f64,
}

// 全局状态管理
pub struct AppGlobalState {
    pub current_tasks: RwLock<HashMap<String, TaskHandle>>,
    pub working_directory: RwLock<Option<PathBuf>>,
    pub db_manager: Arc<Mutex<Option<DuckDbManager>>>, // 全局单例数据库管理器
}

impl Default for AppGlobalState {
    fn default() -> Self {
        Self {
            current_tasks: RwLock::new(HashMap::new()),
            working_directory: RwLock::new(None),
            db_manager: Arc::new(Mutex::new(None)),
        }
    }
}

impl AppGlobalState {
    /// 从数据库加载保存的工作目录设置（应用启动时调用）
    pub async fn load_working_directory_from_db(&self) -> Result<(), String> {
        // 先尝试从默认位置加载数据库，检查是否有保存的工作目录设置
        let default_work_dir = get_default_work_directory();
        let default_db_path = default_work_dir.join("data").join(config::DATABASE_FILE_NAME);
        
        if default_db_path.exists() {
            // 如果默认位置有数据库，尝试加载工作目录设置
            match DuckDbManager::new(&default_db_path).await {
                Ok(manager) => {
                    if let Ok(Some(saved_dir)) = manager.get_config("app.working_directory").await {
                        let saved_path = std::path::PathBuf::from(saved_dir);
                        if saved_path.exists() {
                            let mut working_dir = self.working_directory.write().await;
                            *working_dir = Some(saved_path);
                            return Ok(());
                        }
                    }
                }
                Err(_) => {
                    // 如果加载失败，继续使用默认目录
                }
            }
        }
        
        // 如果没有保存的设置或加载失败，设置为默认目录
        let mut working_dir = self.working_directory.write().await;
        *working_dir = Some(default_work_dir);
        Ok(())
    }
    
    /// 保存工作目录设置到数据库
    pub async fn save_working_directory_to_db(&self, new_dir: &std::path::Path) -> Result<(), String> {
        // 使用新的工作目录路径保存设置
        let db_path = new_dir.join("data").join(config::DATABASE_FILE_NAME);
        
        // 确保data目录存在
        let data_dir = new_dir.join("data");
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| format!("创建data目录失败: {}", e))?;
        }
        
        let manager = DuckDbManager::new(&db_path).await
            .map_err(|e| format!("初始化数据库失败: {}", e))?;
        
        manager.set_config("app.working_directory", &new_dir.to_string_lossy())
            .await
            .map_err(|e| format!("保存工作目录配置失败: {}", e))?;
        
        Ok(())
    }
    
    /// 获取或初始化数据库管理器（✅ 全局单例模式）
    pub async fn get_or_init_db_manager(&self, working_dir: &std::path::Path) -> Result<DuckDbManager, String> {
        let mut manager_guard = self.db_manager.lock().await;
        
        if let Some(ref manager) = *manager_guard {
            return Ok(manager.clone());
        }
        
        // 使用常量构建数据库路径
        let db_path = working_dir.join("data").join(config::DATABASE_FILE_NAME); // 使用常量
        let new_manager = DuckDbManager::new(&db_path)
            .await
            .map_err(|e| format!("初始化数据库管理器失败: {}", e))?;
        
        *manager_guard = Some(new_manager.clone());
        Ok(new_manager)
    }
    
    /// 重置数据库管理器（当工作目录改变时使用）
    pub async fn reset_db_manager(&self) {
        let mut db_manager_guard = self.db_manager.lock().await;
        *db_manager_guard = None;
    }
}

// 获取默认工作目录
fn get_default_work_directory() -> std::path::PathBuf {
    if let Some(home_dir) = dirs::home_dir() {
        home_dir.join("Documents").join("DuckClient")
    } else {
        // 如果无法获取home目录，使用当前目录
        std::path::PathBuf::from("./DuckClient")
    }
}

// ================== 事件数据结构 ==================

#[derive(Debug, Clone, Serialize)]
pub struct InitProgressEvent {
    pub task_id: String,
    pub stage: String,
    pub message: String,
    pub percentage: f64,
    pub current_step: usize,
    pub total_steps: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct InitCompletedEvent {
    pub task_id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgressEvent {
    pub task_id: String,
    pub file_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub download_speed: f64,
    pub eta_seconds: u64,
    pub percentage: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadCompletedEvent {
    pub task_id: String,
    pub success: bool,
    pub error: Option<String>,
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