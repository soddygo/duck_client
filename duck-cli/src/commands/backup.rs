use crate::app::CliApp;
use crate::docker_service::{DockerService, ServiceStatus};
use client_core::{
    backup::{BackupOptions, RestoreOptions},
    database::BackupType,
    error::Result,
};
use std::path::PathBuf;
use tracing::{error, info, warn};

/// 创建备份
pub async fn run_backup(app: &CliApp) -> Result<()> {
    info!("💾 创建数据备份");
    info!("===============");

    // 1. 检查docker-compose.yml文件是否存在
    let compose_path = std::path::Path::new(&app.config.docker.compose_file);
    if !compose_path.exists() {
        error!("❌ Docker Compose文件不存在: {}", compose_path.display());
        info!("💡 请先确保Docker服务已正确部署");
        return Ok(());
    }

    // 2. 检查Docker服务是否已停止
    info!("检查Docker服务状态...");
    info!("🔍 检查Docker服务状态...");

    let docker_service = DockerService::new(app.config.clone(), app.docker_manager.clone())?;
    match docker_service.get_service_status().await {
        Ok(status) => {
            if status.overall_status != ServiceStatus::AllStopped {
                warn!("⚠️  Docker服务仍在运行中！");
                error!("❌ 冷备份要求所有Docker服务必须处于停止状态");
                info!("📝 运行中的容器:");

                for container in status.containers.iter() {
                    if container.status.is_healthy() {
                        info!(
                            "   - {} (状态: {})",
                            container.name,
                            container.status.display_name()
                        );
                    }
                }

                info!("💡 请先停止所有Docker服务:");
                info!("   duck-cli docker-service stop");
                info!("   或者");
                info!(
                    "   cd {} && docker-compose down",
                    compose_path
                        .parent()
                        .unwrap_or(std::path::Path::new("."))
                        .display()
                );

                return Ok(());
            }

            info!("✅ 所有Docker服务已停止，可以进行备份");
        }
        Err(e) => {
            warn!("⚠️  无法确定Docker服务状态: {}", e);
            warn!("❓ 是否继续备份？这可能导致数据不一致");
            info!("💡 建议手动确认所有容器已停止:");
            info!("   docker ps");

            // 简单的用户确认
            info!("输入 'yes' 继续备份，其他任意键取消: ");
            use std::io::{self, Write};
            print!("输入 'yes' 继续备份，其他任意键取消: ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() != "yes" {
                warn!("❌ 用户取消备份操作");
                return Ok(());
            }
        }
    }

    // 3. 检查关键目录是否存在
    let docker_dir = std::path::Path::new("./docker");
    let data_dir = docker_dir.join("data");
    let app_dir = docker_dir.join("app");

    if !docker_dir.exists() {
        error!("❌ Docker目录不存在: {}", docker_dir.display());
        info!("💡 请先确保Docker服务已正确部署");
        return Ok(());
    }

    // 检查数据目录
    let mut has_data = false;
    if data_dir.exists() && data_dir.is_dir() {
        match std::fs::read_dir(&data_dir) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    info!("✅ 发现数据目录: {} (有内容)", data_dir.display());
                    has_data = true;
                } else {
                    warn!("⚠️  数据目录为空: {}", data_dir.display());
                }
            }
            Err(e) => {
                error!("❌ 无法读取数据目录: {}", e);
                return Err(e.into());
            }
        }
    } else {
        warn!("⚠️  数据目录不存在: {}", data_dir.display());
    }

    // 检查应用目录
    let mut has_app = false;
    if app_dir.exists() && app_dir.is_dir() {
        match std::fs::read_dir(&app_dir) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    info!("✅ 发现应用目录: {} (有内容)", app_dir.display());
                    has_app = true;
                } else {
                    warn!("⚠️  应用目录为空: {}", app_dir.display());
                }
            }
            Err(e) => {
                error!("❌ 无法读取应用目录: {}", e);
                return Err(e.into());
            }
        }
    } else {
        warn!("⚠️  应用目录不存在: {}", app_dir.display());
    }

    // 如果两个目录都不存在或都为空，警告用户
    if !has_data && !has_app {
        warn!("⚠️  重要目录都不存在或为空!");
        info!("💡 备份将只包含Docker配置文件");
        info!("   请确认这是您想要的操作");

        print!("是否继续创建备份? (y/N): ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() != "y" {
            warn!("❌ 用户取消备份操作");
            return Ok(());
        }
    }

    // 4. 创建备份
    info!("开始创建备份...");
    info!("📦 开始创建备份...");
    info!("   备份策略: 精确备份关键数据目录");
    info!("   备份内容:");
    if has_data {
        info!("     ✅ 数据目录: {} (容器持久化数据)", data_dir.display());
    }
    if has_app {
        info!(
            "     ✅ 应用目录: {} (Java工程和前端资源)",
            app_dir.display()
        );
    }
    info!("   备份目录: {}", app.config.get_backup_dir().display());

    // 准备备份选项 - 只备份关键的数据目录
    let mut source_dirs = Vec::new();
    if has_data {
        source_dirs.push(data_dir.to_path_buf());
    }
    if has_app {
        source_dirs.push(app_dir.to_path_buf());
    }

    if source_dirs.is_empty() {
        error!("❌ 没有找到需要备份的数据目录");
        info!("💡 请确保以下目录至少有一个存在且包含数据:");
        info!("   - {}", data_dir.display());
        info!("   - {}", app_dir.display());
        return Ok(());
    }

    let backup_options = BackupOptions {
        backup_type: BackupType::Manual,
        service_version: app.config.versions.docker_service.clone(),
        source_dirs,
        compression_level: 6, // 中等压缩级别
    };

    match app.backup_manager.create_backup(backup_options).await {
        Ok(backup_record) => {
            info!("🎉 备份创建成功！");
            info!("   备份ID: {}", backup_record.id);
            info!("   备份文件: {}", backup_record.file_path);
            info!("   备份时间: {}", backup_record.created_at);
            info!("   服务版本: {}", backup_record.service_version);

            // 显示备份文件大小
            if let Ok(metadata) = std::fs::metadata(&backup_record.file_path) {
                let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
                info!("   文件大小: {:.2} MB", size_mb);
            }

            info!("📋 备份内容说明:");
            info!("   此备份包含关键的数据目录:");
            if has_data {
                info!("   ✅ data/ - 数据库和容器持久化数据");
            }
            if has_app {
                info!("   ✅ app/ - Java应用和前端资源");
            }
            info!("   💡 备份文件只包含数据，不包含配置文件");
            info!("💡 备份完成，现在可以安全地启动Docker服务:");
            info!("   duck-cli docker-service start");
        }
        Err(e) => {
            error!("❌ 备份创建失败: {}", e);
            info!("💡 请检查:");
            info!("   - 备份目录是否有写入权限");
            info!("   - 磁盘空间是否充足");
            info!("   - 数据目录是否可读");
            return Err(e);
        }
    }

    Ok(())
}

/// 列出备份
pub async fn run_list_backups(app: &CliApp) -> Result<()> {
    let backups = app.backup_manager.list_backups().await?;

    if backups.is_empty() {
        info!("📦 暂无备份记录");
        info!("💡 使用以下命令创建备份:");
        info!("   duck-cli backup");
        return Ok(());
    }

    info!("📦 备份列表");
    info!("============");

    // 统计信息
    let total_backups = backups.len();
    let mut valid_backups = 0;
    let mut invalid_backups = 0;
    let mut total_size = 0u64;

    // 详细信息表头
    info!(
        "{:<4} {:<12} {:<20} {:<10} {:<8} {:<12} {}",
        "ID", "类型", "创建时间", "版本", "状态", "大小", "文件路径"
    );
    info!("{}", "-".repeat(100));

    for backup in &backups {
        let backup_path = std::path::Path::new(&backup.file_path);
        let file_exists = backup_path.exists();

        // 文件状态和大小信息
        let (status_display, size_display) = if file_exists {
            valid_backups += 1;

            // 获取文件大小
            let size = if let Ok(metadata) = std::fs::metadata(&backup.file_path) {
                let file_size = metadata.len();
                total_size += file_size;
                if file_size > 1024 * 1024 * 1024 {
                    format!("{:.1}GB", file_size as f64 / (1024.0 * 1024.0 * 1024.0))
                } else if file_size > 1024 * 1024 {
                    format!("{:.1}MB", file_size as f64 / (1024.0 * 1024.0))
                } else if file_size > 1024 {
                    format!("{:.1}KB", file_size as f64 / 1024.0)
                } else {
                    format!("{file_size}B")
                }
            } else {
                "未知".to_string()
            };

            ("✅ 可用", size)
        } else {
            invalid_backups += 1;
            ("❌ 文件缺失", "---".to_string())
        };

        // 备份类型显示
        let backup_type_display = match backup.backup_type {
            client_core::database::BackupType::Manual => "手动",
            client_core::database::BackupType::PreUpgrade => "升级前",
        };

        // 获取文件名而不是完整路径用于显示
        let filename = backup_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| backup.file_path.clone());

        info!(
            "{:<4} {:<12} {:<20} {:<10} {:<8} {:<12} {}",
            backup.id,
            backup_type_display,
            backup.created_at.format("%Y-%m-%d %H:%M:%S"),
            backup.service_version,
            status_display,
            size_display,
            filename
        );

        // 如果文件不存在，显示警告信息
        if !file_exists {
            warn!("     ⚠️  警告: 备份文件不存在，无法用于回滚！");
            warn!("         预期路径: {}", backup.file_path);
        }
    }

    info!("{}", "-".repeat(100));

    // 统计摘要
    info!("📊 备份统计:");
    info!("   总备份数: {}", total_backups);
    info!("   可用备份: {} ✅", valid_backups);
    if invalid_backups > 0 {
        warn!("   无效备份: {} ❌", invalid_backups);
    }

    if total_size > 0 {
        let total_size_display = if total_size > 1024 * 1024 * 1024 {
            format!("{:.2} GB", total_size as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if total_size > 1024 * 1024 {
            format!("{:.2} MB", total_size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} KB", total_size as f64 / 1024.0)
        };
        info!("   总大小: {}", total_size_display);
    }

    // 操作提示
    if valid_backups > 0 {
        info!("💡 可用操作:");
        info!("   - 从备份恢复: duck-cli rollback <备份ID>");
        info!("   - 创建新备份: duck-cli backup");
    }

    if invalid_backups > 0 {
        warn!("⚠️  发现 {} 个无效备份（文件缺失）", invalid_backups);
        info!("💡 建议:");
        info!(
            "   - 检查备份目录设置: {}",
            app.config.get_backup_dir().display()
        );
        info!("   - 如果备份文件被误删，这些记录将无法用于恢复");
        info!("   - 可考虑手动清理这些无效记录");
    }

    Ok(())
}

/// 从备份恢复
pub async fn run_rollback(app: &CliApp, backup_id: i64, force: bool) -> Result<()> {
    if !force {
        warn!("⚠️  警告: 此操作将覆盖当前所有服务文件和数据!");
        print!("请确认您要从备份 {backup_id} 恢复 (y/N): ");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() != "y" {
            warn!("操作已取消");
            return Ok(());
        }
    }

    info!("开始回滚操作...");

    let options = RestoreOptions {
        target_dir: PathBuf::from("./docker"),
        force_overwrite: true,
    };
    app.backup_manager
        .restore_from_backup(backup_id, options)
        .await?;
    info!("✅ 回滚完成");
    Ok(())
}
