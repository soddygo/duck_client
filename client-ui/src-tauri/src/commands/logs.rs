use tauri::command;
use super::types::ActivityLogEntry;

/// 获取活动日志
#[command]
pub async fn get_activity_logs() -> Result<Vec<ActivityLogEntry>, String> {
    use chrono::Utc;
    
    let now = Utc::now();
    let logs = vec![
        ActivityLogEntry {
            timestamp: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            action: "系统启动".to_string(),
            status: "成功".to_string(),
            details: Some("应用程序已成功启动".to_string()),
        },
        ActivityLogEntry {
            timestamp: (now - chrono::Duration::minutes(5)).format("%Y-%m-%d %H:%M:%S").to_string(),
            action: "检查更新".to_string(),
            status: "完成".to_string(),
            details: Some("已检查最新版本".to_string()),
        },
    ];
    
    Ok(logs)
} 