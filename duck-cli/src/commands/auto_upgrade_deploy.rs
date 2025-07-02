use crate::app::CliApp;
use crate::cli::AutoUpgradeDeployCommand;
use crate::commands::{backup, docker_service, update};
use crate::docker_utils;
use client_core::constants::{docker, timeout};
use client_core::error::Result;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// 运行自动升级部署相关命令的统一入口
pub async fn handle_auto_upgrade_deploy_command(app: &mut CliApp, cmd: AutoUpgradeDeployCommand) -> Result<()> {
    match cmd {
        AutoUpgradeDeployCommand::Run { port } => {
            info!("🚀 开始自动升级部署流程...");
            run_auto_upgrade_deploy(app, port).await
        }
        AutoUpgradeDeployCommand::DelayTimeDeploy { time, unit } => {
            info!("配置延迟自动升级部署: {} {}", time, unit);
            schedule_delayed_deploy(app, time, &unit).await
        }
        AutoUpgradeDeployCommand::Status => {
            info!("显示自动升级部署状态");
            show_status(app).await
        }
    }
}

/// 执行自动升级部署流程
pub async fn run_auto_upgrade_deploy(app: &mut CliApp, frontend_port: Option<u16>) -> Result<()> {
    info!("🚀 开始自动升级部署流程...");

    // 如果指定了端口，显示端口信息
    if let Some(port) = frontend_port {
        info!("🔌 自定义frontend端口: {}", port);
    }

    // 1. 获取最新版本信息并下载
    info!("开始下载最新的Docker服务版本");
    info!("📥 正在下载最新的Docker服务版本...");

    // 获取最新版本信息
    let latest_version = match app
        .api_client
        .check_docker_version(&app.config.versions.docker_service)
        .await
    {
        Ok(version_info) => {
            info!(
                "📋 版本信息: {} -> {}",
                version_info.current_version, version_info.latest_version
            );
            version_info.latest_version
        }
        Err(e) => {
            warn!("⚠️ 获取版本信息失败，使用配置版本: {}", e);
            app.config.versions.docker_service.clone()
        }
    };

    update::run_upgrade(app, true, false).await?; // 全量下载

    // 1.5. 解压下载的docker.zip文件
    info!("📦 正在解压Docker服务包...");

    // 🔍 检测部署类型：第一次部署 vs 升级部署
    let is_first_deployment = is_first_deployment().await;
    if is_first_deployment {
        info!("🆕 检测到第一次部署，跳过数据备份流程");
    } else {
        info!("🔄 检测到升级部署，需要保护现有数据");
    }

    // 🛡️ 数据保护：只在升级部署时备份现有的数据目录
    let temp_data_backup = if is_first_deployment {
        None
    } else {
        backup_data_before_cleanup().await?
    };

    // 清理现有的docker目录以避免路径冲突
    let docker_dir = std::path::Path::new("docker");
    if docker_dir.exists() {
        info!("🧹 清理现有docker目录以避免文件冲突...");
        match std::fs::remove_dir_all(docker_dir) {
            Ok(_) => info!("✅ docker目录清理完成"),
            Err(e) => {
                warn!("⚠️ 清理docker目录失败: {}, 尝试继续解压", e);
                // 清理失败时，恢复备份的数据（仅在升级部署时）
                if !is_first_deployment {
                    restore_data_after_cleanup(&temp_data_backup).await?;
                }
                return Err(client_core::error::DuckError::custom(format!(
                    "清理docker目录失败: {e}"
                )));
            }
        }
    }

    // 解压新的Docker服务包（使用最新版本）
    match docker_service::extract_docker_service(app, None, Some(latest_version.clone())).await {
        Ok(_) => {
            info!("✅ Docker服务包解压完成");

            // 🛡️ 数据恢复：仅在升级部署时恢复备份的数据目录
            if !is_first_deployment {
                restore_data_after_cleanup(&temp_data_backup).await?;
            } else {
                info!("🆕 第一次部署，无需数据恢复");
            }

            // 📝 更新配置文件中的Docker服务版本
            if latest_version != app.config.versions.docker_service {
                info!(
                    "📝 更新Docker服务版本: {} -> {}",
                    app.config.versions.docker_service, latest_version
                );

                // 更新内存中的版本信息
                app.config.versions.docker_service = latest_version.clone();

                // 持久化到配置文件
                match app.config.save_to_file("config.toml") {
                    Ok(_) => {
                        info!("✅ 配置文件版本号已更新并保存");
                    }
                    Err(e) => {
                        warn!("⚠️ 保存配置文件失败: {}", e);
                        warn!("   版本号已在内存中更新，但配置文件未同步");
                    }
                }
            } else {
                info!("📝 版本号无需更新 (已是最新版本: {})", latest_version);
            }
        }
        Err(e) => {
            error!("❌ Docker服务包解压失败: {}", e);
            // 解压失败时，恢复备份的数据（仅在升级部署时）
            if !is_first_deployment {
                restore_data_after_cleanup(&temp_data_backup).await?;
            }
            return Err(e);
        }
    }

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
    docker_service::deploy_docker_services(app, frontend_port).await?;

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
                "不支持的时间单位: {unit}，支持的单位: hours, minutes, days"
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
    match run_auto_upgrade_deploy(app, None).await {
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
        format!("{seconds} 秒")
    }
}

/// 检测是否为第一次部署
async fn is_first_deployment() -> bool {
    let docker_dir = std::path::Path::new("docker");
    let docker_data_dir = docker_dir.join("data");

    // 如果docker目录不存在，肯定是第一次部署
    if !docker_dir.exists() {
        return true;
    }

    // 如果docker/data目录不存在，也是第一次部署
    if !docker_data_dir.exists() {
        return true;
    }

    // 检查data目录是否有实际的数据内容
    match std::fs::read_dir(&docker_data_dir) {
        Ok(entries) => {
            let mut has_meaningful_data = false;

            for entry in entries.flatten() {
                let path = entry.path();

                // 检查是否有重要的数据目录（mysql, redis, milvus等）
                if path.is_dir() {
                    let dir_name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("");

                    match dir_name {
                        "mysql" | "redis" | "milvus" | "postgres" | "mongodb" => {
                            // 检查这些目录是否有实际内容
                            if let Ok(sub_entries) = std::fs::read_dir(&path) {
                                if sub_entries.count() > 0 {
                                    has_meaningful_data = true;
                                    break;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            !has_meaningful_data
        }
        Err(_) => true, // 读取失败，当作第一次部署
    }
}

/// 在清理docker目录前备份数据目录
async fn backup_data_before_cleanup() -> Result<Option<std::path::PathBuf>> {
    let docker_data_dir = Path::new("docker/data");

    if !docker_data_dir.exists() {
        info!("📁 无现有数据目录需要备份");
        return Ok(None);
    }

    // 创建临时备份目录
    let temp_dir = std::env::temp_dir();
    let backup_name = format!("duck_data_backup_{}", chrono::Utc::now().timestamp());
    let temp_backup_path = temp_dir.join(backup_name);

    info!(
        "🛡️ 正在备份数据目录到临时位置: {}",
        temp_backup_path.display()
    );

    // 递归复制数据目录到临时位置
    match copy_dir_recursively(docker_data_dir, &temp_backup_path) {
        Ok(_) => {
            info!("✅ 数据目录备份完成");
            Ok(Some(temp_backup_path))
        }
        Err(e) => {
            warn!("⚠️ 数据目录备份失败: {}", e);
            // 备份失败时，返回None表示没有备份
            Ok(None)
        }
    }
}

/// 解压完成后恢复备份的数据目录
async fn restore_data_after_cleanup(temp_backup_path: &Option<std::path::PathBuf>) -> Result<()> {
    if let Some(backup_path) = temp_backup_path {
        if backup_path.exists() {
            let docker_data_dir = Path::new("docker/data");

            info!("🔄 正在恢复数据目录从: {}", backup_path.display());

            // 确保目标目录存在
            if let Some(parent) = docker_data_dir.parent() {
                fs::create_dir_all(parent)?;
            }

            // 如果新解压的包中有data目录，先删除它
            if docker_data_dir.exists() {
                fs::remove_dir_all(docker_data_dir)?;
            }

            // 从临时备份恢复数据目录
            match copy_dir_recursively(backup_path, docker_data_dir) {
                Ok(_) => {
                    info!("✅ 数据目录恢复完成");

                    // 设置正确的权限（特别是MySQL目录需要777权限）
                    let mysql_data_dir = docker_data_dir.join("mysql");
                    if mysql_data_dir.exists() {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let permissions = fs::Permissions::from_mode(0o777);
                            fs::set_permissions(&mysql_data_dir, permissions)?;
                            info!("🔒 已设置MySQL数据目录权限为777");
                        }
                    }
                }
                Err(e) => {
                    error!("❌ 数据目录恢复失败: {}", e);
                    return Err(client_core::error::DuckError::custom(format!(
                        "数据目录恢复失败: {e}"
                    )));
                }
            }

            // 清理临时备份
            if let Err(e) = fs::remove_dir_all(backup_path) {
                warn!("⚠️ 清理临时备份失败: {}", e);
            } else {
                info!("🧹 临时备份已清理");
            }
        }
    } else {
        info!("📁 无备份数据需要恢复");
    }

    Ok(())
}

/// 递归复制目录
fn copy_dir_recursively(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.exists() {
        return Ok(());
    }

    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursively(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
