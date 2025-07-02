use crate::app::CliApp;
use crate::docker_utils;
use client_core::container::{DockerManager, ServiceStatus};
use client_core::error::Result;
use tracing::{error, info, warn};

/// 显示客户端版本信息（标题和基本信息）
pub fn show_client_version() {
    info!("🦆 Duck Client 状态");
    info!("==================");
    info!("📋 基本信息:");
    info!("   客户端版本: v{}", env!("CARGO_PKG_VERSION"));
}

/// 显示服务状态（完整版本，包含基本信息）
pub async fn run_status(app: &CliApp) -> Result<()> {
    show_client_version();
    run_status_details(app).await
}

/// 显示详细状态信息（不包含基本信息标题）
pub async fn run_status_details(app: &CliApp) -> Result<()> {
    // 继续显示其他基本信息
    info!("   Docker服务版本: {}", app.config.versions.docker_service);
    info!("   配置文件: {}", "config.toml");

    // 显示客户端UUID
    let client_uuid = app.database.get_or_create_client_uuid().await?;
    info!("   客户端UUID: {}", client_uuid);

    // 检查文件状态
    info!("📁 文件状态:");
    let docker_compose_path = std::path::Path::new(&app.config.docker.compose_file);

    // 使用新的版本化路径检查服务包文件
    let current_version = &app.config.versions.docker_service;
    let download_path = app.config.get_version_download_file_path(
        current_version,
        "full",
        client_core::constants::upgrade::DOCKER_SERVICE_PACKAGE,
    );

    if docker_compose_path.exists() {
        info!(
            "   ✅ Docker Compose文件: {}",
            app.config.docker.compose_file
        );
    } else {
        info!(
            "   ❌ Docker Compose文件: {} (不存在)",
            app.config.docker.compose_file
        );
    }

    if download_path.exists() {
        info!("   ✅ 服务包文件: {}", download_path.display());
    } else {
        info!("   ❌ 服务包文件: {} (不存在)", download_path.display());
    }

    // Docker服务状态
    info!("🐳 Docker服务状态:");
    if docker_compose_path.exists() {
        info!("   📋 Docker Compose文件已就绪");

        // 检查具体的服务状态
        match check_docker_services_status(docker_compose_path).await {
            Ok(()) => {
                // 状态检查成功，详细信息已在函数内部显示
            }
            Err(e) => {
                warn!("   ⚠️  服务状态检查失败: {}", e);
                info!("   💡 建议检查:");
                info!("      - Docker是否已安装并运行");
                info!("      - docker-compose是否可用");
                info!("      - 使用 'docker-compose ps' 手动查看状态");
            }
        }
    } else {
        warn!("   ❌ Docker Compose文件不存在，服务未初始化");
    }

    // 根据状态提供建议
    info!("💡 状态分析和建议:");

    if !docker_compose_path.exists() && !download_path.exists() {
        info!("   🆕 您似乎是首次使用");
        info!("   📝 建议执行以下步骤:");
        info!("      1. duck-cli upgrade                  (下载Docker服务包)");
        info!("      2. duck-cli docker-service deploy    (部署并启动服务)");
    } else if !docker_compose_path.exists() && download_path.exists() {
        info!("   📦 发现服务包文件，但尚未解压");
        info!("   📝 建议执行:");
        info!("      - duck-cli docker-service deploy  (完整部署流程)");
        info!("      - duck-cli docker-service start   (仅启动服务)");
    } else {
        info!("   ✅ 系统文件完整，可以正常使用所有功能");
        info!("   📝 可用命令:");
        info!("      - duck-cli docker-service start/stop/restart  (控制服务)");
        info!("      - duck-cli upgrade                            (升级服务)");
        info!("      - duck-cli backup                             (创建备份)");
        info!("      - duck-cli check-update                       (检查客户端更新)");
    }

    Ok(())
}

/// 显示API配置信息
pub async fn run_api_info(app: &CliApp) -> Result<()> {
    let api_config = app.api_client.get_config();
    info!("{}", api_config);
    Ok(())
}

/// 检查Docker服务状态的内部辅助函数
async fn check_docker_services_status(compose_file_path: &std::path::Path) -> Result<()> {
    // 首先尝试使用 docker_utils 快速检查服务运行状态
    match docker_utils::check_compose_services_running(compose_file_path).await {
        Ok(services_running) => {
            if services_running {
                info!("   ✅ 服务正在运行");

                // 尝试获取详细的服务状态信息
                if let Ok(docker_manager) = DockerManager::new(compose_file_path) {
                    match docker_manager.get_services_status().await {
                        Ok(services) => {
                            if !services.is_empty() {
                                info!("   📋 服务详情:");
                                let mut running_count = 0;
                                let total_count = services.len();

                                for service in &services {
                                    let status_icon = match service.status {
                                        ServiceStatus::Running => {
                                            running_count += 1;
                                            "🟢"
                                        }
                                        ServiceStatus::Stopped => "🔴",
                                        ServiceStatus::Unknown => "🟡",
                                    };

                                    info!(
                                        "      {} {} - {} ({})",
                                        status_icon,
                                        service.name,
                                        format!("{:?}", service.status).to_lowercase(),
                                        service.image
                                    );

                                    // 显示端口信息（如果有的话）
                                    if !service.ports.is_empty() {
                                        info!("         端口: {}", service.ports.join(", "));
                                    }
                                }

                                info!(
                                    "   📊 状态摘要: {}/{} 服务运行中",
                                    running_count, total_count
                                );

                                // 提供访问信息
                                if running_count > 0 {
                                    info!("   🌐 可能的访问地址:");
                                    use client_core::constants::docker::ports;
                                    info!(
                                        "      - 前端页面: http://localhost:{}",
                                        ports::DEFAULT_FRONTEND_PORT
                                    );
                                    info!(
                                        "      - 后端API: http://localhost:{}",
                                        ports::DEFAULT_BACKEND_PORT
                                    );
                                }
                            } else {
                                info!("   📋 没有检测到运行中的服务容器");
                            }
                        }
                        Err(e) => {
                            warn!("   ⚠️  无法获取服务详情: {}", e);
                            info!("   💡 但基本检查显示服务正在运行");
                        }
                    }
                } else {
                    warn!("   ⚠️  无法创建Docker管理器，但服务似乎在运行");
                }
            } else {
                warn!("   🔴 服务未运行");
                info!("   💡 启动建议:");
                info!("      - duck-cli docker-service start    (启动服务)");
                info!("      - duck-cli docker-service deploy   (重新部署并启动)");
            }
        }
        Err(e) => {
            error!("   ❌ 无法检查服务状态: {}", e);

            // 尝试进行基本的Docker环境检查
            if let Ok(docker_manager) = DockerManager::new(compose_file_path) {
                match docker_manager.check_docker_status().await {
                    Ok(_) => {
                        info!("   ✅ Docker环境正常");
                        info!("   💡 可能的原因:");
                        info!("      - 服务尚未启动");
                        info!("      - docker-compose配置有问题");
                    }
                    Err(docker_err) => {
                        error!("   ❌ Docker环境问题: {}", docker_err);
                        info!("   💡 请检查:");
                        info!("      - Docker是否已安装");
                        info!("      - Docker服务是否正在运行");
                    }
                }
            }

            return Err(e);
        }
    }

    Ok(())
}
