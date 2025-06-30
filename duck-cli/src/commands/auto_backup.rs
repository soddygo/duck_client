use client_core::error::Result;
use client_core::constants::{docker, timeout, cron};
use crate::app::CliApp;
use crate::commands::{docker_service, backup};
use crate::docker_utils;
use tracing::{info, warn, error, debug, instrument};
use std::path::Path;

/// 执行自动备份流程：停止服务 -> 备份 -> 重启服务
#[instrument(skip(app))]
pub async fn run_auto_backup(app: &mut CliApp) -> Result<()> {
    info!("开始自动备份流程");
    
    let backup_start_time = chrono::Utc::now();
    let mut backup_success = false;
    
    // 1. 检查Docker服务状态
    debug!("检查Docker服务状态");
    let service_running = check_docker_service_status(app).await?;
    
    if service_running {
        // 2. 停止Docker服务
        info!("停止Docker服务以进行备份");
        docker_service::stop_docker_services(app).await?;
        
        // 等待服务完全停止
        info!("等待Docker服务完全停止");
        let compose_path = client_core::constants::docker::get_compose_file_path();
        if !docker_utils::wait_for_compose_services_stopped(&compose_path, timeout::SERVICE_STOP_TIMEOUT).await? {
            warn!("等待服务停止超时，但继续进行备份");
        }
    } else {
        info!("Docker服务未运行，直接进行备份");
    }
    
    // 3. 执行备份
    info!("开始执行备份操作");
    match backup::run_backup(app).await {
        Ok(_) => {
            backup_success = true;
            info!("备份执行成功");
        }
        Err(e) => {
            error!(error = %e, "备份执行失败");
            // 记录失败但继续执行后续步骤
        }
    }
    
    // 记录备份执行时间和结果
    {
        let config_manager = client_core::config_manager::ConfigManager::new(&app.database);
        if let Err(e) = config_manager.update_last_backup_time(backup_start_time, backup_success).await {
            warn!(error = %e, "记录备份时间失败");
        }
    }
    
    if service_running {
        // 4. 重新启动Docker服务
        info!("重新启动Docker服务");
        docker_service::start_docker_services(app).await?;
        
        // 等待服务启动完成
        info!("等待Docker服务完全启动");
        let compose_path = client_core::constants::docker::get_compose_file_path();
        if docker_utils::wait_for_compose_services_started(&compose_path, timeout::SERVICE_START_TIMEOUT).await? {
            if backup_success {
                info!("自动备份流程完成，服务已重新启动");
            } else {
                warn!("自动备份流程完成（备份失败），服务已重新启动");
            }
        } else {
            warn!("等待服务启动超时，需要手动检查服务状态");
            
            // 最后再检查一次状态
            match check_docker_service_status(app).await {
                Ok(true) => {
                    debug!("最终检查：服务已正常启动");
                }
                Ok(false) => {
                    debug!("最终检查：服务未正常启动");
                }
                Err(e) => {
                    error!(error = %e, "最终检查失败");
                }
            }
        }
    } else {
        if backup_success {
            info!("自动备份流程完成");
        } else {
            warn!("自动备份流程完成（备份失败）");
        }
    }
    
    // 如果备份失败，返回错误
    if !backup_success {
        return Err(client_core::error::DuckError::custom("自动备份执行失败"));
    }
    
    Ok(())
}

/// 配置自动备份的cron表达式
#[instrument(skip(app))]
pub async fn configure_cron(app: &mut CliApp, expression: Option<String>) -> Result<()> {
    let config_manager = client_core::config_manager::ConfigManager::new(&app.database);
    
    match expression {
        Some(expr) => {
            debug!(expression = %expr, "尝试设置自动备份cron表达式");
            
            // 验证cron表达式
            if validate_cron_expression(&expr) {
                // 保存cron表达式到数据库
                config_manager.set_auto_backup_cron(&expr).await?;
                info!(expression = %expr, "设置自动备份cron表达式成功");
                
                info!("注意：当前版本暂未实现定时任务功能，请使用系统cron手动配置");
            } else {
                error!(expression = %expr, "无效的cron表达式");
                return Err(client_core::error::DuckError::custom(
                    format!("无效的cron表达式: {}", expr)
                ));
            }
        }
        None => {
            debug!("显示当前自动备份配置");
            // 显示当前配置
            let config = config_manager.get_auto_backup_config().await?;
            info!(
                cron_expression = %config.cron_expression,
                enabled = config.enabled,
                last_backup_at = ?config.last_backup_at,
                consecutive_failures = config.consecutive_failures,
                max_failures = config.max_failures,
                "当前自动备份配置"
            );
        }
    }
    
    Ok(())
}

/// 设置自动备份启用状态
#[instrument(skip(app))]
pub async fn set_enabled(app: &mut CliApp, enabled: Option<bool>) -> Result<()> {
    let config_manager = client_core::config_manager::ConfigManager::new(&app.database);
    
    match enabled {
        Some(enable) => {
            debug!(enabled = enable, "设置自动备份启用状态");
            // 保存启用状态到数据库
            config_manager.set_auto_backup_enabled(enable).await?;
            if enable {
                info!("启用自动备份");
            } else {
                info!("禁用自动备份");
            }
            
            info!("注意：当前版本暂未实现定时任务功能，请使用系统cron手动配置");
        }
        None => {
            debug!("显示当前自动备份启用状态");
            // 显示当前状态
            let config = config_manager.get_auto_backup_config().await?;
            info!(
                enabled = config.enabled,
                cron_expression = %config.cron_expression,
                "自动备份状态"
            );
        }
    }
    
    Ok(())
}

/// 显示自动备份状态
#[instrument(skip(app))]
pub async fn show_status(app: &mut CliApp) -> Result<()> {
    debug!("显示自动备份状态信息");
    let config_manager = client_core::config_manager::ConfigManager::new(&app.database);
    
    info!("自动备份状态: 功能已实现, 定时任务需要手动配置系统cron, 流程为停止服务->备份数据->重启服务");
    
    // 显示配置状态
    let config = config_manager.get_auto_backup_config().await?;
    info!(
        enabled = config.enabled,
        cron_expression = %config.cron_expression,
        last_backup_at = ?config.last_backup_at,
        consecutive_failures = config.consecutive_failures,
        max_failures = config.max_failures,
        "自动备份配置信息"
    );
    
    // 显示最近的备份
    info!("显示最近的备份记录");
    backup::run_list_backups(app).await?;
    
    Ok(())
}

/// 检查Docker服务状态
#[instrument(skip(app))]
async fn check_docker_service_status(app: &mut CliApp) -> Result<bool> {
    let compose_path = client_core::constants::docker::get_compose_file_path();
    match docker_utils::check_compose_services_running(&compose_path).await {
        Ok(running) => {
            debug!(running, "Docker服务状态检查结果");
            Ok(running)
        }
        Err(e) => {
            warn!(error = %e, "检查Docker服务状态失败，回退到简化检查");
            // 回退到原来的简化实现
            match docker_service::check_docker_services_status(app).await {
                Ok(_) => {
                    debug!("回退检查：服务运行中");
                    Ok(true)
                }
                Err(_) => {
                    debug!("回退检查：服务未运行");
                    Ok(false)
                }
            }
        }
    }
}

/// 验证cron表达式格式
fn validate_cron_expression(expr: &str) -> bool {
    // 简单的cron表达式验证
    let parts: Vec<&str> = expr.split_whitespace().collect();
    
    // 标准cron表达式应该有5个字段: 分 时 日 月 周
    if parts.len() != cron::CRON_FIELDS_COUNT {
        return false;
    }
    
    // 基础格式检查（这里可以更严格）
    for part in parts {
        if part.is_empty() {
            return false;
        }
    }
    
    true
}

fn check_docker_files_exist() -> std::result::Result<Vec<String>, Box<dyn std::error::Error>> {
    let compose_path = client_core::constants::docker::get_compose_file_path();
    let mut missing_files = Vec::new();
    
    if !compose_path.exists() {
        missing_files.push(format!("docker-compose.yml 文件不存在: {}", compose_path.display()));
    }
    
    Ok(missing_files)
} 