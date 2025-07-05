use client_core::error::Result;
use std::io::{Read, Write};
#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

/// 判断是否应该跳过某个文件（智能过滤）
/// 
/// 跳过的文件类型：
/// - macOS 系统文件：__MACOSX, .DS_Store, ._*
/// - 版本控制文件：.git/, .gitignore, .gitattributes
/// - 临时文件：.tmp, .temp, .bak
/// - IDE 文件：.vscode/, .idea/
/// 
/// 保留的重要配置文件：
/// - Docker 配置：.env, .env.*, .dockerignore
/// - 其他配置：.editorconfig, .prettier*, .eslint*
fn should_skip_file(file_name: &str) -> bool {
    // 跳过 macOS 系统文件和临时文件
    if file_name.starts_with("__MACOSX") 
        || file_name.ends_with(".DS_Store")
        || file_name.starts_with("._")
        || file_name.ends_with(".tmp")
        || file_name.ends_with(".temp")
        || file_name.ends_with(".bak") {
        return true;
    }

    // 跳过版本控制相关文件
    if file_name.starts_with(".git/") 
        || file_name == ".gitignore"
        || file_name == ".gitattributes"
        || file_name == ".gitmodules" {
        return true;
    }

    // 跳过 IDE 和编辑器配置目录
    if file_name.starts_with(".vscode/")
        || file_name.starts_with(".idea/")
        || file_name.starts_with(".vs/") {
        return true;
    }

    // 保留重要的配置文件（即使以.开头）
    if file_name == ".env"
        || file_name.starts_with(".env.")
        || file_name == ".dockerignore"
        || file_name == ".editorconfig"
        || file_name.starts_with(".prettier")
        || file_name.starts_with(".eslint") {
        return false;
    }

    // 其他以.开头的文件，谨慎起见也保留（除非明确知道要跳过）
    false
}

/// # Duck CLI 日志系统使用说明
///
/// 本项目遵循 Rust CLI 应用的日志最佳实践：
///
/// ## 基本原则
/// 1. **库代码只使用 `tracing` 宏**：`info!()`, `warn!()`, `error!()`, `debug!()`
/// 2. **应用入口控制日志配置**：在 `main.rs` 中调用 `setup_logging()`
/// 3. **用户界面输出与日志分离**：备份列表等用户友好信息通过其他方式输出
///
/// ## 日志配置选项
///
/// ### 命令行参数
/// - `-v, --verbose`：启用详细日志模式（DEBUG 级别）
///
/// ### 环境变量
/// - `RUST_LOG`：标准的 Rust 日志级别控制（如 `debug`, `info`, `warn`, `error`）
/// - `DUCK_LOG_FILE`：日志文件路径，设置后日志输出到文件而非终端
/// - `DUCK_GUI_MODE`：GUI模式标识，设置后禁用大部分tracing日志输出，避免与程序输出重复
///
/// ## 使用示例
///
/// ```bash
/// # 标准日志输出到终端
/// duck-cli auto-backup status
///
/// # 详细日志输出到终端
/// duck-cli -v auto-backup status
///
/// # 日志输出到文件
/// DUCK_LOG_FILE=duck.log duck-cli auto-backup status
///
/// # GUI模式（自动设置，避免日志重复）
/// DUCK_GUI_MODE=1 duck-cli auto-backup status
///
/// # 使用 RUST_LOG 控制特定模块的日志级别
/// RUST_LOG=duck_cli::commands::auto_backup=debug duck-cli auto-backup status
/// ```
///
/// ## 作为库使用
///
/// 当 duck-cli 作为库被其他项目使用时，可以：
/// 1. 让使用者完全控制日志配置（推荐）
/// 2. 或调用 `setup_minimal_logging()` 进行最小化配置
///
/// ## 日志格式
/// - **终端输出**：人类可读格式，不显示模块路径
/// - **文件输出**：包含完整模块路径和更多调试信息
///
/// 带进度显示的文件复制
#[allow(dead_code)]
pub fn copy_with_progress<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    total_size: u64,
    file_name: &str,
) -> std::io::Result<u64> {
    let mut buf = [0u8; 8192]; // 8KB 缓冲区
    let mut copied = 0u64;
    let mut last_percent = 0;

    loop {
        let bytes_read = reader.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }

        writer.write_all(&buf[..bytes_read])?;
        copied += bytes_read as u64;

        // 显示大文件的复制进度（每10%或每100MB显示一次）
        if total_size > 100 * 1024 * 1024 {
            // 只对大于100MB的文件显示详细进度
            let percent = if total_size > 0 {
                (copied * 100) / total_size
            } else {
                0
            };
            let mb_copied = copied as f64 / 1024.0 / 1024.0;
            let mb_total = total_size as f64 / 1024.0 / 1024.0;

            // 每10%或每100MB更新一次进度
            if (percent != last_percent && percent % 10 == 0)
                || (copied % (100 * 1024 * 1024) == 0 && copied > 0)
            {
                info!(
                    "     ⏳ {} 复制进度: {:.1}% ({:.1}/{:.1} MB)",
                    file_name, percent as f64, mb_copied, mb_total
                );
                last_percent = percent;
            }
        }
    }

    Ok(copied)
}

/// 解压Docker服务包
#[allow(dead_code)]
pub async fn extract_docker_service(zip_path: &std::path::Path) -> Result<()> {
    use std::time::Instant;
    let extract_start = Instant::now();
    
    info!("🔍 正在分析ZIP文件: {}", zip_path.display());

    // 打开ZIP文件
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    info!("✅ ZIP文件打开成功，开始分析内部结构...");

    // 分析ZIP内部结构，检查是否有顶层docker目录
    let mut has_docker_root = false;
    let mut docker_root_prefix = String::new();

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let file_name = file.name();

        // 跳过隐藏文件和macOS临时文件
        if file_name.starts_with('.') || file_name.starts_with("__MACOSX") {
            continue;
        }

        // 检查是否有docker-compose.yml，确定根目录结构
        if file_name.ends_with(client_core::constants::docker::COMPOSE_FILE_NAME) {
            info!("🎯 发现 docker-compose.yml: {}", file_name);
            
            // 检查文件路径，确定解压策略
            if let Some(parent_dir) = std::path::Path::new(file_name).parent() {
                if parent_dir != std::path::Path::new("") {
                    has_docker_root = true;
                    docker_root_prefix = parent_dir.to_string_lossy().to_string();
                    info!("📁 检测到顶层目录: {}", docker_root_prefix);
                    break;
                }
            }
        }
    }

    // 首先统计需要解压的文件数量（使用智能过滤跳过系统文件，保留重要配置文件）
    let mut total_files = 0;
    let mut total_size = 0u64;
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        if !should_skip_file(file.name()) && !file.is_dir() {
            total_files += 1;
            total_size += file.size();
        }
    }

    info!("📊 解压统计分析:");
    info!("   📁 总文件数: {}", total_files);
    info!("   📏 总数据量: {:.1} MB", total_size as f64 / 1024.0 / 1024.0);
    info!("   🗂️  解压策略: {}", if has_docker_root { 
        format!("移除顶层目录 '{}'", docker_root_prefix) 
    } else { 
        "直接解压到docker目录".to_string() 
    });

    let output_dir = std::path::Path::new("docker");
    
    // 重新打开archive进行解压（避免借用冲突）
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut extracted_files = 0;
    let mut extracted_size = 0u64;
    let mut last_progress_report = 0; // 最后一次进度报告

    info!("🚀 开始解压文件...");
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        
        // 先获取必要的文件信息
        let file_name = file.name().to_string();
        let file_size = file.size();
        let file_is_dir = file.is_dir();

        // 使用智能过滤跳过系统文件，保留重要配置文件如 .env
        if should_skip_file(&file_name) {
            continue;
        }

        // 处理文件路径（移除顶层docker目录前缀）
        let target_path = if has_docker_root && file_name.starts_with(&docker_root_prefix) {
            // 移除顶层目录前缀
            let relative_path = file_name.strip_prefix(&format!("{}/", docker_root_prefix))
                .unwrap_or(&file_name);
            output_dir.join(relative_path)
        } else {
            output_dir.join(&file_name)
        };

        if file_is_dir {
            // 创建目录
            debug!("📁 创建目录: {}", target_path.display());
            std::fs::create_dir_all(&target_path)?;
        } else {
            // 确保父目录存在
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // 解压文件
            if file_size > 50 * 1024 * 1024 { // 大于50MB的文件显示详细信息
                info!("📦 正在解压大文件: {} ({:.1} MB)", 
                    target_path.file_name().unwrap_or_default().to_string_lossy(),
                    file_size as f64 / 1024.0 / 1024.0
                );
            }

            let mut outfile = std::fs::File::create(&target_path)?;
            std::io::copy(&mut file, &mut outfile)?;

            extracted_files += 1;
            extracted_size += file_size;

            // 每解压25%的文件或每1000个文件报告一次进度
            let progress_percentage = (extracted_files * 100) / total_files;
            if progress_percentage >= last_progress_report + 25 || extracted_files % 1000 == 0 {
                last_progress_report = progress_percentage;
                let extracted_mb = extracted_size as f64 / 1024.0 / 1024.0;
                let total_mb = total_size as f64 / 1024.0 / 1024.0;
                let speed_mbps = extracted_mb / extract_start.elapsed().as_secs_f64();
                
                info!("📈 解压进度: {}% ({}/{} 文件, {:.1}/{:.1} MB, {:.1} MB/s)", 
                    progress_percentage, extracted_files, total_files, 
                    extracted_mb, total_mb, speed_mbps);
            }
        }
    }

    let total_elapsed = extract_start.elapsed();
    let extracted_size_mb = extracted_size as f64 / 1024.0 / 1024.0;
    
    info!("🎉 解压完成！");
    info!("📊 解压统计:");
    info!("   ✅ 成功解压文件: {} 个", extracted_files);
    info!("   📏 解压数据大小: {:.1} MB", extracted_size_mb);
    info!("   ⏱️  总耗时: {:?}", total_elapsed);
    info!("   🚀 平均速度: {:.1} MB/s", extracted_size_mb / total_elapsed.as_secs_f64());
    
    info!("解压统计: {} 文件, {:.1}MB, 耗时 {:?}", 
        extracted_files, extracted_size_mb, total_elapsed);

    Ok(())
}

/// 设置日志记录系统
///
/// 这个函数遵循Rust CLI应用的最佳实践：
/// - 库代码只使用 tracing 宏记录日志
/// - 在应用入口配置日志输出行为
/// - 支持 RUST_LOG 环境变量控制日志级别
/// - 默认输出到stderr，避免与程序输出混淆
/// - 终端输出简洁格式，文件输出详细格式
pub fn setup_logging(verbose: bool) {
    #[allow(unused_imports)]
    use tracing_subscriber::{EnvFilter, fmt, util::SubscriberInitExt};

    // 检查是否为GUI模式 - 如果是，则大幅简化日志输出
    if std::env::var("DUCK_GUI_MODE").is_ok() {
        // GUI模式：大幅减少tracing日志输出，避免与程序输出重复
        // 只保留WARN和ERROR级别的日志，过滤掉大部分INFO级别
        let env_filter = EnvFilter::new("warn")
            .add_directive("duck_cli=error".parse().unwrap())
            .add_directive("client_core=error".parse().unwrap());
        
        // 输出到stderr，使用最简格式
        fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .with_thread_names(false)
            .with_line_number(false)
            .without_time()
            .compact()
            .init();
        return;
    }

    // 根据verbose参数和环境变量确定日志级别
    let default_level = if verbose { "debug" } else { "info" };
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    // 检查环境变量，决定是否输出到文件
    if let Ok(log_file) = std::env::var("DUCK_LOG_FILE") {
        // 输出到文件 - 使用详细格式便于调试
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .expect("Failed to create log file");

        fmt()
            .with_env_filter(env_filter)
            .with_writer(file)
            .with_target(true)
            .with_thread_names(true)
            .with_line_number(true)
            .init();
    } else {
        // 输出到终端 - 使用简洁格式，用户友好
        fmt()
            .with_env_filter(env_filter)
            .with_target(false) // 不显示模块路径
            .with_thread_names(false) // 不显示线程名
            .with_line_number(false) // 不显示行号
            .without_time() // 不显示时间戳
            .compact() // 使用紧凑格式
            .init();
    }
}

/// 为库使用提供的简化日志初始化
///
/// 当duck-cli作为库使用时，可以调用此函数进行最小化的日志配置
/// 或者让库的使用者完全控制日志配置
#[allow(dead_code)]
pub fn setup_minimal_logging() {
    #[allow(unused_imports)]
    use tracing_subscriber::{EnvFilter, fmt, util::SubscriberInitExt};

    // 尝试初始化一个简单的订阅者
    // 如果已经有全局订阅者，这会返回错误，我们忽略它
    let _ = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact() // 使用紧凑格式
        .try_init();
}
