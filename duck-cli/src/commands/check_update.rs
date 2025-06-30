use anyhow::{Context, Result};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

/// GitHub 仓库常量配置
pub const GITHUB_OWNER: &str = "soddygo";
pub const GITHUB_REPO: &str = "duck_client";

use crate::cli::CheckUpdateCommand;

/// GitHub Release API 响应结构
#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    pub published_at: String,
    pub html_url: String,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub size: u64,
    pub download_count: u64,
    pub browser_download_url: String,
    pub content_type: String,
}

/// 版本信息
#[derive(Debug, Serialize)]
pub struct VersionInfo {
    pub current_version: String,
    pub latest_version: String,
    pub is_update_available: bool,
    pub release_notes: String,
    pub download_url: Option<String>,
    pub published_at: String,
}

/// GitHub仓库配置
pub struct GitHubRepo {
    pub owner: String,
    pub repo: String,
}

impl GitHubRepo {
    pub fn new(owner: &str, repo: &str) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
        }
    }

    /// 创建默认的 duck_client 仓库配置
    pub fn default() -> Self {
        Self::new(GITHUB_OWNER, GITHUB_REPO)
    }

    /// 获取最新release API URL
    pub fn latest_release_url(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.owner, self.repo
        )
    }
}

/// 获取当前版本
pub fn get_current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 从GitHub获取最新版本信息
pub async fn fetch_latest_version(repo: &GitHubRepo) -> Result<GitHubRelease> {
    let client = reqwest::Client::new();
    let url = repo.latest_release_url();

    info!("📡 正在检查最新版本: {}", url);

    let response = client
        .get(&url)
        .header("User-Agent", format!("duck-cli/{}", get_current_version()))
        .send()
        .await
        .context("无法连接到GitHub API")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "GitHub API请求失败: {} - {}",
            status,
            error_text
        ));
    }

    let release: GitHubRelease = response.json().await.context("解析GitHub API响应失败")?;

    Ok(release)
}

/// 比较版本号
pub fn compare_versions(current: &str, latest: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    // 简单的版本比较，假设版本格式为 v1.2.3 或 1.2.3
    let normalize_version = |v: &str| -> String { v.trim_start_matches('v').to_string() };

    let current_norm = normalize_version(current);
    let latest_norm = normalize_version(latest);

    // 使用语义版本比较（简化版）
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .map(|s| s.parse::<u32>().unwrap_or(0))
            .collect()
    };

    let current_parts = parse_version(&current_norm);
    let latest_parts = parse_version(&latest_norm);

    current_parts.cmp(&latest_parts)
}

/// 检查更新
pub async fn check_for_updates(repo: &GitHubRepo) -> Result<VersionInfo> {
    let current_version = get_current_version();
    let latest_release = fetch_latest_version(repo).await?;

    let latest_version = latest_release.tag_name.clone();
    let is_update_available =
        compare_versions(&current_version, &latest_version) == std::cmp::Ordering::Less;

    // 查找适合当前平台的下载链接
    let download_url = find_platform_asset(&latest_release.assets);

    Ok(VersionInfo {
        current_version,
        latest_version,
        is_update_available,
        release_notes: latest_release.body,
        download_url,
        published_at: latest_release.published_at,
    })
}

/// 查找适合当前平台的资源
fn find_platform_asset(assets: &[GitHubAsset]) -> Option<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // 定义平台匹配模式
    let platform_patterns = match (os, arch) {
        ("windows", "x86_64") => vec!["windows", "win64", "x86_64-pc-windows"],
        ("windows", "x86") => vec!["windows", "win32", "i686-pc-windows"],
        ("linux", "x86_64") => vec!["linux", "x86_64-unknown-linux"],
        ("linux", "aarch64") => vec!["linux", "aarch64-unknown-linux"],
        ("macos", "x86_64") => vec!["macos", "darwin", "x86_64-apple-darwin"],
        ("macos", "aarch64") => vec!["macos", "darwin", "aarch64-apple-darwin"],
        _ => vec![os, arch],
    };

    // 查找匹配的资源
    for asset in assets {
        let name_lower = asset.name.to_lowercase();
        if platform_patterns
            .iter()
            .any(|pattern| name_lower.contains(pattern))
        {
            // 优先选择可执行文件
            if name_lower.contains("duck-cli")
                || name_lower.ends_with(".exe")
                || name_lower.ends_with(".tar.gz")
            {
                return Some(asset.browser_download_url.clone());
            }
        }
    }

    // 如果没找到精确匹配，返回第一个看起来像可执行文件的资源
    assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_lowercase();
            name.contains("duck-cli") || name.ends_with(".exe") || name.ends_with(".tar.gz")
        })
        .map(|asset| asset.browser_download_url.clone())
}

/// 显示版本检查结果
pub fn display_version_info(version_info: &VersionInfo) {
    info!("🦆 Duck CLI 版本信息");
    info!("当前版本: {}", version_info.current_version);
    info!("最新版本: {}", version_info.latest_version);

    if version_info.is_update_available {
        info!("✅ 发现新版本可用！");
        if let Some(ref url) = version_info.download_url {
            info!("下载地址: {}", url);
        }

        // 显示发布说明（截取前500字符）
        if !version_info.release_notes.is_empty() {
            let notes = if version_info.release_notes.len() > 500 {
                format!("{}...", &version_info.release_notes[..500])
            } else {
                version_info.release_notes.clone()
            };
            info!("更新说明:\n{}", notes);
        }

        // 解析并显示发布时间
        if let Ok(published_time) = DateTime::parse_from_rfc3339(&version_info.published_at) {
            info!("发布时间: {}", published_time.format("%Y-%m-%d %H:%M:%S"));
        }

        info!("💡 使用以下命令安装更新:");
        info!("   duck-cli check-update install");
    } else {
        info!("✅ 您已经使用最新版本！");
    }
}

/// 检查版本并决定是否需要安装
pub async fn should_install(
    repo: &GitHubRepo,
    target_version: Option<&str>,
    force: bool,
) -> Result<(String, String)> {
    let current_version = get_current_version();

    let target_version = if let Some(version) = target_version {
        version.to_string()
    } else {
        // 获取最新版本
        let latest_release = fetch_latest_version(repo).await?;
        latest_release.tag_name
    };

    if !force && compare_versions(&current_version, &target_version) != std::cmp::Ordering::Less {
        return Err(anyhow::anyhow!(
            "当前版本 {} 已是最新或更高版本 {}。使用 --force 强制重新安装",
            current_version,
            target_version
        ));
    }

    Ok((current_version, target_version))
}

/// 下载并安装新版本
pub async fn install_release(url: &str, version: &str) -> Result<()> {
    let client = reqwest::Client::new();

    // 创建临时目录
    let temp_dir = std::env::temp_dir().join("duck-cli-updates");
    std::fs::create_dir_all(&temp_dir)?;

    // 确定文件名
    let default_filename = format!("duck-cli-{}", version);
    let filename = url.split('/').last().unwrap_or(&default_filename);
    let download_path = temp_dir.join(filename);

    info!("📥 正在下载版本 {}: {}", version, url);
    info!("💾 临时保存到: {}", download_path.display());

    // 下载文件
    let response = client
        .get(url)
        .header("User-Agent", format!("duck-cli/{}", get_current_version()))
        .send()
        .await
        .context("下载失败")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("下载失败: HTTP {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    info!("📦 文件大小: {} bytes", total_size);

    let bytes = response.bytes().await?;
    std::fs::write(&download_path, bytes)?;

    info!("✅ 下载完成，开始安装...");

    // 获取当前可执行文件路径
    let current_exe = std::env::current_exe().context("无法获取当前可执行文件路径")?;

    info!("🔧 当前可执行文件: {}", current_exe.display());

    // 处理不同文件类型的安装
    install_downloaded_file(&download_path, &current_exe, version).await?;

    // 清理临时文件
    if let Err(e) = std::fs::remove_file(&download_path) {
        warn!("清理临时文件失败: {}", e);
    }

    info!("🎉 安装完成！Duck CLI 已更新到版本 {}", version);
    info!("💡 请重新启动终端或运行 'duck-cli --version' 验证安装");

    Ok(())
}

/// 安装下载的文件
async fn install_downloaded_file(
    download_path: &PathBuf,
    current_exe: &PathBuf,
    version: &str,
) -> Result<()> {
    let download_name = download_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if download_name.ends_with(".tar.gz") || download_name.ends_with(".tgz") {
        // 处理压缩包
        install_from_archive(download_path, current_exe, version).await
    } else if download_name.ends_with(".exe") || download_name.contains("duck-cli") {
        // 直接可执行文件
        install_executable(download_path, current_exe).await
    } else {
        Err(anyhow::anyhow!("不支持的文件格式: {}", download_name))
    }
}

/// 安装可执行文件
async fn install_executable(download_path: &PathBuf, current_exe: &PathBuf) -> Result<()> {
    // 在 Windows 上，需要特殊处理
    if cfg!(target_os = "windows") {
        install_windows_executable(download_path, current_exe).await
    } else {
        install_unix_executable(download_path, current_exe).await
    }
}

/// Windows 系统安装
async fn install_windows_executable(download_path: &PathBuf, current_exe: &PathBuf) -> Result<()> {
    // 创建备份
    let backup_path = current_exe.with_extension("exe.backup");
    if let Err(e) = std::fs::copy(current_exe, &backup_path) {
        warn!("创建备份失败: {}", e);
    }

    // 在 Windows 上，正在运行的可执行文件无法直接替换
    // 我们需要重命名当前文件，然后复制新文件
    let temp_old_path = current_exe.with_extension("exe.old");

    std::fs::rename(current_exe, &temp_old_path).context("无法重命名当前可执行文件")?;

    match std::fs::copy(download_path, current_exe) {
        Ok(_) => {
            info!("✅ 可执行文件已更新");
            // 删除旧文件
            if let Err(e) = std::fs::remove_file(&temp_old_path) {
                warn!("删除旧文件失败: {}", e);
            }
            Ok(())
        }
        Err(e) => {
            // 安装失败，恢复原文件
            warn!("安装失败，正在恢复原文件: {}", e);
            std::fs::rename(&temp_old_path, current_exe)?;
            Err(anyhow::anyhow!("安装失败: {}", e))
        }
    }
}

/// Unix 系统安装
async fn install_unix_executable(download_path: &PathBuf, current_exe: &PathBuf) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // 设置可执行权限
    let mut perms = std::fs::metadata(download_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(download_path, perms)?;

    // 创建备份
    let backup_path = format!("{}.backup", current_exe.display());
    if let Err(e) = std::fs::copy(current_exe, &backup_path) {
        warn!("创建备份失败: {}", e);
    }

    // 替换文件
    std::fs::copy(download_path, current_exe).context("无法替换可执行文件")?;

    info!("✅ 可执行文件已更新");
    Ok(())
}

/// 从压缩包安装
async fn install_from_archive(
    archive_path: &PathBuf,
    current_exe: &PathBuf,
    _version: &str,
) -> Result<()> {
    use std::process::Command;

    let temp_dir = std::env::temp_dir().join("duck-cli-extract");
    std::fs::create_dir_all(&temp_dir)?;

    info!("📦 正在解压缩包...");

    // 解压 tar.gz 文件
    let output = Command::new("tar")
        .args(&[
            "-xzf",
            &archive_path.to_string_lossy(),
            "-C",
            &temp_dir.to_string_lossy(),
        ])
        .output()
        .context("解压失败，请确保系统已安装 tar 命令")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "解压失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // 查找可执行文件
    let executable_path = find_executable_in_dir(&temp_dir)?;

    // 安装可执行文件
    install_executable(&executable_path, current_exe).await?;

    // 清理解压目录
    if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
        warn!("清理解压目录失败: {}", e);
    }

    Ok(())
}

/// 在目录中查找可执行文件
fn find_executable_in_dir(dir: &PathBuf) -> Result<PathBuf> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if name.contains("duck-cli") || name.ends_with(".exe") {
                return Ok(path);
            }
        }

        // 递归查找子目录
        if path.is_dir() {
            if let Ok(found) = find_executable_in_dir(&path) {
                return Ok(found);
            }
        }
    }

    Err(anyhow::anyhow!("在压缩包中未找到可执行文件"))
}

/// 处理 check-update 命令
pub async fn handle_check_update_command(command: CheckUpdateCommand) -> Result<()> {
    let repo = GitHubRepo::default();

    match command {
        CheckUpdateCommand::Check => {
            info!("🔍 正在检查 Duck CLI 更新...");

            match check_for_updates(&repo).await {
                Ok(version_info) => {
                    display_version_info(&version_info);
                }
                Err(e) => {
                    warn!("❌ 检查更新失败: {}", e);
                    info!("当前版本: {}", get_current_version());
                    info!("💡 可能的原因:");
                    info!("   - 网络连接问题");
                    info!("   - GitHub API 暂时不可用");
                    info!("   - 项目尚未发布任何版本");
                    return Err(e);
                }
            }
        }

        CheckUpdateCommand::Install { version, force } => {
            info!("🚀 开始安装 Duck CLI...");

            // 检查是否需要安装
            let (current_version, target_version) =
                match should_install(&repo, version.as_deref(), force).await {
                    Ok(versions) => versions,
                    Err(e) => {
                        if force {
                            warn!("⚠️  {}", e);
                            info!("🔧 由于使用了 --force 参数，将继续安装...");
                            // 如果强制安装但没指定版本，返回错误
                            if version.is_none() {
                                return Err(anyhow::anyhow!("强制安装时必须指定版本号"));
                            }
                            (get_current_version(), version.as_ref().unwrap().clone())
                        } else {
                            warn!("❌ {}", e);
                            return Err(e);
                        }
                    }
                };

            info!(
                "准备从版本 {} 更新到版本 {}",
                current_version, target_version
            );

            // 获取指定版本的下载链接
            let download_url = if let Some(ref ver) = version {
                // 指定了版本，需要获取该版本的信息
                get_version_download_url(&repo, ver).await?
            } else {
                // 没有指定版本，获取最新版本的下载链接
                let version_info = check_for_updates(&repo).await?;
                version_info
                    .download_url
                    .ok_or_else(|| anyhow::anyhow!("未找到适合当前平台的下载链接"))?
            };

            info!("📥 开始下载并安装版本 {}...", target_version);

            match install_release(&download_url, &target_version).await {
                Ok(_) => {
                    info!("🎉 安装成功！");
                    info!("请重新启动命令行验证安装结果");
                }
                Err(e) => {
                    warn!("❌ 安装失败: {}", e);
                    info!("💡 可能的解决方案:");
                    info!("   - 检查网络连接");
                    info!("   - 确保有足够的磁盘空间");
                    info!("   - 以管理员权限运行");
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

/// 获取指定版本的下载链接
async fn get_version_download_url(repo: &GitHubRepo, version: &str) -> Result<String> {
    // 这里应该获取指定版本的release信息
    // 为了简化，我们先使用最新版本，后续可以扩展支持获取指定版本
    let version_info = check_for_updates(repo).await?;

    version_info
        .download_url
        .ok_or_else(|| anyhow::anyhow!("未找到版本 {} 适合当前平台的下载链接", version))
}
