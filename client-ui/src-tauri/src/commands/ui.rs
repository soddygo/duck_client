use tauri::command;
use super::types::UiConfig;

/// 获取UI配置
#[command]
pub async fn get_ui_config() -> Result<UiConfig, String> {
    Ok(UiConfig {
        theme: "light".to_string(),
        language: "zh-CN".to_string(),
        auto_refresh: true,
        refresh_interval: 30,
    })
}

/// 更新UI配置
#[command]
pub async fn update_ui_config(_config: UiConfig) -> Result<String, String> {
    // 这里可以保存配置到文件或数据库
    // 暂时只返回成功
    Ok("UI配置已更新".to_string())
} 