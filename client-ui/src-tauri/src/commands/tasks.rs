use tauri::{command, AppHandle, Manager};
use super::types::{TaskHandle, AppGlobalState};

/// 获取当前任务
#[command]
pub async fn get_current_tasks(
    app_handle: AppHandle,
) -> Result<Vec<TaskHandle>, String> {
    let state = app_handle.state::<AppGlobalState>();
    let tasks = state.current_tasks.read().await;
    Ok(tasks.values().cloned().collect())
}

/// 取消任务
#[command]
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