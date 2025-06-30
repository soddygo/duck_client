use crate::app::CliApp;
use crate::commands::{backup, docker_service, update};
use crate::docker_utils;
use client_core::constants::{docker, timeout};
use client_core::error::Result;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// 执行自动升级部署流程
pub async fn run_auto_upgrade_deploy(app: &mut CliApp) -> Result<()> {
    info!("🚀 开始自动升级部署流程...");

    // 1. 下载最新的docker.zip服务版本文件
    info!("开始下载最新的Docker服务版本");
    info!("📥 正在下载最新的Docker服务版本...");
    update::run_upgrade(app, true, false).await?; // 全量下载

    // 2. 检查Docker服务状态
    info!("检查Docker服务状态");
    let service_running = check_docker_service_status(app).await?;

    // 3. 检查是否需要备份
    let need_backup = if service_running {
        // 服务运行中，需要先停止服务再备份
        info!("Docker服务正在运行，准备停止服务进行备份");
        info!("⏹️  正在停止Docker服务以进行备份...");
        docker_service::stop_docker_services(app).await?;

        // 等待服务完全停止（最多等待30秒）
        info!("⏳ 等待Docker服务完全停止...");
        let compose_path = client_core::constants::docker::get_compose_file_path();
        if !docker_utils::wait_for_compose_services_stopped(
            &compose_path,
            timeout::SERVICE_STOP_TIMEOUT,
        )
        .await?
        {
            warn!("等待服务停止超时，但继续进行备份");
            warn!("⚠️  等待服务停止超时，但继续进行备份");
        }

        true
    } else {
        info!("Docker服务未运行，检查是否有文件需要备份");
        info!("ℹ️  Docker服务未运行，检查是否有文件需要备份...");

        // 检查docker目录是否存在且有文件需要备份
        check_docker_files_exist().await?
    };

    // 4. 根据需要执行备份
    if need_backup {
        info!("开始执行备份");
        info!("💾 正在创建备份...");
        backup::run_backup(app).await?;
    } else {
        info!("跳过备份步骤，没有需要备份的文件");
        info!("⏭️  跳过备份步骤，没有需要备份的文件");
    }

    // 5. 自动部署服务
    info!("开始部署Docker服务");
    info!("🔄 正在部署Docker服务...");
    docker_service::deploy_docker_services(app).await?;

    // 6. 启动服务
    info!("启动Docker服务");
    info!("▶️  正在启动Docker服务...");
    docker_service::start_docker_services(app).await?;

    // 等待服务启动完成（最多等待90秒，因为部署后启动可能需要更长时间）
    info!("⏳ 等待Docker服务完全启动...");
    let compose_path = client_core::constants::docker::get_compose_file_path();
    if docker_utils::wait_for_compose_services_started(&compose_path, timeout::DEPLOY_START_TIMEOUT)
        .await?
    {
        info!("✅ 自动升级部署完成，服务已成功启动");
        info!("自动升级部署流程成功完成");
    } else {
        warn!("⚠️  等待服务启动超时，请手动检查服务状态");
        warn!("⚠️  等待服务启动超时，请手动检查服务状态");

        // 最后再检查一次状态
        match check_docker_service_status(app).await {
            Ok(true) => info!("🔍 最终检查：服务似乎已正常启动"),
            Ok(false) => {
                info!("🔍 最终检查：服务可能未正常启动");
                info!("📊 详细状态检查:");
                let _ = docker_service::check_docker_services_status(app).await;
            }
            Err(e) => warn!("🔍 最终检查失败: {}", e),
        }
    }

    Ok(())
}

/// 预约延迟执行自动升级部署
pub async fn schedule_delayed_deploy(app: &mut CliApp, time: u32, unit: &str) -> Result<()> {
    // 计算延迟时间（转换为秒）
    let delay_seconds = match unit.to_lowercase().as_str() {
        "minutes" | "minute" | "min" => time * 60,
        "hours" | "hour" | "h" => time * 3600,
        "days" | "day" | "d" => time * 86400,
        _ => {
            error!("不支持的时间单位: {}", unit);
            return Err(client_core::error::DuckError::custom(format!(
                "不支持的时间单位: {}，支持的单位: hours, minutes, days",
                unit
            )));
        }
    };

    let delay_duration = Duration::from_secs(delay_seconds as u64);
    let scheduled_at = chrono::Utc::now() + chrono::Duration::seconds(delay_seconds as i64);

    // 创建升级任务记录
    let task = client_core::config_manager::AutoUpgradeTask {
        id: None,
        task_type: "delayed".to_string(),
        target_version: None, // 最新版本
        scheduled_at,
        delay_amount: Some(time as i32),
        delay_unit: Some(unit.to_string()),
        status: "pending".to_string(),
        progress: 0,
        error_message: None,
        backup_created: false,
        backup_id: None,
    };

    let task_id = {
        let config_manager = client_core::config_manager::ConfigManager::new(&app.database);
        config_manager.create_auto_upgrade_task(&task).await?
    };

    info!("⏰ 已安排延迟执行自动升级部署");
    info!("   任务ID: {}", task_id);
    info!("   延迟时间: {} {}", time, unit);
    println!("   预计执行时间: {} 后", format_duration(delay_duration));
    info!(
        "   计划执行时间: {}",
        scheduled_at.format("%Y-%m-%d %H:%M:%S UTC")
    );

    info!(
        "安排延迟执行自动升级部署: {} {}，任务ID: {}",
        time, unit, task_id
    );

    // 更新任务状态为进行中
    {
        let config_manager = client_core::config_manager::ConfigManager::new(&app.database);
        config_manager
            .update_upgrade_task_status(&task_id, "in_progress", Some(0), None)
            .await?;
    }

    // 开始延迟等待
    info!("⏳ 等待中...");

    // 这里可以优化为后台任务，避免阻塞
    sleep(delay_duration).await;

    info!("🔔 延迟时间到，开始执行自动升级部署");
    info!("延迟时间到，开始执行自动升级部署，任务ID: {}", task_id);

    // 执行自动升级部署
    match run_auto_upgrade_deploy(app).await {
        Ok(_) => {
            let config_manager = client_core::config_manager::ConfigManager::new(&app.database);
            config_manager
                .update_upgrade_task_status(&task_id, "completed", Some(100), None)
                .await?;
            info!("✅ 延迟升级部署任务完成");
        }
        Err(e) => {
            let config_manager = client_core::config_manager::ConfigManager::new(&app.database);
            config_manager
                .update_upgrade_task_status(&task_id, "failed", None, Some(&e.to_string()))
                .await?;
            error!("延迟升级部署任务失败: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// 显示自动升级部署状态
pub async fn show_status(app: &mut CliApp) -> Result<()> {
    let config_manager = client_core::config_manager::ConfigManager::new(&app.database);

    info!("📊 自动升级部署状态信息:");
    info!("   功能状态: 已实现");
    info!("   流程说明: 下载最新版本 -> 智能备份 -> 部署服务 -> 启动服务");

    // 显示待执行的升级任务
    match config_manager.get_pending_upgrade_tasks().await {
        Ok(tasks) => {
            if tasks.is_empty() {
                info!("📋 升级任务: 当前没有待执行的升级任务");
            } else {
                info!("📋 待执行的升级任务:");
                for (task_id, task) in tasks {
                    info!("   - 任务ID: {}", task_id);
                    info!("     类型: {}", task.task_type);
                    info!("     状态: {}", task.status);
                    info!(
                        "     计划执行时间: {}",
                        task.scheduled_at.format("%Y-%m-%d %H:%M:%S UTC")
                    );
                    if let Some(delay_amount) = task.delay_amount {
                        if let Some(delay_unit) = &task.delay_unit {
                            info!("     延迟设置: {} {}", delay_amount, delay_unit);
                        }
                    }
                    info!("     进度: {}%", task.progress);
                    if let Some(error) = &task.error_message {
                        warn!("     错误信息: {}", error);
                    }
                }
            }
        }
        Err(e) => {
            warn!("⚠️  获取升级任务信息失败: {}", e);
            info!("   注意: 当前版本的任务查询功能有限");
        }
    }

    // 显示当前Docker服务状态
    info!("🐳 当前Docker服务状态:");
    docker_service::check_docker_services_status(app).await?;

    // 显示最近的备份
    info!("📝 最近的备份:");
    backup::run_list_backups(app).await?;

    Ok(())
}

/// 检查Docker服务状态
async fn check_docker_service_status(app: &mut CliApp) -> Result<bool> {
    let compose_path = client_core::constants::docker::get_compose_file_path();
    match docker_utils::check_compose_services_running(&compose_path).await {
        Ok(running) => Ok(running),
        Err(e) => {
            warn!("检查Docker服务状态失败，回退到简化检查: {}", e);
            // 回退到原来的简化实现
            match docker_service::check_docker_services_status(app).await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        }
    }
}

/// 检查docker目录是否存在且有文件需要备份
async fn check_docker_files_exist() -> Result<bool> {
    let docker_dir = Path::new("./docker");

    if !docker_dir.exists() {
        info!("docker目录不存在，无需备份");
        return Ok(false);
    }

    // 检查是否有重要文件需要备份
    let important_files = [
        docker::COMPOSE_FILE_NAME,
        "docker-compose.yaml",
        ".env",
        "data",
        "config",
        "logs",
    ];

    for file_name in important_files.iter() {
        let file_path = docker_dir.join(file_name);
        if file_path.exists() {
            info!("发现需要备份的文件: {}", file_path.display());
            return Ok(true);
        }
    }

    info!("docker目录存在但没有需要备份的重要文件");
    Ok(false)
}

/// 格式化时间间隔为可读字符串
fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();

    if seconds >= 86400 {
        format!("{} 天", seconds / 86400)
    } else if seconds >= 3600 {
        format!("{} 小时", seconds / 3600)
    } else if seconds >= 60 {
        format!("{} 分钟", seconds / 60)
    } else {
        format!("{} 秒", seconds)
    }
}
