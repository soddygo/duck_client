use tauri::{command, AppHandle, Emitter, Manager};
use client_core::{
    config::AppConfig,
    database::Database,
    api::ApiClient,
    container::DockerManager,
    authenticated_client::AuthenticatedClient,
    constants,
};
use duck_cli::download_with_progress;
use super::types::{InitProgressEvent, InitCompletedEvent, DownloadProgressEvent, DownloadCompletedEvent, AppGlobalState};
use std::time::Instant;
use tracing::{warn, info, debug, error};

/// 检查初始化状态
#[command]
pub async fn check_initialization_status(app_handle: AppHandle) -> Result<bool, String> {
    use client_core::constants::config::{DATA_DIR_NAME, CONFIG_FILE_NAME};
    
    // 1. 获取用户设置的工作目录
    let state = app_handle.state::<AppGlobalState>();
    let working_dir = state.working_directory.read().await;
    
    if let Some(base_dir) = working_dir.as_ref() {
        let config_path = base_dir.join(DATA_DIR_NAME).join(CONFIG_FILE_NAME);
        Ok(config_path.exists())
    } else {
        Ok(false) // 如果没有设置工作目录，认为未初始化
    }
}

/// 快速初始化客户端（仅创建本地配置和数据库）
#[command]
pub async fn init_client_with_progress(app_handle: AppHandle) -> Result<String, String> {
    // 1. 获取用户设置的工作目录
    let state = app_handle.state::<AppGlobalState>();
    let working_dir = state.working_directory.read().await;
    
    let base_dir = working_dir.as_ref().ok_or("请先选择工作目录")?;
    
    // 2. 临时切换到用户选择的工作目录
    let original_dir = std::env::current_dir().map_err(|e| format!("获取当前目录失败: {}", e))?;
    std::env::set_current_dir(base_dir).map_err(|e| format!("切换到工作目录失败: {}", e))?;
    
    // 执行初始化（确保在结束时恢复目录）
    let result = async {
        // 3. 获取全局数据库管理器（✅ 使用单例！）
        let db_manager = state.get_or_init_db_manager(base_dir).await?;
        
        // 4. 记录用户操作开始
        let action_id = db_manager.record_user_action(
            "INITIALIZE",
            "用户初始化Duck Client",
            Some(format!(r#"{{"working_directory": "{}"}}"#, base_dir.display()))
        ).await.map_err(|e| format!("记录用户操作失败: {}", e))?;
        
        let start_time = Instant::now();
        
        // 5. 更新应用状态为初始化中
        db_manager.update_app_state(
            "INITIALIZING",
            Some(r#"{"stage": "setup", "message": "正在初始化本地环境"}"#.to_string()),
            Some(10),
            None
        ).await.map_err(|e| format!("更新应用状态失败: {}", e))?;
        
        // 6. 创建默认配置（参考duck-cli init逻辑）
        let config = AppConfig::default();
        
        // 7. 确保缓存目录存在（使用AppConfig的方法）
        config.ensure_cache_dirs().map_err(|e| format!("创建缓存目录失败: {}", e))?;
        
        // 8. 创建data目录和必要的子目录结构
        let data_dir = std::path::Path::new("data");
        let docker_dir = std::path::Path::new("docker");
        
        std::fs::create_dir_all(data_dir).map_err(|e| format!("创建data目录失败: {}", e))?;
        std::fs::create_dir_all(docker_dir).map_err(|e| format!("创建docker目录失败: {}", e))?;
        
        // 创建备份和缓存相关目录
        std::fs::create_dir_all("backup").map_err(|e| format!("创建备份目录失败: {}", e))?;
        std::fs::create_dir_all("cacheDuckData").map_err(|e| format!("创建缓存目录失败: {}", e))?;
        
        // 9. 保存配置文件
        let config_path = data_dir.join("config.toml");
        config.save_to_file(&config_path).map_err(|e| format!("保存配置文件失败: {}", e))?;
        
        // 10. 初始化数据库（使用传统方式，确保兼容性）
        let db_path = base_dir.join("data").join("duck_client.db"); // 使用标准数据库文件名
        let database = Database::connect(&db_path)
            .await
            .map_err(|e| format!("初始化数据库失败: {}", e))?;
        
        // 11. 创建认证客户端并注册（关键步骤！）
        let server_base_url = client_core::constants::api::DEFAULT_BASE_URL.to_string();
        let _authenticated_client = AuthenticatedClient::new(database.clone(), server_base_url)
            .await
            .map_err(|e| format!("客户端注册失败: {}", e))?;
        
        // 12. 更新应用状态为初始化完成
        db_manager.update_app_state(
            "INITIALIZED",
            Some(r#"{"stage": "completed", "message": "本地初始化完成"}"#.to_string()),
            Some(100),
            None
        ).await.map_err(|e| format!("更新应用状态失败: {}", e))?;
        
        // 13. 记录用户操作完成
        let duration = start_time.elapsed().as_secs() as i32;
        db_manager.complete_user_action(
            action_id,
            "SUCCESS",
            Some("本地初始化完成，已注册客户端".to_string()),
            Some(duration)
        ).await.map_err(|e| format!("完成用户操作记录失败: {}", e))?;
        
        // 关闭数据库连接
        drop(database);
        
        Ok("本地初始化完成，已注册客户端".to_string())
    }.await;
    
    // 恢复原始工作目录
    let _ = std::env::set_current_dir(original_dir);
    
    result
}

/// 下载和部署服务包（在初始化完成后单独调用）
#[command]
pub async fn download_and_deploy_services(app_handle: AppHandle) -> Result<String, String> {
    // 1. 获取用户设置的工作目录
    let state = app_handle.state::<AppGlobalState>();
    let working_dir = state.working_directory.read().await;
    
    let base_dir = working_dir.as_ref().ok_or("请先选择工作目录")?;
    
    // 基于用户选择的目录构建路径
    let config_path = base_dir.join("data").join("config.toml");
    let docker_compose_path = base_dir.join("docker").join("docker-compose.yml");
    
    // 检查是否已初始化
    if !config_path.exists() {
        return Err("请先完成初始化".to_string());
    }
    
    // 临时切换到用户选择的工作目录
    let original_dir = std::env::current_dir().map_err(|e| format!("获取当前目录失败: {}", e))?;
    std::env::set_current_dir(base_dir).map_err(|e| format!("切换到工作目录失败: {}", e))?;
    
    // 执行下载和部署（确保在结束时恢复目录）
    let result = async {
        // 2. 获取全局数据库管理器（✅ 使用单例！）
        let db_manager = state.get_or_init_db_manager(base_dir).await?;
        
        // 3. 记录用户操作开始
        let action_id = db_manager.record_user_action(
            "DEPLOY_SERVICES",
            "下载和部署Docker服务",
            Some(r#"{"service_type": "docker_services"}"#.to_string())
        ).await.map_err(|e| format!("记录用户操作失败: {}", e))?;
        
        let start_time = Instant::now();
        
        // 4. 更新应用状态为部署中
        db_manager.update_app_state(
            "DEPLOYING",
            Some(r#"{"stage": "downloading", "message": "正在下载服务包"}"#.to_string()),
            Some(5),
            None
        ).await.map_err(|e| format!("更新应用状态失败: {}", e))?;
        
        // 5. 加载配置以获取API客户端
        let config = AppConfig::find_and_load_config().map_err(|e| format!("加载配置失败: {}", e))?;
        
        // 6. 初始化数据库
        let db_path = base_dir.join("data").join("duck_client.db"); // 使用标准数据库文件名
        let database = Database::connect(&db_path)
            .await
            .map_err(|e| format!("初始化数据库失败: {}", e))?;
        
        // 7. 创建认证客户端
        let server_base_url = client_core::constants::api::DEFAULT_BASE_URL.to_string();
        let authenticated_client = AuthenticatedClient::new(database.clone(), server_base_url)
            .await
            .map_err(|e| format!("创建认证客户端失败: {}", e))?;
        
        // 8. 获取客户端ID
        let client_id = database.get_api_client_id().await.map_err(|e| format!("获取客户端ID失败: {}", e))?;
        let mut api_client = ApiClient::new(client_id);
        api_client.set_authenticated_client(authenticated_client.clone());
        
        // 9. 获取最新版本信息 - 先调用 checkVersion 接口
        // 临时进度发送函数（没有task_id，只用于版本检查阶段）
        let emit_temp_progress = |stage: &str, message: &str, percentage: f64, current_step: u32| {
            let _ = app_handle.emit("init_progress", InitProgressEvent {
                task_id: "version_check".to_string(), // 临时ID
                stage: stage.to_string(),
                message: message.to_string(),
                percentage,
                current_step: current_step as usize,
                total_steps: 4,
            });
        };
        
        emit_temp_progress("checking_version", "正在检查最新Docker服务版本...", 15.0, 1);
        
        info!("🔍 开始检查最新Docker服务版本...");
        println!("🔍 开始检查最新Docker服务版本...");
        
        let docker_service_version = match api_client.check_docker_version(&config.versions.docker_service).await {
            Ok(version_info) => {
                info!("✅ 版本检查成功：{} -> {}", version_info.current_version, version_info.latest_version);
                println!("✅ 版本检查成功：{} -> {}", version_info.current_version, version_info.latest_version);
                emit_temp_progress("checking_version", &format!("发现最新版本: {}", version_info.latest_version), 18.0, 1);
                version_info.latest_version
            }
            Err(e) => {
                warn!("⚠️ 获取版本信息失败，使用默认版本: {}", e);
                println!("⚠️ 获取版本信息失败，使用默认版本: {}", e);
                emit_temp_progress("checking_version", &format!("版本检查失败，使用默认版本: {}", config.versions.docker_service), 18.0, 1);
                config.versions.docker_service.clone()
            }
        };
        
        // 10. 计算下载路径 - 使用最新版本号
        // 路径格式：{工作目录}/cacheDuckData/download/{version}/full/docker.zip
        let relative_download_path = config.get_version_download_file_path(
            &docker_service_version,
            "full",
            client_core::constants::upgrade::DOCKER_SERVICE_PACKAGE
        );
        
        // 将相对路径转换为基于用户工作目录的绝对路径
        let download_path = base_dir.join(relative_download_path);
        
        info!("📂 下载路径配置：{}", download_path.display());
        println!("📂 下载路径配置：{}", download_path.display());
        
        // 确保下载目录存在
        if let Some(download_dir) = download_path.parent() {
            tokio::fs::create_dir_all(download_dir).await
                .map_err(|e| format!("创建下载目录失败: {}", e))?;
            info!("📁 下载目录创建完成：{}", download_dir.display());
            println!("📁 下载目录创建完成：{}", download_dir.display());
        }
        
        // 构建下载URL - 使用实际版本号而不是固定的 latest
        let download_url = format!("{}{}", 
            client_core::constants::api::DEFAULT_BASE_URL,
            client_core::constants::api::endpoints::DOCKER_DOWNLOAD_FULL
        );
        
        // 11. 创建下载任务记录（✅ 正确使用数据库！）
        let download_task_id = db_manager.create_download_task(
            "docker-service-deployment".to_string(),
            download_url.clone(),
            0, // 初始大小，稍后更新
            download_path.display().to_string(),
            None
        ).await.map_err(|e| format!("创建下载任务失败: {}", e))?;
        
        // 12. 创建进度发送函数 - 现在可以正确引用 download_task_id
        let emit_init_progress = |stage: &str, message: &str, percentage: f64, current_step: u32| {
            let _ = app_handle.emit("init_progress", InitProgressEvent {
                task_id: download_task_id.to_string(),
                stage: stage.to_string(),
                message: message.to_string(),
                percentage,
                current_step: current_step as usize,
                total_steps: 4,
            });
        };
        
        // 13. 步骤1: 下载服务包
        emit_init_progress("downloading", "正在检查服务版本和文件完整性...", 20.0, 1);
        
        info!("📥 开始下载Docker服务包...");
        println!("📥 开始下载Docker服务包...");
        println!("   📦 版本：{}", docker_service_version);
        println!("   🌐 下载URL：{}", download_url);
        println!("   💾 保存路径：{}", download_path.display());
        
        // 更新下载任务状态为下载中
        db_manager.update_download_task_status(
            download_task_id,
            "DOWNLOADING",
            None,
            None
        ).await.map_err(|e| format!("更新下载任务状态失败: {}", e))?;
        
        // 使用API客户端的智能下载方法（带哈希验证和进度回调）
        let app_handle_for_download = app_handle.clone();
        let download_task_id_for_progress = download_task_id;
        
        let download_result = api_client.download_service_update_optimized_with_progress(
            &download_path,
            Some(&docker_service_version),
            Some(move |progress: client_core::api::DownloadProgress| {
                // 发送下载进度事件到前端
                let _ = app_handle_for_download.emit("download_progress", DownloadProgressEvent {
                    task_id: download_task_id_for_progress.to_string(),
                    file_name: progress.file_name.clone(),
                    downloaded_bytes: progress.downloaded_bytes,
                    total_bytes: progress.total_bytes,
                    download_speed: progress.download_speed,
                    eta_seconds: progress.eta_seconds,
                    percentage: progress.percentage,
                    status: format!("{:?}", progress.status),
                });
                
                // 同时发送初始化进度事件，进度范围从20%到80%
                let init_percentage = 20.0 + (progress.percentage * 0.6); // 20%-80%
                let _ = app_handle_for_download.emit("init_progress", InitProgressEvent {
                    task_id: download_task_id_for_progress.to_string(),
                    stage: "downloading".to_string(),
                    message: format!("正在下载 {}... {:.1}%", progress.file_name, progress.percentage),
                    percentage: init_percentage,
                    current_step: 1,
                    total_steps: 4,
                });
            })
        ).await
        .map_err(|e| e.to_string()); // 立即转换错误为String
        
        match &download_result {
            Ok(_) => {
                // 下载成功，更新任务状态
                let _ = db_manager.update_download_task_status(
                    download_task_id,
                    "COMPLETED",
                    Some(100),
                    None
                ).await;
                
                info!("✅ Docker服务包下载完成！");
                println!("✅ Docker服务包下载完成！");
                
                // 发送下载完成事件（成功）
                let _ = app_handle.emit("download_completed", DownloadCompletedEvent {
                    task_id: download_task_id.to_string(),
                    success: true,
                    error: None,
                });
                
                emit_init_progress("downloading", "Docker服务包下载完成", 40.0, 1);
            }
            Err(error_message) => {
                // 下载失败，更新任务状态
                let _ = db_manager.update_download_task_status(
                    download_task_id,
                    "FAILED",
                    None,
                    Some(error_message.clone())
                ).await;
                
                error!("❌ Docker服务包下载失败: {}", error_message);
                println!("❌ Docker服务包下载失败: {}", error_message);
                
                // 发送下载完成事件（失败）
                let _ = app_handle.emit("download_completed", DownloadCompletedEvent {
                    task_id: download_task_id.to_string(),
                    success: false,
                    error: Some(error_message.clone()),
                });
                
                return Err(format!("下载服务包失败: {}", error_message));
            }
        }
        
        // 13. 完成下载任务
        let download_duration = start_time.elapsed().as_secs() as i32;
        db_manager.complete_download_task(
            download_task_id,
            Some(1024 * 1024), // 示例平均速度 1MB/s
            Some(download_duration)
        ).await.map_err(|e| format!("完成下载任务记录失败: {}", e))?;
        
        // 14. 步骤2: 解压服务包
        emit_init_progress("extracting", "正在解压Docker服务包...", 45.0, 2);
        
        info!("📦 开始解压Docker服务包...");
        println!("📦 开始解压Docker服务包...");
        
        // 更新应用状态
        db_manager.update_app_state(
            "DEPLOYING",
            Some(r#"{"stage": "extracting", "message": "正在解压服务包"}"#.to_string()),
            Some(45),
            None
        ).await.map_err(|e| format!("更新应用状态失败: {}", e))?;
        
        // 检查并清理现有的docker目录
        let docker_dir = base_dir.join("docker");
        if docker_dir.exists() {
            info!("🧹 清理现有docker目录: {}", docker_dir.display());
            println!("🧹 清理现有docker目录: {}", docker_dir.display());
            emit_init_progress("extracting", "清理现有docker目录...", 50.0, 2);
            std::fs::remove_dir_all(&docker_dir).map_err(|e| format!("清理docker目录失败: {}", e))?;
        }
        
        // 使用duck-cli中的解压函数
        emit_init_progress("extracting", "正在解压文件...", 55.0, 2);
        
        info!("🔄 正在解压文件到docker目录...");
        println!("🔄 正在解压文件到docker目录...");
        
        duck_cli::extract_docker_service(&download_path)
            .await
            .map_err(|e| format!("解压服务包失败: {}", e))?;
        
        info!("✅ 文件解压完成！");
        println!("✅ 文件解压完成！");
        
        emit_init_progress("extracting", "解压完成", 70.0, 2);
        
        // 15. 步骤3: 验证和准备环境
        emit_init_progress("preparing", "正在验证环境...", 75.0, 3);
        
        // 更新应用状态
        db_manager.update_app_state(
            "DEPLOYING",
            Some(r#"{"stage": "preparing", "message": "正在验证环境"}"#.to_string()),
            Some(75),
            None
        ).await.map_err(|e| format!("更新应用状态失败: {}", e))?;
        
        // 检查docker-compose.yml是否存在
        if !docker_compose_path.exists() {
            return Err("解压后的docker-compose.yml文件不存在".to_string());
        }
        
        // 创建DockerManager
        let docker_manager = DockerManager::new(&docker_compose_path)
            .map_err(|e| format!("创建Docker管理器失败: {}", e))?;
        
        // 检查Docker环境
        emit_init_progress("preparing", "检查Docker环境...", 80.0, 3);
        docker_manager.check_docker_status()
            .await
            .map_err(|e| format!("Docker环境检查失败: {}", e))?;
        
        emit_init_progress("preparing", "环境准备完成", 85.0, 3);
        
        // 16. 步骤4: 部署服务
        emit_init_progress("deploying", "正在部署Docker服务...", 90.0, 4);
        
        info!("🚀 开始部署Docker服务...");
        println!("🚀 开始部署Docker服务...");
        println!("   📁 工作目录：{}", base_dir.display());
        println!("   📄 compose文件：{}", docker_compose_path.display());
        
        // 更新应用状态
        db_manager.update_app_state(
            "DEPLOYING",
            Some(r#"{"stage": "deploying", "message": "正在部署服务"}"#.to_string()),
            Some(90),
            None
        ).await.map_err(|e| format!("更新应用状态失败: {}", e))?;
        
        // 创建DockerServiceManager
        let work_dir = base_dir.to_path_buf();
        let mut docker_service_manager = duck_cli::DockerServiceManager::new(config, docker_manager, work_dir);
        
        info!("📋 DockerServiceManager 创建完成，开始执行部署...");
        println!("📋 DockerServiceManager 创建完成，开始执行部署...");
        println!("⏳ 注意：Docker服务部署可能需要5-10分钟，请耐心等待...");
        
        // 执行完整的服务部署
        docker_service_manager.deploy_services()
            .await
            .map_err(|e| format!("服务部署失败: {}", e))?;
        
        info!("🎉 Docker服务部署完成！");
        println!("🎉 Docker服务部署完成！");
        
        emit_init_progress("deploying", "部署完成", 100.0, 4);
        
        // 17. 更新应用状态为就绪
        db_manager.update_app_state(
            "READY",
            Some(r#"{"stage": "completed", "message": "服务部署完成"}"#.to_string()),
            Some(100),
            None
        ).await.map_err(|e| format!("更新应用状态失败: {}", e))?;
        
        // 18. 记录用户操作完成
        let total_duration = start_time.elapsed().as_secs() as i32;
        db_manager.complete_user_action(
            action_id,
            "SUCCESS",
            Some(format!("服务包下载和部署完成，下载任务ID: {}", download_task_id)),
            Some(total_duration)
        ).await.map_err(|e| format!("完成用户操作记录失败: {}", e))?;
        
        // 19. 发送完成事件
        let _ = app_handle.emit("init_completed", InitCompletedEvent {
            task_id: download_task_id.to_string(), // 使用真实的数据库ID
            success: true,
            error: None,
        });
        
        Ok(format!("服务包下载和部署完成，下载任务ID: {}", download_task_id))
    }.await;
    
    // 恢复原始工作目录
    let _ = std::env::set_current_dir(original_dir);
    
    result
}

/// 下载包并显示进度
#[command]
pub async fn download_package_with_progress(
    app_handle: AppHandle,
    url: String,
    target_path: String,
) -> Result<String, String> {
    // 获取用户设置的工作目录 
    let state = app_handle.state::<AppGlobalState>();
    let working_dir = state.working_directory.read().await;
    let base_dir = working_dir.as_ref().ok_or("请先选择工作目录")?;
    
    // 获取全局数据库管理器（✅ 使用单例！）
    let db_manager = state.get_or_init_db_manager(base_dir).await?;
    
    // 创建下载任务记录
    let download_task_id = db_manager.create_download_task(
        "manual-download".to_string(),
        url.clone(),
        0, // 初始大小，下载过程中更新
        target_path.clone(),
        None
    ).await.map_err(|e| format!("创建下载任务失败: {}", e))?;
    
    // 将target_path转换为PathBuf
    let target_path_buf = std::path::PathBuf::from(&target_path);
    
    // 克隆必要的数据用于任务处理
    let app_handle_clone = app_handle.clone();
    let db_manager_clone = db_manager.clone();
    let url_clone = url.clone();
    
    // 在单独的任务中执行下载，避免Send trait问题
    tokio::spawn(async move {
        let start_time = Instant::now();
        
        // 更新任务状态为下载中
        let _ = db_manager_clone.update_download_task_status(
            download_task_id,
            "DOWNLOADING",
            None,
            None
        ).await;
        
        // ✅ 在闭包中执行下载，将错误立即转换为String
        let download_result: Result<(), String> = {
            let app_handle_for_progress = app_handle_clone.clone();
            
            // 执行下载，立即转换错误为String
            match download_with_progress(&url_clone, &target_path_buf, move |progress| {
                // 发送前端事件（这个闭包是同步的）
                let _ = app_handle_for_progress.emit("download_progress", DownloadProgressEvent {
                    task_id: download_task_id.to_string(),
                    file_name: progress.file_name,
                    downloaded_bytes: progress.downloaded_bytes,
                    total_bytes: progress.total_bytes,
                    download_speed: progress.download_speed,
                    eta_seconds: progress.eta_seconds,
                    percentage: progress.percentage,
                    status: format!("{:?}", progress.status),
                });
            }).await {
                Ok(_) => Ok(()),
                Err(e) => Err(e.to_string()), // ✅ 立即转换为String
            }
        };
        
        // 处理结果并发送完成事件
        match download_result {
            Ok(_) => {
                // 完成下载任务
                let duration = start_time.elapsed().as_secs() as i32;
                let _ = db_manager_clone.complete_download_task(
                    download_task_id,
                    Some(1024 * 1024), // 示例平均速度
                    Some(duration)
                ).await;
                
                let _ = app_handle_clone.emit("download_completed", DownloadCompletedEvent {
                    task_id: download_task_id.to_string(),
                    success: true,
                    error: None,
                });
            },
            Err(error_message) => {
                // 更新任务状态为失败
                let _ = db_manager_clone.update_download_task_status(
                    download_task_id,
                    "FAILED",
                    None,
                    Some(error_message.clone())
                ).await;
                
                let _ = app_handle_clone.emit("download_completed", DownloadCompletedEvent {
                    task_id: download_task_id.to_string(),
                    success: false,
                    error: Some(error_message),
                });
            }
        }
    });

    Ok(format!("开始下载包，任务ID: {}", download_task_id))
} 