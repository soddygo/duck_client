//! # API客户端模块
//!
//! 提供与后端服务通信的统一接口，包括：
//! - 客户端注册与认证
//! - 版本检查与更新
//! - 服务包下载与管理  
//! - 遥测数据上报
//! - 文件完整性验证
//!
//! ## 智能下载系统
//!
//! 本模块实现了一个智能的文件下载和缓存系统：
//!
//! ### 缓存路径结构
//! ```
//! cacheDuckData/download/{版本号}/full/docker.zip
//! cacheDuckData/download/{版本号}/full/docker.zip.hash
//! ```
//!
//! ### 智能下载流程
//! 1. **获取服务清单**：从服务器获取最新版本信息和文件哈希
//! 2. **版本检查**：比较请求版本与服务器最新版本
//! 3. **本地文件检查**：
//!    - 文件不存在 → 需要下载
//!    - 文件存在 → 进入哈希验证流程
//! 4. **哈希验证流程**：
//!    - 读取本地保存的哈希值（.hash文件）
//!    - 比较本地哈希与远程哈希
//!    - 哈希相同 → 验证文件完整性
//!    - 哈希不同 → 需要下载新版本
//! 5. **文件完整性验证**：
//!    - 计算文件实际哈希值
//!    - 与预期哈希值比较
//!    - 完整性验证通过 → 跳过下载
//!    - 完整性验证失败 → 文件损坏，重新下载
//! 6. **下载执行**：
//!    - 下载新文件或替换损坏文件
//!    - 验证下载文件的完整性
//!    - 保存哈希值到 .hash 文件
//!
//! ### 优势
//! - **避免重复下载**：相同版本且文件完整时跳过下载
//! - **自动修复**：检测并修复损坏的缓存文件
//! - **版本管理**：支持多版本并存的缓存管理
//! - **完整性保证**：SHA-256哈希验证确保文件完整性
//!
//! ### 使用示例
//! ```rust
//! let api_client = ApiClient::new(Some("client_id".to_string()));
//!
//! // 智能下载（自动处理缓存和版本检查）
//! api_client.download_service_update_optimized(
//!     &Path::new("cacheDuckData/download/0.0.2/full/docker.zip"),
//!     Some("0.0.2")
//! ).await?;
//! ```

use crate::api_config::ApiConfig;
use crate::authenticated_client::AuthenticatedClient;
use crate::error::{DuckError, Result};
use chrono;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info, warn};

/// 下载进度状态枚举
#[derive(Debug, Clone)]
pub enum DownloadStatus {
    Starting,
    Downloading,
    Paused,
    Completed,
    Failed(String),
}

/// 下载进度信息
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub task_id: String,
    pub file_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub download_speed: f64, // bytes/sec
    pub eta_seconds: u64,
    pub percentage: f64,
    pub status: DownloadStatus,
}

/// API 客户端
#[derive(Debug, Clone)]
pub struct ApiClient {
    client: Client,
    config: ApiConfig,
    client_id: Option<String>,
    authenticated_client: Option<AuthenticatedClient>,
}

/// 客户端注册请求
#[derive(Debug, Serialize)]
pub struct ClientRegisterRequest {
    pub os: String,
    pub arch: String,
}

/// 注册客户端响应
#[derive(Debug, Deserialize)]
pub struct RegisterClientResponse {
    client_id: String,
}

/// 服务更新清单响应
#[derive(Debug, Deserialize)]
pub struct ServiceManifest {
    pub version: String,
    pub release_date: String,
    pub release_notes: String,
    pub packages: ServicePackages,
}

/// 服务包信息
#[derive(Debug, Deserialize)]
pub struct ServicePackages {
    pub full: PackageInfo,
    pub patch: Option<PackageInfo>,
}

/// 包信息
#[derive(Debug, Deserialize)]
pub struct PackageInfo {
    pub url: String,
    pub hash: String,
    pub signature: String,
    pub size: u64,
}

/// 客户端更新清单响应
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ClientManifest {
    pub version: String,
    pub notes: String,
    pub pub_date: String,
    pub platforms: HashMap<String, PlatformInfo>,
}

/// 客户端平台信息
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PlatformInfo {
    pub signature: String,
    pub url: String,
}

/// 服务升级历史上报请求
#[derive(Debug, Serialize)]
pub struct ServiceUpgradeReport {
    pub from_version: String,
    pub to_version: String,
    pub status: String,
    pub details: String,
}

/// 客户端自升级历史上报请求
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ClientUpgradeReport {
    pub from_version: String,
    pub to_version: String,
    pub status: String,
    pub details: String,
}

/// 公告信息
#[derive(Debug, Deserialize)]
pub struct Announcement {
    pub id: i64,
    pub level: String,
    pub content: String,
    pub created_at: String,
}

/// 公告列表响应
#[derive(Debug, Deserialize)]
pub struct AnnouncementsResponse {
    pub announcements: Vec<Announcement>,
}

/// Docker版本检查响应
#[derive(Deserialize, Debug)]
pub struct DockerVersionResponse {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_notes: Option<String>,
}

/// Docker版本列表响应
#[derive(Deserialize, Debug)]
pub struct DockerVersionListResponse {
    pub versions: Vec<DockerVersion>,
}

/// Docker版本信息
#[derive(Deserialize, Debug)]
pub struct DockerVersion {
    pub version: String,
    pub release_date: String,
    pub notes: String,
    pub is_latest: bool,
}

/// 服务升级历史上报请求
#[derive(Serialize)]
pub struct ServiceUpgradeHistoryRequest {
    pub service_name: String,
    pub from_version: String,
    pub to_version: String,
    pub status: String,
    pub details: Option<String>,
}

/// 客户端自升级历史上报请求
#[derive(Serialize)]
pub struct ClientSelfUpgradeHistoryRequest {
    pub from_version: String,
    pub to_version: String,
    pub status: String,
    pub details: Option<String>,
}

/// 遥测数据上报请求
#[derive(Serialize)]
pub struct TelemetryRequest {
    pub event_type: String,
    pub data: serde_json::Value,
}

impl ApiClient {
    /// 创建新的 API 客户端
    pub fn new(client_id: Option<String>) -> Self {
        Self {
            client: Client::new(),
            config: ApiConfig::default(),
            client_id,
            authenticated_client: None,
        }
    }

    /// 设置客户端ID
    pub fn set_client_id(&mut self, client_id: String) {
        self.client_id = Some(client_id);
    }

    /// 设置认证客户端
    pub fn set_authenticated_client(&mut self, authenticated_client: AuthenticatedClient) {
        self.authenticated_client = Some(authenticated_client);
    }

    /// 获取当前API配置
    pub fn get_config(&self) -> &ApiConfig {
        &self.config
    }

    /// 构建带客户端ID的请求
    fn build_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.client.get(url);
        if let Some(ref client_id) = self.client_id {
            request = request.header("X-Client-ID", client_id);
        }
        request
    }

    /// 构建POST请求
    fn build_post_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.client.post(url);
        if let Some(ref client_id) = self.client_id {
            request = request.header("X-Client-ID", client_id);
        }
        request
    }

    /// 注册客户端
    pub async fn register_client(&self, request: ClientRegisterRequest) -> Result<String> {
        let url = self
            .config
            .get_endpoint_url(&self.config.endpoints.client_register);

        let response = self.client.post(&url).json(&request).send().await?;

        if response.status().is_success() {
            let register_response: RegisterClientResponse = response.json().await?;
            info!(
                "客户端注册成功，获得客户端ID: {}",
                register_response.client_id
            );
            Ok(register_response.client_id)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("客户端注册失败: {} - {}", status, text);
            Err(DuckError::Api(format!("注册失败: {status} - {text}")))
        }
    }

    /// 获取系统公告
    pub async fn get_announcements(&self, since: Option<&str>) -> Result<AnnouncementsResponse> {
        let mut url = self
            .config
            .get_endpoint_url(&self.config.endpoints.announcements);

        if let Some(since_time) = since {
            url = format!("{url}?since={since_time}");
        }

        let response = self.build_request(&url).send().await?;

        if response.status().is_success() {
            let announcements = response.json().await?;
            Ok(announcements)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("获取公告失败: {} - {}", status, text);
            Err(DuckError::Api(format!("获取公告失败: {status} - {text}")))
        }
    }

    /// 检查Docker服务版本
    pub async fn check_docker_version(
        &self,
        current_version: &str,
    ) -> Result<DockerVersionResponse> {
        let url = self
            .config
            .get_endpoint_url(&self.config.endpoints.docker_check_version);

        let response = self.build_request(&url).send().await?;

        if response.status().is_success() {
            let manifest: ServiceManifest = response.json().await?;

            // 从ServiceManifest构造DockerVersionResponse
            let has_update = manifest.version != current_version;
            let docker_version_response = DockerVersionResponse {
                current_version: current_version.to_string(),
                latest_version: manifest.version,
                has_update,
                release_notes: Some(manifest.release_notes),
            };

            Ok(docker_version_response)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("检查Docker版本失败: {} - {}", status, text);
            Err(DuckError::Api(format!(
                "检查Docker版本失败: {status} - {text}"
            )))
        }
    }

    /// 获取Docker版本列表
    pub async fn get_docker_version_list(&self) -> Result<DockerVersionListResponse> {
        let url = self
            .config
            .get_endpoint_url(&self.config.endpoints.docker_update_version_list);

        let response = self.build_request(&url).send().await?;

        if response.status().is_success() {
            let version_list = response.json().await?;
            Ok(version_list)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("获取Docker版本列表失败: {} - {}", status, text);
            Err(DuckError::Api(format!(
                "获取Docker版本列表失败: {status} - {text}"
            )))
        }
    }

    /// 下载Docker服务更新包
    pub async fn download_service_update<P: AsRef<Path>>(&self, save_path: P) -> Result<()> {
        let url = self
            .config
            .get_endpoint_url(&self.config.endpoints.docker_download_full);

        self.download_service_update_from_url(&url, save_path).await
    }

    /// 从指定URL下载Docker服务更新包
    pub async fn download_service_update_from_url<P: AsRef<Path>>(
        &self,
        url: &str,
        save_path: P,
    ) -> Result<()> {
        self.download_service_update_from_url_with_auth(url, save_path, true)
            .await
    }

    /// 从指定URL下载Docker服务更新包（支持认证控制）
    pub async fn download_service_update_from_url_with_auth<P: AsRef<Path>>(
        &self,
        url: &str,
        save_path: P,
        use_auth: bool,
    ) -> Result<()> {
        info!("开始下载Docker服务更新包: {}", url);

        // 根据是否需要认证决定使用哪种客户端
        let response = if use_auth && self.authenticated_client.is_some() {
            // 使用认证客户端（API下载）
            let auth_client = self.authenticated_client.as_ref().unwrap();
            match auth_client.get(url).await {
                Ok(request_builder) => auth_client.send(request_builder, url).await?,
                Err(e) => {
                    warn!("使用AuthenticatedClient失败，回退到普通请求: {}", e);
                    self.build_request(url).send().await?
                }
            }
        } else {
            // 使用普通客户端（直接URL下载）
            info!("使用普通HTTP客户端下载");
            self.build_request(url).send().await?
        };

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("下载Docker服务更新包失败: {} - {}", status, text);
            return Err(DuckError::Api(format!("下载失败: {status} - {text}")));
        }

        // 获取文件大小
        let total_size = response.content_length();

        if let Some(size) = total_size {
            info!(
                "Docker服务更新包大小: {} bytes ({:.1} MB)",
                size,
                size as f64 / 1024.0 / 1024.0
            );
        }

        // 流式写入文件
        let mut file = File::create(&save_path).await?;
        let mut stream = response.bytes_stream();
        let mut downloaded = 0u64;
        let mut last_update = std::time::Instant::now();

        use futures::StreamExt;
        use std::io::{self, Write};

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            // 每500KB或每秒更新一次进度显示
            let should_update =
                downloaded % (512 * 1024) == 0 || last_update.elapsed().as_secs() >= 1;

            if should_update {
                if let Some(total) = total_size {
                    let percentage = (downloaded as f64 / total as f64) * 100.0;
                    let downloaded_mb = downloaded as f64 / 1024.0 / 1024.0;
                    let total_mb = total as f64 / 1024.0 / 1024.0;

                    // 创建简单的进度条
                    let bar_width = 30;
                    let filled = ((percentage / 100.0) * bar_width as f64) as usize;
                    let progress_bar = "█".repeat(filled) + &"░".repeat(bar_width - filled);

                    print!(
                        "\r📦 下载进度: [{progress_bar}] {percentage:.1}% ({downloaded_mb:.1}/{total_mb:.1} MB)"
                    );
                    io::stdout().flush().unwrap();

                    last_update = std::time::Instant::now();
                } else {
                    // 没有总大小信息时，只显示已下载量
                    let downloaded_mb = downloaded as f64 / 1024.0 / 1024.0;
                    print!("\r📦 下载进度: {downloaded_mb:.1} MB");
                    io::stdout().flush().unwrap();

                    last_update = std::time::Instant::now();
                }
            }
        }

        // 下载完成，强制显示100%进度条
        if let Some(total) = total_size {
            let downloaded_mb = downloaded as f64 / 1024.0 / 1024.0;
            let total_mb = total as f64 / 1024.0 / 1024.0;

            // 创建完整的进度条
            let bar_width = 30;
            let progress_bar = "█".repeat(bar_width);

            print!("\r📦 下载进度: [{progress_bar}] 100.0% ({downloaded_mb:.1}/{total_mb:.1} MB)");
            io::stdout().flush().unwrap();
        } else {
            // 没有总大小信息时，显示最终下载量
            let downloaded_mb = downloaded as f64 / 1024.0 / 1024.0;
            print!("\r📦 下载进度: {downloaded_mb:.1} MB (完成)");
            io::stdout().flush().unwrap();
        }

        // 下载完成，换行并显示完成信息
        println!(); // 换行
        file.flush().await?;
        info!("Docker服务更新包下载完成: {}", save_path.as_ref().display());
        Ok(())
    }

    /// 上报服务升级历史
    pub async fn report_service_upgrade_history(
        &self,
        request: ServiceUpgradeHistoryRequest,
    ) -> Result<()> {
        let url = self
            .config
            .get_service_upgrade_history_url(&request.service_name);

        let response = self.build_post_request(&url).json(&request).send().await?;

        if response.status().is_success() {
            info!("服务升级历史上报成功");
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            warn!("服务升级历史上报失败: {} - {}", status, text);
            // 上报失败不影响主流程，只记录警告
            Ok(())
        }
    }

    /// 上报客户端自升级历史
    pub async fn report_client_self_upgrade_history(
        &self,
        request: ClientSelfUpgradeHistoryRequest,
    ) -> Result<()> {
        let url = self
            .config
            .get_endpoint_url(&self.config.endpoints.client_self_upgrade_history);

        let response = self.build_post_request(&url).json(&request).send().await?;

        if response.status().is_success() {
            info!("客户端自升级历史上报成功");
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            warn!("客户端自升级历史上报失败: {} - {}", status, text);
            // 上报失败不影响主流程，只记录警告
            Ok(())
        }
    }

    /// 上报遥测数据
    pub async fn report_telemetry(&self, request: TelemetryRequest) -> Result<()> {
        let url = self
            .config
            .get_endpoint_url(&self.config.endpoints.telemetry);

        let response = self.build_post_request(&url).json(&request).send().await?;

        if response.status().is_success() {
            info!("遥测数据上报成功");
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            warn!("遥测数据上报失败: {} - {}", status, text);
            // 上报失败不影响主流程，只记录警告
            Ok(())
        }
    }

    /// 获取服务下载URL（用于配置显示）
    pub fn get_service_download_url(&self) -> String {
        self.config
            .get_endpoint_url(&self.config.endpoints.docker_download_full)
    }

    /// 计算文件的SHA256哈希值
    pub async fn calculate_file_hash(file_path: &Path) -> Result<String> {
        if !file_path.exists() {
            return Err(DuckError::Custom(format!(
                "文件不存在: {}",
                file_path.display()
            )));
        }

        let mut file = File::open(file_path).await.map_err(|e| {
            DuckError::Custom(format!("无法打开文件 {}: {}", file_path.display(), e))
        })?;

        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192]; // 8KB buffer

        loop {
            let bytes_read = file.read(&mut buffer).await.map_err(|e| {
                DuckError::Custom(format!("读取文件失败 {}: {}", file_path.display(), e))
            })?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize();
        Ok(format!("{hash:x}"))
    }

    /// 保存文件哈希信息到.hash文件
    pub async fn save_file_hash(file_path: &Path, hash: &str) -> Result<()> {
        let hash_file_path = file_path.with_extension("hash");
        let mut hash_file = File::create(&hash_file_path).await.map_err(|e| {
            DuckError::Custom(format!(
                "无法创建哈希文件 {}: {}",
                hash_file_path.display(),
                e
            ))
        })?;

        hash_file.write_all(hash.as_bytes()).await.map_err(|e| {
            DuckError::Custom(format!(
                "写入哈希文件失败 {}: {}",
                hash_file_path.display(),
                e
            ))
        })?;

        info!("已保存文件哈希: {}", hash_file_path.display());
        Ok(())
    }

    /// 从.hash文件读取哈希信息
    pub async fn load_file_hash(file_path: &Path) -> Result<Option<String>> {
        let hash_file_path = file_path.with_extension("hash");

        if !hash_file_path.exists() {
            return Ok(None);
        }

        let mut hash_file = File::open(&hash_file_path).await.map_err(|e| {
            DuckError::Custom(format!(
                "无法打开哈希文件 {}: {}",
                hash_file_path.display(),
                e
            ))
        })?;

        let mut hash_content = String::new();
        hash_file
            .read_to_string(&mut hash_content)
            .await
            .map_err(|e| {
                DuckError::Custom(format!(
                    "读取哈希文件失败 {}: {}",
                    hash_file_path.display(),
                    e
                ))
            })?;

        Ok(Some(hash_content.trim().to_string()))
    }

    /// 验证文件完整性
    pub async fn verify_file_integrity(file_path: &Path, expected_hash: &str) -> Result<bool> {
        info!("验证文件完整性: {}", file_path.display());

        // 计算当前文件的哈希值
        let actual_hash = Self::calculate_file_hash(file_path).await?;

        // 比较哈希值（忽略大小写）
        let matches = actual_hash.to_lowercase() == expected_hash.to_lowercase();

        if matches {
            info!("✅ 文件完整性验证通过: {}", file_path.display());
        } else {
            warn!("❌ 文件完整性验证失败: {}", file_path.display());
            warn!("   期望哈希: {}", expected_hash);
            warn!("   实际哈希: {}", actual_hash);
        }

        Ok(matches)
    }

    /// 检查文件是否需要下载（简化版本）
    pub async fn needs_file_download(&self, file_path: &Path, remote_hash: &str) -> Result<bool> {
        // 计算当前文件哈希值并比较
        match Self::calculate_file_hash(file_path).await {
            Ok(actual_hash) => {
                info!("🧮 计算出的文件哈希: {}", actual_hash);
                if actual_hash.to_lowercase() == remote_hash.to_lowercase() {
                    info!("✅ 文件哈希匹配，跳过下载");
                    Ok(false)
                } else {
                    info!("🔄 文件哈希不匹配，需要下载新版本");
                    info!("   本地哈希: {}", actual_hash);
                    info!("   远程哈希: {}", remote_hash);
                    Ok(true)
                }
            }
            Err(e) => {
                warn!("💥 计算文件哈希失败: {}，需要重新下载", e);
                Ok(true)
            }
        }
    }

    /// 检查文件是否需要下载（完整版本，包含哈希文件缓存）
    pub async fn should_download_file(&self, file_path: &Path, remote_hash: &str) -> Result<bool> {
        info!("🔍 开始智能下载决策检查...");
        info!("   目标文件: {}", file_path.display());
        info!("   远程哈希: {}", remote_hash);

        // 文件不存在，需要下载
        if !file_path.exists() {
            info!("📂 文件不存在，需要下载: {}", file_path.display());
            // 清理可能存在的哈希文件
            let hash_file_path = file_path.with_extension("hash");
            if hash_file_path.exists() {
                info!(
                    "🧹 发现孤立的哈希文件，正在清理: {}",
                    hash_file_path.display()
                );
                if let Err(e) = tokio::fs::remove_file(&hash_file_path).await {
                    warn!("⚠️ 清理哈希文件失败: {}", e);
                }
            }
            return Ok(true);
        }

        info!("🔍 检查本地文件: {}", file_path.display());

        // 检查文件大小
        match tokio::fs::metadata(file_path).await {
            Ok(metadata) => {
                let file_size = metadata.len();
                info!("📊 本地文件大小: {} bytes", file_size);
                if file_size == 0 {
                    warn!("⚠️ 本地文件大小为0，需要重新下载");
                    return Ok(true);
                }
            }
            Err(e) => {
                warn!("⚠️ 无法获取文件元数据: {}，需要重新下载", e);
                return Ok(true);
            }
        }

        // 尝试读取本地保存的哈希值
        if let Some(saved_hash) = Self::load_file_hash(file_path).await? {
            info!("📜 找到本地哈希记录: {}", saved_hash);
            info!("🌐 远程文件哈希值: {}", remote_hash);

            // 比较保存的哈希值与远程哈希值
            if saved_hash.to_lowercase() == remote_hash.to_lowercase() {
                info!("✅ 哈希值匹配，验证文件完整性...");
                // 再验证文件是否真的完整（防止文件被损坏）
                match Self::verify_file_integrity(file_path, &saved_hash).await {
                    Ok(true) => {
                        info!("🎯 文件已是最新且完整，跳过下载");
                        return Ok(false);
                    }
                    Ok(false) => {
                        warn!("💥 文件哈希记录正确但文件已损坏，需要重新下载");
                        return Ok(true);
                    }
                    Err(e) => {
                        warn!("💥 文件完整性验证出错: {}，需要重新下载", e);
                        return Ok(true);
                    }
                }
            } else {
                info!("🆕 检测到新版本，需要下载更新");
                info!("   本地哈希: {}", saved_hash);
                info!("   远程哈希: {}", remote_hash);
                return Ok(true);
            }
        }

        // 没有哈希文件，计算当前文件哈希值并比较
        info!("📝 未找到哈希记录，计算当前文件哈希值...");
        match Self::calculate_file_hash(file_path).await {
            Ok(actual_hash) => {
                info!("🧮 计算出的文件哈希: {}", actual_hash);

                if actual_hash.to_lowercase() == remote_hash.to_lowercase() {
                    // 文件匹配，保存哈希值以供下次使用
                    if let Err(e) = Self::save_file_hash(file_path, &actual_hash).await {
                        warn!("⚠️ 保存哈希文件失败: {}", e);
                    }
                    info!("💾 文件与远程匹配，已保存哈希记录，跳过下载");
                    Ok(false)
                } else {
                    info!("🔄 文件与远程不匹配，需要下载新版本");
                    info!("   本地哈希: {}", actual_hash);
                    info!("   远程哈希: {}", remote_hash);
                    Ok(true)
                }
            }
            Err(e) => {
                warn!("💥 计算文件哈希失败: {}，需要重新下载", e);
                Ok(true)
            }
        }
    }

    /// 获取Docker服务版本信息和包信息
    pub async fn get_docker_service_manifest(&self) -> Result<ServiceManifest> {
        let url = self
            .config
            .get_endpoint_url(&self.config.endpoints.docker_check_version);

        let response = self.build_request(&url).send().await?;

        if response.status().is_success() {
            let manifest: ServiceManifest = response.json().await?;
            Ok(manifest)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("获取Docker服务清单失败: {} - {}", status, text);
            Err(DuckError::Api(format!(
                "获取Docker服务清单失败: {status} - {text}"
            )))
        }
    }

    /// 下载服务更新包（带哈希验证和优化及进度回调）
    pub async fn download_service_update_optimized_with_progress<F>(
        &self,
        download_path: &Path,
        version: Option<&str>,
        progress_callback: Option<F>,
    ) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        // 1. 获取服务清单信息
        info!("🔍 获取服务版本信息...");
        let manifest = self.get_docker_service_manifest().await?;

        info!("📋 服务清单信息:");
        info!("   版本: {}", manifest.version);
        info!("   发布日期: {}", manifest.release_date);
        info!("   包URL: {}", manifest.packages.full.url);
        info!("   包哈希: {}", manifest.packages.full.hash);
        if manifest.packages.full.size > 0 {
            info!(
                "   包大小: {} bytes ({:.2} MB)",
                manifest.packages.full.size,
                manifest.packages.full.size as f64 / 1024.0 / 1024.0
            );
        } else {
            info!("   包大小: 未提供 (外链文件)");
        }

        // 2. 检查版本参数
        if let Some(target_version) = version {
            if target_version != manifest.version {
                warn!(
                    "⚠️ 请求版本 {} 与服务器最新版本 {} 不匹配",
                    target_version, manifest.version
                );
                info!("   将下载服务器最新版本: {}", manifest.version);
            } else {
                info!("✅ 请求版本与服务器版本一致: {}", target_version);
            }
        }

        // 3. 检查是否为外链文件（hash为"external"）
        let is_external_file = manifest.packages.full.hash.to_lowercase() == "external";

        info!("🔍 下载方式判断:");
        info!("   原始URL: {}", manifest.packages.full.url);
        info!("   Hash值: {}", manifest.packages.full.hash);
        info!("   是否外链: {}", is_external_file);
        info!("   配置的base_url: {}", self.config.base_url);

        if is_external_file {
            info!("📦 检测到外链文件，跳过本地文件验证");
            // 外链文件始终需要下载，不进行本地文件检查
        } else {
            // 内部文件，进行常规的文件大小和哈希验证
            if download_path.exists() {
                if let Ok(metadata) = std::fs::metadata(download_path) {
                    let file_size = metadata.len();
                    if manifest.packages.full.size > 0 && file_size == manifest.packages.full.size {
                        info!("📦 文件已存在且大小匹配，开始哈希验证...");

                        // 进行哈希验证
                        let needs_download = self
                            .needs_file_download(download_path, &manifest.packages.full.hash)
                            .await?;

                        if !needs_download {
                            info!("✅ 文件已存在且验证通过，跳过下载");
                            return Ok(());
                        }
                    } else {
                        info!(
                            "📦 文件已存在但大小不匹配: {} != {}, 需要重新下载",
                            file_size, manifest.packages.full.size
                        );
                    }
                }
            }
        }

        // 4. 确保下载目录存在
        if let Some(parent_dir) = download_path.parent() {
            std::fs::create_dir_all(parent_dir)?;
        }

        // 5. 根据hash字段智能判断下载方式
        let (download_url, use_auth) = if is_external_file {
            // 外链文件，直接使用URL下载
            info!("📥 使用直接下载方式 (外链文件: hash=external)");
            (manifest.packages.full.url.clone(), false)
        } else if manifest.packages.full.url.starts_with("http://")
            || manifest.packages.full.url.starts_with("https://")
        {
            // 完整URL，检查是否是本地服务器
            if manifest
                .packages
                .full
                .url
                .starts_with(&self.config.base_url)
            {
                // 是本地服务器的URL，使用API模式
                info!("📥 使用API接口下载方式 (本地服务器URL)");
                let mut url = manifest.packages.full.url.clone();
                if let Some(v) = version {
                    url = format!("{url}?version={v}");
                    info!("   添加版本参数后的URL: {}", url);
                }
                (url, true)
            } else {
                // 外部URL，使用直接下载
                info!("📥 使用直接下载方式 (外部URL)");
                (manifest.packages.full.url.clone(), false)
            }
        } else {
            // 相对路径，使用API接口下载
            info!("📥 使用API接口下载方式 (相对路径)");
            let mut url = self
                .config
                .get_endpoint_url(&self.config.endpoints.docker_download_full);

            info!("   构建的API接口URL: {}", url);
            if let Some(v) = version {
                url = format!("{url}?version={v}");
                info!("   添加版本参数后的URL: {}", url);
            }
            (url, true)
        };

        info!("📥 开始下载服务更新包...");
        info!(
            "   下载方式: {}",
            if use_auth {
                "API接口"
            } else {
                "直接下载"
            }
        );
        info!("   最终下载URL: {}", download_url);
        info!("   目标路径: {}", download_path.display());
        info!("   使用认证: {}", use_auth);
        if manifest.packages.full.size > 0 {
            info!("   预期文件大小: {} bytes", manifest.packages.full.size);
        } else {
            info!("   预期文件大小: 未知 (外链文件)");
        }

        // 6. 执行下载
        if let Some(callback) = progress_callback {
            // 使用带进度回调的下载
            info!("🚀 开始带进度的下载...");
            self.download_with_progress_internal(&download_url, download_path, callback, use_auth)
                .await?;
        } else {
            // 使用普通下载方法
            info!("🚀 开始普通下载...");
            self.download_service_update_from_url_with_auth(&download_url, download_path, use_auth)
                .await?;
        }

        // 7. 下载完成后，对于外链文件跳过哈希验证
        if is_external_file {
            info!("📦 外链文件下载完成，跳过哈希验证");
        } else {
            info!("📦 内部文件下载完成，可以进行哈希验证");
        }

        info!("🎉 服务更新包下载完成!");
        Ok(())
    }

    /// 下载服务更新包（带哈希验证和优化）- 保持向后兼容
    pub async fn download_service_update_optimized(
        &self,
        download_path: &Path,
        version: Option<&str>,
    ) -> Result<()> {
        self.download_service_update_optimized_with_progress::<fn(DownloadProgress)>(
            download_path,
            version,
            None,
        )
        .await
    }

    /// 带进度回调的下载函数
    pub async fn download_with_progress<F>(
        &self,
        url: &str,
        target_path: &Path,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        self.download_with_progress_internal(url, target_path, progress_callback, true)
            .await
    }

    /// 带进度回调的下载函数（内部实现，支持是否使用认证）
    async fn download_with_progress_internal<F>(
        &self,
        url: &str,
        target_path: &Path,
        progress_callback: F,
        use_auth: bool,
    ) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        let callback = Arc::new(progress_callback);

        // 解析文件名
        let file_name = target_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let task_id = format!("download_{}", chrono::Utc::now().timestamp());

        // 开始下载进度报告
        let mut progress = DownloadProgress {
            task_id: task_id.clone(),
            file_name: file_name.clone(),
            downloaded_bytes: 0,
            total_bytes: 0,
            download_speed: 0.0,
            eta_seconds: 0,
            percentage: 0.0,
            status: DownloadStatus::Starting,
        };

        callback(progress.clone());

        info!("🔍 开始下载: {}", url);

        // 开始下载 - 根据是否需要认证决定使用哪种客户端
        let mut response = if use_auth && self.authenticated_client.is_some() {
            // 使用认证客户端（API下载）
            let auth_client = self.authenticated_client.as_ref().unwrap();
            match auth_client.get(url).await {
                Ok(request_builder) => auth_client.send(request_builder, url).await?,
                Err(e) => {
                    warn!("使用AuthenticatedClient下载失败，回退到普通请求: {}", e);
                    self.build_request(url)
                        .send()
                        .await
                        .map_err(|e| DuckError::Api(format!("开始下载失败: {}", e)))?
                }
            }
        } else {
            // 使用普通客户端（直接URL下载）
            info!("使用普通HTTP客户端下载");
            self.build_request(url)
                .send()
                .await
                .map_err(|e| DuckError::Api(format!("开始下载失败: {}", e)))?
        };

        // 检查GET请求状态
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(DuckError::Api(format!(
                "下载失败: HTTP {status} - {error_text}",
            )));
        }

        info!("✅ 下载响应成功，开始接收数据...");

        // 从响应中获取文件大小
        let total_size = response.content_length().unwrap_or(0);
        info!(
            "📊 文件大小: {} bytes ({:.2} MB)",
            total_size,
            total_size as f64 / 1024.0 / 1024.0
        );

        progress.total_bytes = total_size;
        progress.status = DownloadStatus::Downloading;
        callback(progress.clone());

        // 确保目标目录存在
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| DuckError::Custom(format!("创建目录失败: {}", e)))?;
        }

        let mut file = tokio::fs::File::create(target_path)
            .await
            .map_err(|e| DuckError::Custom(format!("创建文件失败: {}", e)))?;
        let mut downloaded = 0u64;
        let start_time = std::time::Instant::now();
        let mut last_update = start_time;

        info!("💾 开始写入文件: {}", target_path.display());

        // 流式下载
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| DuckError::Api(format!("下载数据失败: {}", e)))?
        {
            file.write_all(&chunk)
                .await
                .map_err(|e| DuckError::Custom(format!("写入文件失败: {}", e)))?;
            downloaded += chunk.len() as u64;

            let now = std::time::Instant::now();

            // 每500ms更新一次进度
            if now.duration_since(last_update).as_millis() > 500 {
                let elapsed = now.duration_since(start_time).as_secs_f64();
                let speed = if elapsed > 0.0 {
                    downloaded as f64 / elapsed
                } else {
                    0.0
                };
                let eta = if speed > 0.0 {
                    ((total_size - downloaded) as f64 / speed) as u64
                } else {
                    0
                };

                progress.downloaded_bytes = downloaded;
                progress.download_speed = speed;
                progress.eta_seconds = eta;
                progress.percentage = if total_size > 0 {
                    (downloaded as f64 / total_size as f64) * 100.0
                } else {
                    0.0
                };

                callback(progress.clone());
                last_update = now;
            }
        }

        // 确保文件被刷新到磁盘
        file.flush()
            .await
            .map_err(|e| DuckError::Custom(format!("刷新文件失败: {}", e)))?;

        info!("📊 下载完成统计:");
        info!(
            "   实际下载: {} bytes ({:.2} MB)",
            downloaded,
            downloaded as f64 / 1024.0 / 1024.0
        );
        info!(
            "   预期大小: {} bytes ({:.2} MB)",
            total_size,
            total_size as f64 / 1024.0 / 1024.0
        );

        // 验证下载是否完整
        if total_size > 0 && downloaded != total_size {
            let error_msg = format!(
                "下载不完整: 预期 {} bytes ({:.2} MB)，实际下载 {} bytes ({:.2} MB)",
                total_size,
                total_size as f64 / 1024.0 / 1024.0,
                downloaded,
                downloaded as f64 / 1024.0 / 1024.0
            );
            error!("{}", error_msg);
            return Err(DuckError::Custom(error_msg));
        }

        info!("✅ 文件下载完成: {} bytes", downloaded);

        // 完成下载
        progress.downloaded_bytes = downloaded;
        progress.percentage = 100.0;
        progress.status = DownloadStatus::Completed;
        callback(progress);

        Ok(())
    }
}

/// 系统信息模块
/// 用于获取操作系统类型和版本等信息
#[allow(dead_code)]
pub mod system_info {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Info {
        os_type: String,
        version: String,
    }

    impl Info {
        pub fn os_type(&self) -> &str {
            &self.os_type
        }
        pub fn version(&self) -> &str {
            &self.version
        }
    }

    pub fn get() -> Info {
        Info {
            os_type: std::env::consts::OS.to_string(),
            version: std::env::consts::ARCH.to_string(),
        }
    }
}
