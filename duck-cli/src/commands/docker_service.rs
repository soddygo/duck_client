use crate::app::CliApp;
use crate::docker_service::{ContainerStatus, DockerService};
use client_core::Result;
use tracing::{error, info, warn};

/// 部署 Docker 服务
pub async fn deploy_docker_services(app: &CliApp, frontend_port: Option<u16>) -> Result<()> {
    info!("🚀 开始部署 Docker 服务...");

    // 如果指定了端口，先设置端口配置
    if let Some(port) = frontend_port {
        info!("🔧 配置frontend端口: {}", port);
        set_frontend_port(port).await?;
    }

    // 创建 Docker 服务管理器
    let mut docker_service_manager =
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

    let mut docker_service_manager =
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

    let mut docker_service_manager =
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
                use client_core::constants::docker::ports;
                info!("🌐 服务访问信息:");
                info!("  • 前端页面: http://localhost:{}", ports::DEFAULT_FRONTEND_PORT);
                info!("  • 后端API: http://localhost:{}", ports::DEFAULT_BACKEND_PORT);
                info!("  • 管理界面: http://localhost:{} (如果配置)", ports::DEFAULT_MINIO_API_PORT);
                info!("  📝 注意: 如果使用了自定义端口参数，请使用相应的端口访问");
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

    // 先加载镜像以获取实际的镜像映射
    info!("📦 检查已加载的镜像...");
    let load_result = docker_service_manager.load_images().await?;

    if load_result.image_mappings.is_empty() {
        warn!("⚠️ 未找到已加载的镜像映射，请先运行 load-images 命令");
        return Ok(());
    }

    // 使用基于映射的新方法
    match docker_service_manager
        .setup_image_tags_with_mappings(&load_result.image_mappings)
        .await
    {
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

/// 解压Docker服务包
pub async fn extract_docker_service(
    app: &CliApp,
    file: Option<String>,
    version: Option<String>,
) -> Result<()> {
    info!("📦 开始解压Docker服务包...");

    // 确定要解压的文件路径
    let zip_path = if let Some(file_path) = file {
        // 使用用户指定的文件路径
        std::path::PathBuf::from(file_path)
    } else {
        // 使用默认路径（基于版本）
        let target_version = version
            .as_deref()
            .unwrap_or(&app.config.versions.docker_service);

        app.config.get_version_download_file_path(
            target_version,
            "full",
            client_core::constants::upgrade::DOCKER_SERVICE_PACKAGE,
        )
    };

    // 检查文件是否存在
    if !zip_path.exists() {
        error!("❌ Docker服务包文件不存在: {}", zip_path.display());
        return Err(client_core::DuckError::Custom(format!(
            "Docker服务包文件不存在: {}",
            zip_path.display()
        )));
    }

    info!("📦 找到Docker服务包: {}", zip_path.display());

    // 使用utils中的解压函数
    crate::utils::extract_docker_service(&zip_path).await?;

    info!("✅ Docker服务包解压完成");
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

/// 使用 ducker 列出 Docker 镜像
pub async fn list_docker_images_with_ducker(app: &CliApp) -> Result<()> {
    info!("🔍 使用 ducker 列出 Docker 镜像...");

    let docker_service_manager =
        DockerService::new(app.config.clone(), app.docker_manager.clone())?;

    match docker_service_manager
        .list_docker_images_with_ducker()
        .await
    {
        Ok(images) => {
            if images.is_empty() {
                info!("📭 未找到任何 Docker 镜像");
            } else {
                info!("🎯 找到 {} 个 Docker 镜像:", images.len());
                for (index, image) in images.iter().enumerate() {
                    info!("  {}. {}", index + 1, image);
                }

                // 显示与我们业务相关的镜像
                let business_images: Vec<&String> = images
                    .iter()
                    .filter(|img| {
                        img.contains("registry.yichamao.com")
                            || img.contains("mysql")
                            || img.contains("redis")
                            || img.contains("milvus")
                            || img.contains("quickwit")
                    })
                    .collect();

                if !business_images.is_empty() {
                    info!("");
                    info!("🏢 业务相关镜像 ({} 个):", business_images.len());
                    for image in business_images {
                        let status = if image.contains(":latest") && !image.contains("latest-") {
                            "✅ 已准备"
                        } else if image.contains("latest-arm64") || image.contains("latest-amd64") {
                            "🔄 需要标签"
                        } else {
                            "ℹ️  其他版本"
                        };
                        info!("  • {} {}", status, image);
                    }
                }
            }
        }
        Err(e) => {
            error!("❌ 获取镜像列表失败: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// 设置frontend服务端口
async fn set_frontend_port(port: u16) -> Result<()> {
    use std::fs;
    use client_core::constants::docker::{get_env_file_path, env_vars};

    let env_file_path = get_env_file_path();
    
    if !env_file_path.exists() {
        warn!("⚠️  .env文件不存在: {}", env_file_path.display());
        return Err(client_core::DuckError::custom(
            ".env文件不存在，无法设置frontend端口"
        ));
    }

    // 读取现有的.env文件内容
    let content = fs::read_to_string(&env_file_path)
        .map_err(|e| client_core::DuckError::custom(format!("读取.env文件失败: {}", e)))?;

    // 处理内容，更新FRONTEND_HOST_PORT的值
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut found = false;
    let env_var_prefix = format!("{}=", env_vars::FRONTEND_HOST_PORT);

    for line in &mut lines {
        if line.starts_with(&env_var_prefix) {
            *line = format!("{}={}", env_vars::FRONTEND_HOST_PORT, port);
            found = true;
            info!("✅ 更新{}={}", env_vars::FRONTEND_HOST_PORT, port);
            break;
        }
    }

    // 如果没找到，添加新行
    if !found {
        lines.push(format!("{}={}", env_vars::FRONTEND_HOST_PORT, port));
        info!("✅ 添加{}={}", env_vars::FRONTEND_HOST_PORT, port);
    }

    // 写回文件
    let updated_content = lines.join("\n");
    fs::write(&env_file_path, updated_content)
        .map_err(|e| client_core::DuckError::custom(format!("写入.env文件失败: {}", e)))?;

    info!("🔧 Frontend端口已设置为: {}", port);
    Ok(())
}
