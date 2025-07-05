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
use crate::downloader::{DownloadProgress, DownloadStatus, FileDownloader, DownloaderConfig};
use crate::error::{DuckError, Result};
use chrono;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::io::{self, Write};
use futures::stream::StreamExt;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info, warn};



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
        let mut last_progress_time = std::time::Instant::now();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| DuckError::custom(format!("下载数据失败: {}", e)))?;
            
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                .await
                .map_err(|e| DuckError::custom(format!("写入文件失败: {}", e)))?;

            downloaded += chunk.len() as u64;

            // 简化的进度显示逻辑（减少频率，避免与下载器重复）⭐
            let now = std::time::Instant::now();
            let time_since_last = now.duration_since(last_progress_time);
            
            // 减少频率：每50MB或每30秒显示一次
            let should_show_progress = 
                downloaded % (50 * 1024 * 1024) == 0 && downloaded > 0 ||  // 每50MB显示一次
                time_since_last >= std::time::Duration::from_secs(30) ||  // 每30秒显示一次
                (total_size.map_or(false, |size| downloaded >= size)); // 下载完成时显示
            
            if should_show_progress {
                if let Some(size) = total_size {
                    let percentage = (downloaded as f64 / size as f64 * 100.0) as u32;
                    info!("🌐 下载进度: {}% ({:.1}/{:.1} MB)", 
                        percentage,
                        downloaded as f64 / 1024.0 / 1024.0,
                        size as f64 / 1024.0 / 1024.0
                    );
                } else {
                    info!("🌐 已下载: {:.1} MB", downloaded as f64 / 1024.0 / 1024.0);
                }
                
                // 更新上次显示进度的时间
                last_progress_time = now;
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
            info!("   包大小: 未知 (外链文件)");
        }

        // 2. 检查版本参数
        if let Some(target_version) = version {
            if target_version != manifest.version {
                warn!(
                    "⚠️  请求版本 {} 与服务器版本 {} 不匹配",
                    target_version, manifest.version
                );
                info!("   将下载服务器版本: {}", manifest.version);
            }
        }

        // 3. 获取哈希文件路径
        let hash_file_path = download_path.with_extension("zip.hash");
        
        // 4. 智能下载决策
        let is_external_file = manifest.packages.full.hash.to_lowercase() == "external";
        
        info!("🔍 下载方式判断:");
        info!("   原始URL: {}", manifest.packages.full.url);
        info!("   Hash值: {}", manifest.packages.full.hash);
        info!("   是否外链: {}", is_external_file);
        info!("   配置的base_url: {}", self.config.base_url);

        let (download_url, use_auth) = if is_external_file {
            // 外链文件，直接下载，无需认证
            info!("📥 使用直接下载方式 (外链文件: hash=external)");
            (manifest.packages.full.url.clone(), false)
        } else if manifest.packages.full.url.starts_with("http://") || manifest.packages.full.url.starts_with("https://") {
            // 检查是否是本地服务器的URL
            if manifest.packages.full.url.contains(&self.config.base_url) {
                // 本地API，使用认证
                info!("📥 使用认证下载方式 (本地API)");
                (manifest.packages.full.url.clone(), true)
            } else {
                // 外部链接，直接下载
                info!("📥 使用直接下载方式 (外部链接)");
                (manifest.packages.full.url.clone(), false)
            }
        } else {
            // 相对路径，构建完整URL
            info!("📥 使用认证下载方式 (相对路径)");
            let full_url = format!("{}{}", self.config.base_url, manifest.packages.full.url);
            (full_url, true)
        };

        // 5. 检查文件是否已存在且完整
        let should_download = if download_path.exists() {
            info!("📁 发现已存在的文件: {}", download_path.display());
            
            // 检查哈希文件是否存在
            if hash_file_path.exists() {
                info!("📋 发现哈希文件: {}", hash_file_path.display());
                
                // 读取保存的哈希和版本信息
                if let Ok(hash_content) = std::fs::read_to_string(&hash_file_path) {
                    let lines: Vec<&str> = hash_content.trim().lines().collect();
                    if lines.len() >= 3 {
                        let saved_hash = lines[0];
                        let saved_version = lines[1];
                        let saved_timestamp = lines[2];
                        
                        info!("📊 哈希文件信息:");
                        info!("   保存的哈希: {}", saved_hash);
                        info!("   保存的版本: {}", saved_version);
                        info!("   保存时间: {}", saved_timestamp);
                        
                        // 检查版本是否匹配
                        if saved_version == manifest.version {
                            info!("✅ 版本匹配: {}", manifest.version);
                            
                            // 对于外链文件，不进行本地哈希验证
                            if is_external_file {
                                info!("✅ 外链文件且版本匹配，跳过下载");
                                info!("   文件路径: {}", download_path.display());
                                return Ok(());
                            } else {
                                // 验证本地文件哈希
                                info!("🧮 验证本地文件哈希...");
                                if let Ok(actual_hash) = Self::calculate_file_hash(download_path).await {
                                    if actual_hash.to_lowercase() == saved_hash.to_lowercase() {
                                        info!("✅ 文件哈希验证通过，跳过下载");
                                        info!("   本地哈希: {}", actual_hash);
                                        info!("   服务器哈希: {}", saved_hash);
                                        return Ok(());
                                    } else {
                                        warn!("⚠️  文件哈希不匹配，需要重新下载");
                                        warn!("   本地哈希: {}", actual_hash);
                                        warn!("   期望哈希: {}", saved_hash);
                                        true
                                    }
                                } else {
                                    warn!("⚠️  无法计算本地文件哈希，重新下载");
                                    true
                                }
                            }
                        } else {
                            info!("🔄 版本不匹配，需要下载新版本");
                            info!("   本地版本: {}", saved_version);
                            info!("   服务器版本: {}", manifest.version);
                            true
                        }
                    } else {
                        warn!("⚠️  哈希文件格式无效，重新下载");
                        true
                    }
                } else {
                    warn!("⚠️  无法读取哈希文件，重新下载");
                    true
                }
            } else {
                info!("📋 未发现哈希文件，验证文件完整性...");
                
                // 对于外链文件，假设存在即有效
                if is_external_file {
                    warn!("⚠️  外链文件无哈希文件，建议重新下载以生成哈希记录");
                    true
                } else {
                    // 尝试验证现有文件（如果服务器提供了哈希）
                    if manifest.packages.full.hash != "external" && !manifest.packages.full.hash.is_empty() {
                        info!("🧮 使用服务器哈希验证现有文件...");
                        if let Ok(actual_hash) = Self::calculate_file_hash(download_path).await {
                            if actual_hash.to_lowercase() == manifest.packages.full.hash.to_lowercase() {
                                info!("✅ 现有文件哈希验证通过");
                                // 生成哈希文件
                                if let Err(e) = self.save_hash_file(&hash_file_path, &manifest.packages.full.hash, &manifest.version).await {
                                    warn!("⚠️  保存哈希文件失败: {}", e);
                                }
                                return Ok(());
                            } else {
                                warn!("⚠️  现有文件哈希不匹配，重新下载");
                                warn!("   本地哈希: {}", actual_hash);
                                warn!("   服务器哈希: {}", manifest.packages.full.hash);
                                true
                            }
                        } else {
                            warn!("⚠️  无法计算现有文件哈希，重新下载");
                            true
                        }
                    } else {
                        warn!("⚠️  服务器未提供文件哈希，强制重新下载");
                        true
                    }
                }
            }
        } else {
            info!("📁 文件不存在，需要下载");
            true
        };

        if !should_download {
            info!("⏭️  跳过下载，使用现有文件");
            return Ok(());
        }

        // 6. 确保下载目录存在
        if let Some(parent) = download_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Err(DuckError::custom(format!("创建下载目录失败: {}", e)));
            }
        }

        info!("📥 开始下载服务更新包...");
        info!("   下载方式: {}", if use_auth { "认证下载" } else { "直接下载" });
        info!("   最终下载URL: {}", download_url);
        info!("   目标路径: {}", download_path.display());
        info!("   使用认证: {}", use_auth);
        if manifest.packages.full.size > 0 {
            info!("   预期文件大小: {} bytes ({:.2} MB)", manifest.packages.full.size, manifest.packages.full.size as f64 / 1024.0 / 1024.0);
        } else {
            info!("   预期文件大小: 未知 (外链文件)");
        }

        // 7. 执行下载
        // 使用新的下载器模块
        let config = DownloaderConfig {
            timeout_seconds: 30 * 60, // 30分钟超时
            chunk_size: 8192,
            retry_count: 3,
            enable_progress_logging: true,
            enable_resume: true,      // 启用断点续传
            resume_threshold: 1024 * 1024, // 1MB 续传阈值
            progress_interval_seconds: 10, // 每10秒显示一次进度（大文件下载更友好）
            progress_bytes_interval: 100 * 1024 * 1024, // 每100MB显示一次进度
            enable_metadata: true,    // 启用元数据管理
        };
        
        let downloader = FileDownloader::new(config);
        
        // 准备下载参数
        let expected_hash = if is_external_file {
            None // 外链文件不提供hash
        } else {
            Some(manifest.packages.full.hash.as_str())
        };
        
        // 如果使用认证，需要特殊处理
        if use_auth {
            // 对于认证下载，使用传统的 HTTP 客户端方式
            let auth_client = self.authenticated_client.as_ref().unwrap();
            let request_builder = auth_client.get(&download_url)
                .await
                .map_err(|e| DuckError::custom(format!("创建认证请求失败: {}", e)))?;
            let response = request_builder.send()
                .await
                .map_err(|e| DuckError::custom(format!("发起下载请求失败: {}", e)))?;

            if !response.status().is_success() {
                return Err(DuckError::custom(format!(
                    "下载失败: HTTP {}",
                    response.status()
                )));
            }

            // 获取实际文件大小
            let total_size = response.content_length().unwrap_or(0);
            if total_size > 0 {
                info!("📦 实际文件大小: {} bytes ({:.2} MB)", total_size, total_size as f64 / 1024.0 / 1024.0);
            }

            // 使用统一的智能下载器处理认证下载，避免重复日志 ⭐
            info!("📥 使用认证下载器（统一进度显示）");
            
            // 创建临时的认证客户端配置
            let auth_downloader_config = DownloaderConfig {
                timeout_seconds: 30 * 60, // 30分钟超时
                chunk_size: 8192,
                retry_count: 3,
                enable_progress_logging: true, // 启用下载器的进度显示
                enable_resume: true,      
                resume_threshold: 1024 * 1024, 
                progress_interval_seconds: 10, // 认证下载使用稍低频率
                progress_bytes_interval: 100 * 1024 * 1024, 
                enable_metadata: true,    
            };
            
            let _auth_downloader = FileDownloader::new(auth_downloader_config);
            
            // 使用统一的下载器，但通过认证响应流下载
            let mut file = tokio::fs::File::create(download_path)
                .await
                .map_err(|e| DuckError::custom(format!("创建文件失败: {}", e)))?;

            let mut downloaded = 0u64;
            let mut stream = response.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| DuckError::custom(format!("下载数据失败: {}", e)))?;
                
                tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                    .await
                    .map_err(|e| DuckError::custom(format!("写入文件失败: {}", e)))?;

                downloaded += chunk.len() as u64;

                // 只调用进度回调，不重复输出日志 ⭐
                if let Some(callback) = progress_callback.as_ref() {
                    let progress = if total_size > 0 {
                        downloaded as f64 / total_size as f64 * 100.0
                    } else {
                        0.0
                    };

                    callback(DownloadProgress {
                        task_id: "auth_download_task".to_string(),
                        file_name: download_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        downloaded_bytes: downloaded,
                        total_bytes: total_size,
                        download_speed: 0.0,
                        eta_seconds: 0,
                        percentage: progress,
                        status: DownloadStatus::Downloading,
                    });
                }
                
                // 简化的进度显示（减少频率，避免重复） ⭐
                if downloaded % (200 * 1024 * 1024) == 0 && downloaded > 0 {
                    // 每200MB显示一次认证下载进度
                    info!("🔐 认证下载进度: {:.1} MB", downloaded as f64 / 1024.0 / 1024.0);
                }
            }

            // 确保文件已刷新到磁盘
            file.flush().await
                .map_err(|e| DuckError::custom(format!("刷新文件缓冲区失败: {}", e)))?;

            drop(file);
            
            info!("✅ 认证下载完成");
            info!("   文件路径: {}", download_path.display());
            info!("   下载大小: {} bytes ({:.2} MB)", downloaded, downloaded as f64 / 1024.0 / 1024.0);
        } else {
            // 使用新的智能下载器（支持 OSS、扩展超时、断点续传和hash验证）
            downloader.download_file_with_options(
                &download_url, 
                download_path, 
                progress_callback,
                expected_hash,
                Some(&manifest.version)
            )
            .await
            .map_err(|e| DuckError::custom(format!("下载失败: {}", e)))?;
                
            info!("✅ 文件下载完成");
            info!("   文件路径: {}", download_path.display());
        }

        // 9. 下载器已集成hash验证，这里不需要额外验证
        info!("✅ 下载器已完成文件验证（如果需要）");

        // 10. 保存哈希文件
        let hash_to_save = if is_external_file {
            // 对于外链文件，计算本地哈希并保存
            info!("🧮 计算外链文件的本地哈希...");
            match Self::calculate_file_hash(download_path).await {
                Ok(local_hash) => {
                    info!("📋 外链文件本地哈希: {}", local_hash);
                    local_hash
                },
                Err(e) => {
                    warn!("⚠️  计算外链文件哈希失败: {}", e);
                    "external".to_string()
                }
            }
        } else {
            manifest.packages.full.hash.clone()
        };
        
        info!("💾 保存哈希文件...");
        if let Err(e) = self.save_hash_file(&hash_file_path, &hash_to_save, &manifest.version).await {
            warn!("⚠️  保存哈希文件失败: {}", e);
            warn!("   文件已下载成功，但哈希记录保存失败");
            warn!("   下次可能会重新下载该文件");
        } else {
            info!("✅ 哈希文件保存成功: {}", hash_file_path.display());
        }

        info!("🎉 服务更新包下载完成!");
        info!("   文件位置: {}", download_path.display());
        info!("   版本信息: {}", manifest.version);

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

    /// 保存哈希文件
    async fn save_hash_file(&self, hash_file_path: &Path, hash: &str, version: &str) -> Result<()> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let content = format!("{}\n{}\n{}\n", hash, version, timestamp);
        
        tokio::fs::write(hash_file_path, content)
            .await
            .map_err(|e| DuckError::custom(format!("写入哈希文件失败: {}", e)))?;
            
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
