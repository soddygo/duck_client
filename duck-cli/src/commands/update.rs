use crate::app::CliApp;
use client_core::error::Result;
use tracing::{error, info, warn};

/// 下载Docker服务升级文件
pub async fn run_upgrade(app: &mut CliApp, full: bool, force: bool) -> Result<()> {
    info!("📦 下载Docker服务文件");
    info!("=====================");

    // 检查是否是首次使用（docker目录为空或不存在docker-compose.yml）
    let docker_compose_path = std::path::Path::new(&app.config.docker.compose_file);
    let is_first_time = !docker_compose_path.exists();

    if is_first_time {
        info!("🆕 检测到这是您的首次部署");
        info!("   将下载完整的Docker服务包");
    } else if force {
        info!("🔧 强制重新下载模式");
    }

    // 获取版本信息以确定下载路径
    info!("检查Docker服务版本...");
    let current_version = app.config.versions.docker_service.clone();

    // 使用API客户端检查版本（移除自动注册逻辑，因为现在由AuthenticatedClient处理）
    let version_result = app.api_client.check_docker_version(&current_version).await;
    let version_info = version_result;

    match version_info {
        Ok(version_info) => {
            info!("=== Docker服务版本信息 ===");
            info!("当前版本: {}", version_info.current_version);
            info!("最新版本: {}", version_info.latest_version);

            // 构建基于版本的下载路径
            let target_version = &version_info.latest_version;
            let download_type = "full"; // 暂时只支持全量下载
            let download_path = app.config.get_version_download_file_path(
                target_version,
                download_type,
                client_core::constants::upgrade::DOCKER_SERVICE_PACKAGE,
            );

            // 检查文件是否已存在（智能下载会处理这个检查）

            info!("📂 下载路径结构:");
            info!("   版本目录: ./cacheDuckData/download/{}/", target_version);
            info!("   文件路径: {}", download_path.display());

            // 在强制模式下，直接下载（跳过优化检查）
            if force {
                info!("🔧 强制重新下载模式 - 跳过文件检查");

                // 确保下载目录存在
                if let Err(e) = app
                    .config
                    .ensure_version_download_dir(target_version, download_type)
                {
                    error!("❌ 创建下载目录失败: {}", e);
                    return Err(e);
                }

                info!("📥 开始强制下载服务包...");
                info!("   目标版本: {}", target_version);
                info!("   下载类型: {} (全量)", download_type);

                // 强制模式使用传统下载方法，跳过优化检查
                let download_result = app.api_client.download_service_update(&download_path).await;

                match download_result {
                    Ok(_) => {
                        info!("✅ 强制下载完成!");
                        info!("   文件位置: {}", download_path.display());
                        info!("📝 下一步操作:");
                        info!("   运行 'duck-cli docker-service deploy' 来部署服务");
                        return Ok(());
                    }
                    Err(e) => {
                        error!("❌ 强制下载失败: {}", e);
                        return Err(e);
                    }
                }
            }

            // 准备下载（智能检查模式）

            // 确保下载目录存在
            if let Err(e) = app
                .config
                .ensure_version_download_dir(target_version, download_type)
            {
                error!("❌ 创建下载目录失败: {}", e);
                return Err(e);
            }

            // 使用优化的下载方法（包含哈希验证和重复下载避免）
            info!("📥 智能下载检查...");
            info!("   目标版本: {}", target_version);
            info!("   下载类型: {} (全量)", download_type);

            if is_first_time {
                info!("状态: 🆕 首次部署 - 下载完整服务包");
            } else if version_info.has_update {
                info!("状态: 🎉 发现新版本，开始下载");
                if let Some(notes) = version_info.release_notes {
                    info!("更新说明:");
                    for line in notes.lines() {
                        info!("   {}", line);
                    }
                }
            } else if full {
                info!("状态: 📦 全量下载模式");
            } else {
                info!("状态: 🔍 检查文件完整性");
            }

            let download_result = app
                .api_client
                .download_service_update_optimized(&download_path, Some(target_version))
                .await;

            match download_result {
                Ok(_) => {
                    info!("✅ 服务包已准备就绪!");
                    info!("   文件位置: {}", download_path.display());
                    info!("📝 下一步操作:");
                    info!("   运行 'duck-cli docker-service deploy' 来部署服务");
                }
                Err(client_core::error::DuckError::Api(ref msg))
                    if msg.contains("401") || msg.contains("Unauthorized") =>
                {
                    error!("❌ 操作失败: 认证失败");
                    info!("💡 认证问题已由AuthenticatedClient自动处理，但仍然失败");
                    return Err(client_core::error::DuckError::Api(
                        "操作失败: 认证失败".to_string(),
                    ));
                }
                Err(e) => {
                    error!("❌ 操作失败: {}", e);
                    info!("💡 请检查网络连接或稍后重试");
                    return Err(e);
                }
            }
        }
        Err(e) => {
            warn!("⚠️  检查版本失败: {}", e);

            // 无法获取版本信息时，使用当前配置的版本构建路径
            let fallback_version = &current_version;
            let download_type = "full";
            let download_path = app.config.get_version_download_file_path(
                fallback_version,
                download_type,
                client_core::constants::upgrade::DOCKER_SERVICE_PACKAGE,
            );
            let file_exists = download_path.exists();

            if is_first_time {
                // 首次部署时，检查本地是否已有安装包文件
                if file_exists {
                    info!("✅ 发现本地安装包文件，跳过下载步骤");
                    info!("   文件位置: {}", download_path.display());
                    info!("📝 下一步操作:");
                    info!("   运行 'duck-cli docker-service deploy' 来部署服务");
                    info!("💡 提示: 虽然无法连接更新服务器，但可以使用本地文件继续部署");
                } else {
                    error!("❌ 首次部署时无法获取版本信息，且本地没有安装包文件");
                    info!("💡 首次部署建议:");
                    info!("   由于无法连接到更新服务器，您可以：");
                    info!("   1. 检查网络连接");
                    info!("   2. 联系管理员确认服务器状态");
                    info!("   3. 如有离线安装包，请手动放置到:");
                    info!("      {}", download_path.display());
                    info!("      然后运行 'duck-cli docker-service deploy' 部署服务");

                    // 首次部署时，如果无法获取版本信息且没有本地文件，才返回错误
                    return Err(client_core::DuckError::Custom(format!(
                        "首次部署时无法获取版本信息且本地没有安装包文件: {e}"
                    )));
                }
            } else {
                info!("💡 无法检查版本，可能的原因:");
                info!("   - 网络连接问题");
                info!("   - 服务器暂时不可用");
                info!("   - 服务器尚未配置版本信息");
                info!("📝 当前可用操作:");
                info!("   - 运行 'duck-cli status' 查看当前状态");
                info!("   - 运行 'duck-cli upgrade --force' 强制下载");
                if file_exists {
                    info!("   - 运行 'duck-cli docker-service deploy' 使用现有文件部署");
                    info!("   已存在的文件: {}", download_path.display());
                } else {
                    // 非首次部署但没有现有文件，也应该返回错误
                    warn!("⚠️  无法获取版本信息且本地没有现有的服务包文件");
                    return Err(client_core::DuckError::Custom(format!(
                        "无法获取版本信息且本地没有服务包文件: {e}"
                    )));
                }
            }
        }
    }

    Ok(())
}
