use client_core::error::Result;
use std::io::{Read, Write};
use tracing::info;

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
    info!("   🔍 正在分析ZIP文件...");

    // 打开ZIP文件
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

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
        if file_name.ends_with("docker-compose.yml") {
            if let Some(pos) = file_name.rfind("docker-compose.yml") {
                let prefix = &file_name[..pos];
                if prefix.is_empty() {
                    // docker-compose.yml在根目录
                    has_docker_root = false;
                } else if prefix == "docker/" {
                    // docker-compose.yml在docker/目录下
                    has_docker_root = true;
                    docker_root_prefix = "docker/".to_string();
                } else {
                    // 其他目录结构
                    docker_root_prefix = prefix.to_string();
                    has_docker_root = true;
                }
                break;
            }
        }
    }

    info!(
        "   📋 ZIP结构分析: {}",
        if has_docker_root {
            format!("包含根目录 '{}'", docker_root_prefix.trim_end_matches('/'))
        } else {
            "文件直接在根目录".to_string()
        }
    );

    // 重新打开文件进行统计
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // 首先统计需要解压的文件数量（跳过隐藏文件和目录）
    let mut total_files = 0;
    let mut total_size = 0u64;
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        if !file.name().starts_with('.') && !file.name().starts_with("__MACOSX") && !file.is_dir() {
            total_files += 1;
            total_size += file.size();
        }
    }

    info!(
        "   📊 总计需要解压: {} 个文件 (总大小: {:.1} GB)",
        total_files,
        total_size as f64 / 1024.0 / 1024.0 / 1024.0
    );

    // 确定解压目标目录
    let extract_dir = if has_docker_root {
        // 如果ZIP内部有docker目录，直接解压到当前目录，让内部的docker目录成为我们的docker目录
        std::path::Path::new(".")
    } else {
        // 如果ZIP内部没有docker目录，解压到docker目录
        std::fs::create_dir_all("docker")?;
        std::path::Path::new("docker")
    };

    info!("   📁 解压目标: {}", extract_dir.display());

    // 重新打开ZIP文件进行解压
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut extracted_files = 0;
    let mut extracted_size = 0u64;
    let mut last_percent = 0;

    info!("   📤 开始解压文件...");

    // 解压所有文件
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_name = file.name();

        // 跳过隐藏文件和系统文件
        if file_name.starts_with('.') || file_name.starts_with("__MACOSX") {
            continue;
        }

        // 构建输出路径
        let outpath = if has_docker_root {
            // 如果ZIP内部有docker目录，直接解压到当前目录，保持内部的docker目录结构
            std::path::PathBuf::from(file_name)
        } else {
            // 如果ZIP内部没有docker目录，解压到docker目录下
            std::path::Path::new("docker").join(file_name)
        };

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // 显示当前正在解压的文件（如果文件很大的话）
            let file_size = file.size();
            if file_size > 100 * 1024 * 1024 {
                // 大于100MB的文件
                info!(
                    "   📄 正在解压大文件: {} ({:.1} MB)",
                    file.name(),
                    file_size as f64 / 1024.0 / 1024.0
                );
            }

            let mut outfile = std::fs::File::create(&outpath)?;

            // 使用带进度显示的复制函数
            if file_size > 100 * 1024 * 1024 {
                let file_name = file.name().to_string(); // 先获取文件名
                copy_with_progress(&mut file, &mut outfile, file_size, &file_name)?;
            } else {
                std::io::copy(&mut file, &mut outfile)?;
            }

            extracted_files += 1;
            extracted_size += file_size;

            // 计算并显示进度（每5%显示一次，或者每50个文件显示一次）
            if total_files > 0 {
                let percent = (extracted_files * 100) / total_files;
                if percent != last_percent && (percent % 5 == 0 || extracted_files % 50 == 0) {
                    let size_percent = if total_size > 0 {
                        (extracted_size * 100) / total_size
                    } else {
                        0
                    };
                    info!(
                        "   📤 解压进度: {}% ({}/{} 文件, {:.1}% 大小)",
                        percent, extracted_files, total_files, size_percent
                    );
                    last_percent = percent;
                }
            }
        }
    }

    info!(
        "   📤 解压完成: 100% ({}/{} 文件, {:.1} GB)",
        extracted_files,
        total_files,
        extracted_size as f64 / 1024.0 / 1024.0 / 1024.0
    );

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
    use tracing_subscriber::{EnvFilter, fmt};

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
    use tracing_subscriber::{EnvFilter, fmt};

    // 尝试初始化一个简单的订阅者
    // 如果已经有全局订阅者，这会返回错误，我们忽略它
    let _ = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact() // 使用紧凑格式
        .try_init();
}
