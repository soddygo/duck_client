use crate::app::CliApp;
use crate::docker_service::{ContainerStatus, DockerService};
use client_core::Result;
use tracing::{error, info, warn};

/// 部署 Docker 服务
pub async fn deploy_docker_services(app: &CliApp) -> Result<()> {
    info!("🚀 开始部署 Docker 服务...");

    // 创建 Docker 服务管理器
    let docker_service_manager =
        DockerService::new(app.config.clone(), app.docker_manager.clone())?;

    // 显示系统信息
    let arch = docker_service_manager.get_architecture();
    info!("检测到系统架构: {}", arch.display_name());
    info!(
        "工作目录: {}",
        docker_service_manager.get_work_dir().display()
    );

    // 执行完整的部署流程
    match docker_service_manager.deploy_services().await {
        Ok(_) => {
            info!("✅ Docker 服务部署成功!");

            // 显示服务状态
            if let Ok(report) = docker_service_manager.health_check().await {
                info!("📊 服务状态概览:");
                info!("  • 整体状态: {}", report.overall_status.display_name());
                info!(
                    "  • 运行中容器: {}/{}",
                    report.running_count, report.total_count
                );

                if !report.containers.is_empty() {
                    info!("  • 容器详情:");
                    for container in &report.containers {
                        info!(
                            "    - {} ({}) - {}",
                            container.name,
                            container.image,
                            container.status.display_name()
                        );
                    }
                }
            }
        }
        Err(e) => {
            error!("❌ Docker 服务部署失败: {:?}", e);
            return Err(client_core::DuckError::custom(format!(
                "Docker 服务部署失败: {e:?}"
            )));
        }
    }

    Ok(())
}

/// 启动 Docker 服务
pub async fn start_docker_services(app: &CliApp) -> Result<()> {
    info!("▶️ 启动 Docker 服务...");

    let docker_service_manager =
        DockerService::new(app.config.clone(), app.docker_manager.clone())?;

    match docker_service_manager.start_services().await {
        Ok(_) => {
            info!("✅ Docker 服务启动成功!");
        }
        Err(e) => {
            error!("❌ Docker 服务启动失败: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// 停止 Docker 服务
pub async fn stop_docker_services(app: &CliApp) -> Result<()> {
    info!("⏹️ 停止 Docker 服务...");

    let docker_service_manager =
        DockerService::new(app.config.clone(), app.docker_manager.clone())?;

    match docker_service_manager.stop_services().await {
        Ok(_) => {
            info!("✅ Docker 服务已停止");
        }
        Err(e) => {
            error!("❌ Docker 服务停止失败: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// 重启 Docker 服务
pub async fn restart_docker_services(app: &CliApp) -> Result<()> {
    info!("🔄 重启 Docker 服务...");

    let docker_service_manager =
        DockerService::new(app.config.clone(), app.docker_manager.clone())?;

    match docker_service_manager.restart_services().await {
        Ok(_) => {
            info!("✅ Docker 服务重启成功!");
        }
        Err(e) => {
            error!("❌ Docker 服务重启失败: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// 重启单个容器
pub async fn restart_container(app: &CliApp, container_name: &str) -> Result<()> {
    info!("🔄 重启容器: {}", container_name);

    let docker_service_manager =
        DockerService::new(app.config.clone(), app.docker_manager.clone())?;

    match docker_service_manager
        .restart_container(container_name)
        .await
    {
        Ok(_) => {
            info!("✅ 容器 {} 重启成功!", container_name);
        }
        Err(e) => {
            error!("❌ 容器 {} 重启失败: {}", container_name, e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// 检查 Docker 服务状态
pub async fn check_docker_services_status(app: &CliApp) -> Result<()> {
    info!("📊 检查 Docker 服务状态...");

    let docker_service_manager =
        DockerService::new(app.config.clone(), app.docker_manager.clone())?;

    match docker_service_manager.get_service_status().await {
        Ok(report) => {
            info!("=== Docker 服务状态报告 ===");
            info!(
                "检查时间: {}",
                report.check_time.format("%Y-%m-%d %H:%M:%S UTC")
            );
            info!("整体状态: {}", report.overall_status.display_name());
            info!(
                "运行统计: {}/{} 个容器正在运行",
                report.running_count, report.total_count
            );

            if !report.containers.is_empty() {
                info!("容器详情:");
                for container in &report.containers {
                    let status_icon = match container.status {
                        ContainerStatus::Running => "🟢",
                        ContainerStatus::Stopped => "🔴",
                        ContainerStatus::Starting => "🟡",
                        ContainerStatus::Unhealthy => "🟠",
                        ContainerStatus::Unknown => "⚪",
                    };

                    info!(
                        "  {} {} ({})",
                        status_icon,
                        container.name,
                        container.status.display_name()
                    );
                    info!("     镜像: {}", container.image);

                    if !container.ports.is_empty() {
                        info!("     端口: {}", container.ports.join(", "));
                    }
                }
            }

            if !report.errors.is_empty() {
                warn!("⚠️ 错误信息:");
                for error in &report.errors {
                    warn!("  • {}", error);
                }
            }

            // 显示访问信息
            if report.overall_status.is_healthy() {
                info!("🌐 服务访问信息:");
                info!("  • 前端页面: http://localhost:80");
                info!("  • 后端API: http://localhost:8080");
                info!("  • 管理界面: http://localhost:9000 (如果配置)");
            }
        }
        Err(e) => {
            error!("❌ 获取服务状态失败: {:?}", e);
            return Err(client_core::DuckError::custom(format!(
                "获取服务状态失败: {e:?}"
            )));
        }
    }

    Ok(())
}

/// 加载 Docker 镜像
pub async fn load_docker_images(app: &CliApp) -> Result<()> {
    info!("📦 加载 Docker 镜像...");

    let docker_service_manager =
        DockerService::new(app.config.clone(), app.docker_manager.clone())?;

    // 显示架构信息
    let arch = docker_service_manager.get_architecture();
    info!("当前系统架构: {}", arch.display_name());

    match docker_service_manager.load_images().await {
        Ok(result) => {
            info!("📦 镜像加载完成!");
            info!("  • 成功加载: {} 个镜像", result.success_count());
            info!("  • 加载失败: {} 个镜像", result.failure_count());

            if !result.loaded_images.is_empty() {
                info!("✅ 成功加载的镜像:");
                for image in &result.loaded_images {
                    info!("  • {}", image);
                }
            }

            if !result.failed_images.is_empty() {
                warn!("❌ 加载失败的镜像:");
                for (image, error) in &result.failed_images {
                    warn!("  • {}: {}", image, error);
                }
            }
        }
        Err(e) => {
            error!("❌ 镜像加载失败: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// 设置镜像标签
pub async fn setup_image_tags(app: &CliApp) -> Result<()> {
    info!("🏷️ 设置镜像标签...");

    let docker_service_manager =
        DockerService::new(app.config.clone(), app.docker_manager.clone())?;

    match docker_service_manager.setup_image_tags().await {
        Ok(result) => {
            info!("🏷️ 镜像标签设置完成!");
            info!("  • 成功设置: {} 个标签", result.success_count());
            info!("  • 设置失败: {} 个标签", result.failure_count());

            if !result.tagged_images.is_empty() {
                info!("✅ 成功设置的标签:");
                for (original, target) in &result.tagged_images {
                    info!("  • {} → {}", original, target);
                }
            }

            if !result.failed_tags.is_empty() {
                warn!("❌ 设置失败的标签:");
                for (original, target, error) in &result.failed_tags {
                    warn!("  • {} → {}: {}", original, target, error);
                }
            }
        }
        Err(e) => {
            error!("❌ 镜像标签设置失败: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// 获取系统架构信息
pub async fn show_architecture_info(_app: &CliApp) -> Result<()> {
    let arch = crate::docker_service::get_system_architecture();

    info!("🔧 系统架构信息:");
    info!("  • 架构类型: {}", arch.display_name());
    info!("  • 架构标识: {}", arch.as_str());
    info!(
        "  • 镜像后缀: {}",
        crate::docker_service::get_architecture_suffix(arch)
    );

    Ok(())
}
