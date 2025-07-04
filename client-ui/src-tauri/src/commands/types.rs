use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::{RwLock, Mutex};
use std::sync::Arc;
use client_core::db::DuckDbManager;

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
    /// 获取或初始化数据库管理器（懒加载单例模式）
    pub async fn get_or_init_db_manager(&self, working_dir: &std::path::Path) -> Result<DuckDbManager, String> {
        let mut db_manager_guard = self.db_manager.lock().await;
        
        if let Some(ref manager) = *db_manager_guard {
            // 如果已经初始化，直接返回克隆（DuckDbManager是Clone的）
            Ok(manager.clone())
        } else {
            // 如果未初始化，创建新的管理器
            let db_path = working_dir.join("data").join("duck_client.db");
            let manager = DuckDbManager::new(&db_path)
                .await
                .map_err(|e| format!("初始化数据库管理器失败: {}", e))?;
            
            // 保存到全局状态
            *db_manager_guard = Some(manager.clone());
            Ok(manager)
        }
    }
    
    /// 重置数据库管理器（当工作目录改变时使用）
    pub async fn reset_db_manager(&self) {
        let mut db_manager_guard = self.db_manager.lock().await;
        *db_manager_guard = None;
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