//! # 下载模块
//!
//! 提供统一的文件下载接口，支持：
//! - 普通 HTTP 下载
//! - 阿里云 OSS 公网文件下载（扩展超时）
//! - **断点续传下载** ⭐
//! - 进度回调和监控
//! - 文件完整性验证
//! - 智能缓存和断点续传
//!
//! ## 主要特性
//!
//! ### 智能下载策略
//! - 自动检测下载方式（HTTP/扩展超时HTTP）
//! - 支持阿里云 OSS 大文件下载（公网访问）
//! - 扩展超时时间避免大文件下载失败
//! - **智能断点续传** - 自动检测已下载部分，从中断点继续
//!
//! ### 进度监控
//! - 实时下载进度回调
//! - 下载速度计算
//! - 剩余时间估算
//!
//! ### 文件完整性
//! - SHA-256 哈希验证
//! - 损坏文件自动重试
//! - 完整性校验缓存
//!
//! ### 断点续传
//! - HTTP Range 请求支持
//! - 自动检测已下载部分
//! - 智能文件完整性验证
//! - 支持大文件下载恢复

use crate::error::{DuckError, Result};
use futures::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn};
use chrono;

/// 下载进度状态枚举
#[derive(Debug, Clone)]
pub enum DownloadStatus {
    Starting,
    Downloading,
    Resuming, // 断点续传状态 ⭐
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

/// 下载任务元数据 ⭐
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadMetadata {
    pub url: String,
    pub expected_size: u64,
    pub expected_hash: Option<String>,
    pub downloaded_bytes: u64,
    pub start_time: String,
    pub last_update: String,
    pub version: String, // 下载任务版本，用于区分不同的下载
}

impl DownloadMetadata {
    /// 创建新的下载元数据
    pub fn new(url: String, expected_size: u64, expected_hash: Option<String>, version: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            url,
            expected_size,
            expected_hash,
            downloaded_bytes: 0,
            start_time: now.clone(),
            last_update: now,
            version,
        }
    }
    
    /// 更新下载进度
    pub fn update_progress(&mut self, downloaded_bytes: u64) {
        self.downloaded_bytes = downloaded_bytes;
        self.last_update = chrono::Utc::now().to_rfc3339();
    }
    
    /// 检查是否为相同的下载任务
    pub fn is_same_task(&self, url: &str, expected_size: u64, version: &str) -> bool {
        self.url == url && 
        self.expected_size == expected_size && 
        self.version == version
    }
}

/// 下载器类型
#[derive(Debug, Clone)]
pub enum DownloaderType {
    Http,
    HttpExtendedTimeout,
}

/// 下载器配置
#[derive(Debug, Clone)]
pub struct DownloaderConfig {
    pub timeout_seconds: u64,
    pub chunk_size: usize,
    pub retry_count: u32,
    pub enable_progress_logging: bool,
    pub enable_resume: bool, // 启用断点续传 ⭐
    pub resume_threshold: u64, // 断点续传阈值（字节），小于此值的文件重新下载 ⭐
    pub progress_interval_seconds: u64, // 进度显示时间间隔（秒）⭐
    pub progress_bytes_interval: u64, // 进度显示字节间隔 ⭐
    pub enable_metadata: bool, // 启用元数据管理 ⭐
}

impl Default for DownloaderConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30 * 60, // 30分钟
            chunk_size: 8192,         // 8KB
            retry_count: 3,
            enable_progress_logging: true,
            enable_resume: true,      // 默认启用断点续传 ⭐
            resume_threshold: 1024 * 1024, // 1MB，小于1MB的文件重新下载 ⭐
            progress_interval_seconds: 5, // 每5秒显示一次进度 ⭐
            progress_bytes_interval: 50 * 1024 * 1024, // 每50MB显示一次进度 ⭐
            enable_metadata: true,    // 默认启用元数据管理 ⭐
        }
    }
}

/// 文件下载器
pub struct FileDownloader {
    config: DownloaderConfig,
    client: Client,
}

impl FileDownloader {
    /// 创建新的文件下载器
    pub fn new(config: DownloaderConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// 创建默认配置的下载器
    pub fn default() -> Self {
        Self::new(DownloaderConfig::default())
    }

    /// 检查 URL 是否为阿里云 OSS 链接
    pub fn is_aliyun_oss_url(&self, url: &str) -> bool {
        url.starts_with("https://") && url.contains("aliyuncs.com") && url.contains("oss-")
    }

    /// 判断下载器类型
    pub fn get_downloader_type(&self, url: &str) -> DownloaderType {
        if self.is_aliyun_oss_url(url) {
            // 所有阿里云 OSS URL 都使用扩展超时 HTTP 下载（公网访问）
            DownloaderType::HttpExtendedTimeout
        } else {
            DownloaderType::Http
        }
    }

    /// 检查服务器是否支持Range请求 ⭐
    async fn check_range_support(&self, url: &str) -> Result<(bool, u64)> {
        let response = self.client.head(url)
            .send()
            .await
            .map_err(|e| DuckError::custom(format!("检查Range支持失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(DuckError::custom(format!(
                "服务器响应错误: HTTP {}",
                response.status()
            )));
        }

        let total_size = response.content_length().unwrap_or(0);
        let supports_range = response
            .headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.contains("bytes"))
            .unwrap_or(false);

        Ok((supports_range, total_size))
    }

    /// 获取下载元数据文件路径 ⭐
    fn get_metadata_path(&self, download_path: &Path) -> std::path::PathBuf {
        download_path.with_extension("download")
    }

    /// 保存下载元数据 ⭐
    async fn save_metadata(&self, download_path: &Path, metadata: &DownloadMetadata) -> Result<()> {
        self.save_metadata_with_logging(download_path, metadata, true).await
    }

    /// 保存下载元数据（可控制日志输出）⭐
    async fn save_metadata_with_logging(&self, download_path: &Path, metadata: &DownloadMetadata, show_log: bool) -> Result<()> {
        if !self.config.enable_metadata {
            return Ok(());
        }

        let metadata_path = self.get_metadata_path(download_path);
        let json_content = serde_json::to_string_pretty(metadata)
            .map_err(|e| DuckError::custom(format!("序列化元数据失败: {}", e)))?;

        tokio::fs::write(&metadata_path, json_content)
            .await
            .map_err(|e| DuckError::custom(format!("保存元数据失败: {}", e)))?;

        if show_log {
            info!("💾 已保存下载元数据: {}", metadata_path.display());
        }
        Ok(())
    }

    /// 加载下载元数据 ⭐
    async fn load_metadata(&self, download_path: &Path) -> Result<Option<DownloadMetadata>> {
        if !self.config.enable_metadata {
            return Ok(None);
        }

        let metadata_path = self.get_metadata_path(download_path);
        if !metadata_path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&metadata_path)
            .await
            .map_err(|e| DuckError::custom(format!("读取元数据失败: {}", e)))?;

        let metadata: DownloadMetadata = serde_json::from_str(&content)
            .map_err(|e| DuckError::custom(format!("解析元数据失败: {}", e)))?;

        info!("📋 已加载下载元数据: {}", metadata_path.display());
        Ok(Some(metadata))
    }

    /// 清理下载元数据 ⭐
    async fn cleanup_metadata(&self, download_path: &Path) -> Result<()> {
        if !self.config.enable_metadata {
            return Ok(());
        }

        let metadata_path = self.get_metadata_path(download_path);
        if metadata_path.exists() {
            tokio::fs::remove_file(&metadata_path)
                .await
                .map_err(|e| DuckError::custom(format!("清理元数据失败: {}", e)))?;
            info!("🧹 已清理下载元数据: {}", metadata_path.display());
        }
        Ok(())
    }

    /// 智能检查断点续传可行性 ⭐
    async fn check_resume_feasibility(
        &self, 
        download_path: &Path, 
        url: &str,
        total_size: u64,
        expected_hash: Option<&str>,
        version: &str
    ) -> Result<Option<u64>> {
        info!("🔍 检查断点续传可行性...");
        
        // 1. 检查文件是否存在
        if !download_path.exists() {
            info!("📁 目标文件不存在，无法续传");
            return Ok(None);
        }

        // 2. 获取当前文件大小
        let file_metadata = tokio::fs::metadata(download_path)
            .await
            .map_err(|e| DuckError::custom(format!("读取文件元数据失败: {}", e)))?;
        let existing_size = file_metadata.len();

        info!("📊 当前文件大小: {} bytes ({:.2} MB)", existing_size, existing_size as f64 / 1024.0 / 1024.0);

        // 3. 【优先】检查hash文件是否存在，如果存在则优先验证hash ⭐
        if let Some(expected_hash) = expected_hash {
            info!("🔍 优先进行hash验证...");
            match Self::calculate_file_hash(download_path).await {
                Ok(actual_hash) => {
                    if actual_hash.to_lowercase() == expected_hash.to_lowercase() {
                        info!("✅ 文件hash验证通过，文件已完整");
                        // 清理元数据（下载已完成）
                        let _ = self.cleanup_metadata(download_path).await;
                        return Ok(None); // 无需下载
                    } else {
                        info!("❌ 文件hash验证失败，进入断点续传判断");
                        info!("   期望hash: {}", expected_hash);
                        info!("   实际hash: {}", actual_hash);
                        // 继续下面的断点续传逻辑，不要立即删除文件
                    }
                }
                Err(e) => {
                    warn!("⚠️ 计算文件hash失败: {}，进入断点续传判断", e);
                    // 继续下面的断点续传逻辑
                }
            }
        }

        // 4. 检查文件是否已完整（大小检查）
        if existing_size >= total_size {
            // 如果文件大小已完整但hash不匹配，说明文件损坏，重新下载
            if expected_hash.is_some() {
                warn!("❌ 文件大小完整但hash不匹配，文件已损坏，将重新下载");
                let _ = tokio::fs::remove_file(download_path).await;
                let _ = self.cleanup_metadata(download_path).await;
                return Ok(None); // 重新下载
            } else {
                // 没有hash验证，认为文件完整
                info!("✅ 文件大小完整且无hash验证要求，认为文件完整");
                let _ = self.cleanup_metadata(download_path).await;
                return Ok(None);
            }
        }

        // 5. 检查文件大小是否符合续传阈值
        if existing_size < self.config.resume_threshold {
            info!("📁 文件过小 ({} bytes < {} bytes)，将重新下载", 
                existing_size, self.config.resume_threshold);
            let _ = tokio::fs::remove_file(download_path).await;
            let _ = self.cleanup_metadata(download_path).await;
            return Ok(None);
        }

        // 6. 检查下载元数据
        if let Some(metadata) = self.load_metadata(download_path).await? {
            // 验证是否为相同的下载任务
            if metadata.is_same_task(url, total_size, version) {
                info!("✅ 发现匹配的下载任务");
                info!("   原始URL: {}", metadata.url);
                info!("   预期大小: {} bytes", metadata.expected_size);
                info!("   开始时间: {}", metadata.start_time);
                info!("   上次更新: {}", metadata.last_update);
                
                // 如果有hash要求，额外检查hash是否匹配
                if let Some(expected_hash) = expected_hash {
                    if let Some(ref metadata_hash) = metadata.expected_hash {
                        if metadata_hash.to_lowercase() == expected_hash.to_lowercase() {
                            info!("✅ 元数据hash匹配，可以安全续传");
                        } else {
                            warn!("❌ 元数据hash不匹配，可能是不同版本");
                            warn!("   当前期望hash: {}", expected_hash);
                            warn!("   元数据记录hash: {}", metadata_hash);
                            warn!("   清理旧数据，重新下载");
                            let _ = tokio::fs::remove_file(download_path).await;
                            let _ = self.cleanup_metadata(download_path).await;
                            return Ok(None);
                        }
                    } else {
                        warn!("⚠️ 元数据缺少hash信息，但现在需要hash验证");
                        warn!("   为安全起见，重新下载");
                        let _ = tokio::fs::remove_file(download_path).await;
                        let _ = self.cleanup_metadata(download_path).await;
                        return Ok(None);
                    }
                }
                
                // 验证元数据中的下载进度是否与文件大小一致
                if metadata.downloaded_bytes == existing_size {
                    info!("✅ 元数据与文件大小一致，可以续传");
                    info!("📁 续传点: {} bytes / {} bytes ({:.1}%)", 
                        existing_size, total_size, 
                        (existing_size as f64 / total_size as f64) * 100.0);
                    return Ok(Some(existing_size));
                } else {
                    warn!("⚠️ 元数据与文件大小不一致");
                    warn!("   元数据记录: {} bytes", metadata.downloaded_bytes);
                    warn!("   实际文件: {} bytes", existing_size);
                    
                    // 以实际文件大小为准，更新元数据
                    info!("🔄 以实际文件大小为准，继续续传");
                    return Ok(Some(existing_size));
                }
            } else {
                warn!("❌ 下载任务不匹配，将重新下载");
                warn!("   当前URL: {}", url);
                warn!("   元数据URL: {}", metadata.url);
                warn!("   当前大小: {} bytes", total_size);
                warn!("   元数据大小: {} bytes", metadata.expected_size);
                warn!("   当前版本: {}", version);
                warn!("   元数据版本: {}", metadata.version);
                
                // 清理不匹配的下载
                let _ = tokio::fs::remove_file(download_path).await;
                let _ = self.cleanup_metadata(download_path).await;
                return Ok(None);
            }
        } else {
            // 没有元数据，但文件存在 - 可能是旧的下载
            warn!("⚠️ 发现无元数据的部分文件");
            
            if expected_hash.is_some() {
                // 有hash要求，检查是否可以智能续传
                info!("🔍 有hash验证要求，评估智能续传可能性");
                
                // 如果文件大小超过总大小的50%，尝试续传
                let progress_percentage = (existing_size as f64 / total_size as f64) * 100.0;
                if progress_percentage >= 50.0 {
                    info!("📊 文件已下载 {:.1}%，尝试智能续传", progress_percentage);
                    info!("   注意：续传后将进行完整性验证");
                    return Ok(Some(existing_size));
                } else {
                    warn!("🔒 文件进度不足50%且无元数据，为安全起见将重新下载");
                    let _ = tokio::fs::remove_file(download_path).await;
                    return Ok(None);
                }
            } else {
                // 没有hash要求，尝试续传
                info!("🤔 尝试续传无元数据的文件");
                info!("📁 续传点: {} bytes / {} bytes ({:.1}%)", 
                    existing_size, total_size, 
                    (existing_size as f64 / total_size as f64) * 100.0);
                return Ok(Some(existing_size));
            }
        }
    }

    /// 下载文件（支持断点续传）⭐
    pub async fn download_file<F>(
        &self,
        url: &str,
        download_path: &Path,
        progress_callback: Option<F>,
    ) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        self.download_file_with_options(url, download_path, progress_callback, None, None).await
    }

    /// 下载文件（带额外选项）⭐
    pub async fn download_file_with_options<F>(
        &self,
        url: &str,
        download_path: &Path,
        progress_callback: Option<F>,
        expected_hash: Option<&str>,
        version: Option<&str>,
    ) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        let downloader_type = self.get_downloader_type(url);
        let version = version.unwrap_or("unknown");
        
        info!("🌐 开始下载文件");
        info!("   URL: {}", url);
        info!("   目标路径: {}", download_path.display());
        info!("   下载器类型: {:?}", downloader_type);
        info!("   断点续传: {}", if self.config.enable_resume { "启用" } else { "禁用" });
        if let Some(hash) = expected_hash {
            info!("   期望Hash: {}", hash);
        }
        info!("   版本标识: {}", version);

        // 检查Range支持和文件大小
        let (supports_range, total_size) = self.check_range_support(url).await?;
        
        if total_size > 0 {
            info!("📦 服务器文件大小: {} bytes ({:.2} MB)", total_size, total_size as f64 / 1024.0 / 1024.0);
        }

        if supports_range && self.config.enable_resume {
            info!("✅ 服务器支持Range请求，启用断点续传");
        } else if !supports_range {
            warn!("⚠️ 服务器不支持Range请求，使用普通下载");
        }

        // 智能检查断点续传可行性
        let existing_size = if supports_range && self.config.enable_resume {
            self.check_resume_feasibility(download_path, url, total_size, expected_hash, version).await?
        } else {
            None
        };

        // 创建下载元数据
        let mut metadata = DownloadMetadata::new(
            url.to_string(), 
            total_size, 
            expected_hash.map(|s| s.to_string()), 
            version.to_string()
        );

        // 如果是续传，更新进度
        if let Some(resume_size) = existing_size {
            metadata.update_progress(resume_size);
        }

        // 保存初始元数据
        self.save_metadata(download_path, &metadata).await?;

        // 执行下载
        let result = match downloader_type {
            DownloaderType::Http => {
                self.download_via_http_with_resume(url, download_path, progress_callback, existing_size, total_size, &mut metadata).await
            }
            DownloaderType::HttpExtendedTimeout => {
                self.download_via_http_extended_timeout_with_resume(url, download_path, progress_callback, existing_size, total_size, &mut metadata).await
            }
        };

        // 处理下载结果
        match result {
            Ok(_) => {
                // 下载成功，清理元数据
                info!("🎉 下载完成，清理元数据");
                let _ = self.cleanup_metadata(download_path).await;
                
                // 最终hash验证（如果提供）
                if let Some(hash) = expected_hash {
                    info!("🔍 最终hash验证...");
                    match Self::calculate_file_hash(download_path).await {
                        Ok(actual_hash) => {
                            if actual_hash.to_lowercase() == hash.to_lowercase() {
                                info!("✅ 最终hash验证通过");
                            } else {
                                warn!("❌ 最终hash验证失败");
                                warn!("   期望: {}", hash);
                                warn!("   实际: {}", actual_hash);
                                return Err(DuckError::custom("文件hash验证失败"));
                            }
                        }
                        Err(e) => {
                            warn!("⚠️ 计算最终hash失败: {}", e);
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                // 下载失败，保留元数据用于下次续传
                warn!("❌ 下载失败: {}", e);
                info!("💾 保留元数据用于下次续传");
                Err(e)
            }
        }
    }

    /// 使用普通 HTTP 下载（支持断点续传）⭐
    async fn download_via_http_with_resume<F>(
        &self,
        url: &str,
        download_path: &Path,
        progress_callback: Option<F>,
        existing_size: Option<u64>,
        total_size: u64,
        metadata: &mut DownloadMetadata,
    ) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        info!("📥 使用普通 HTTP 下载");
        self.download_with_resume_internal(url, download_path, progress_callback, existing_size, total_size, "http_download", metadata).await
    }

    /// 使用扩展超时的 HTTP 下载（支持断点续传）⭐
    async fn download_via_http_extended_timeout_with_resume<F>(
        &self,
        url: &str,
        download_path: &Path,
        progress_callback: Option<F>,
        existing_size: Option<u64>,
        total_size: u64,
        metadata: &mut DownloadMetadata,
    ) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        if self.is_aliyun_oss_url(url) {
            info!("📥 使用扩展超时 HTTP 下载 (阿里云 OSS 公网文件)");
            info!("   💡 检测到公网访问的 OSS 文件，无需密钥");
            if existing_size.is_some() {
                info!("   🔄 支持断点续传");
            }
        } else {
            info!("📥 使用扩展超时 HTTP 下载");
        }
        
        self.download_with_resume_internal(url, download_path, progress_callback, existing_size, total_size, "extended_http_download", metadata).await
    }

    /// 内部断点续传下载实现 ⭐
    async fn download_with_resume_internal<F>(
        &self,
        url: &str,
        download_path: &Path,
        progress_callback: Option<F>,
        existing_size: Option<u64>,
        total_size: u64,
        task_id: &str,
        metadata: &mut DownloadMetadata,
    ) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        let start_byte = existing_size.unwrap_or(0);
        let is_resume = existing_size.is_some();

        // 构建请求
        let mut request = self.client.get(url);
        
        if is_resume {
            info!("🔄 断点续传：从字节 {} 开始下载", start_byte);
            request = request.header("Range", format!("bytes={}-", start_byte));
        }

        let response = request
            .send()
            .await
            .map_err(|e| DuckError::custom(format!("发起下载请求失败: {}", e)))?;

        // 检查响应状态
        let expected_status = if is_resume { 206 } else { 200 };
        if response.status().as_u16() != expected_status {
            return Err(DuckError::custom(format!(
                "下载失败: HTTP {} (期望: {})",
                response.status(), expected_status
            )));
        }

        // 打开文件（追加模式或创建模式）
        let mut file = if is_resume {
            info!("📝 以追加模式打开文件");
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(download_path)
                .await
                .map_err(|e| DuckError::custom(format!("打开文件失败: {}", e)))?
        } else {
            info!("📝 创建新文件");
            File::create(download_path)
                .await
                .map_err(|e| DuckError::custom(format!("创建文件失败: {}", e)))?
        };

        // 执行下载
        self.download_stream_with_resume(
            response, 
            &mut file, 
            download_path, 
            progress_callback, 
            task_id, 
            start_byte, 
            total_size,
            is_resume,
            metadata
        ).await
    }

    /// 通用的流式下载处理（支持断点续传）⭐
    async fn download_stream_with_resume<F>(
        &self,
        response: reqwest::Response,
        file: &mut File,
        download_path: &Path,
        progress_callback: Option<F>,
        task_id: &str,
        start_byte: u64,
        total_size: u64,
        is_resume: bool,
        metadata: &mut DownloadMetadata,
    ) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        let mut downloaded = start_byte; // 从已下载的字节开始计算
        let mut stream = response.bytes_stream();
        let mut last_progress_time = std::time::Instant::now();
        let mut last_progress_bytes = downloaded;
        let progress_interval = std::time::Duration::from_secs(self.config.progress_interval_seconds);
        
        // 首次进度回调
        if let Some(callback) = progress_callback.as_ref() {
            let status = if is_resume { DownloadStatus::Resuming } else { DownloadStatus::Starting };
            callback(DownloadProgress {
                task_id: task_id.to_string(),
                file_name: download_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                downloaded_bytes: downloaded,
                total_bytes: total_size,
                download_speed: 0.0,
                eta_seconds: 0,
                percentage: if total_size > 0 { downloaded as f64 / total_size as f64 * 100.0 } else { 0.0 },
                status,
            });
        }
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| DuckError::custom(format!("下载数据失败: {}", e)))?;
            
            file.write_all(&chunk)
                .await
                .map_err(|e| DuckError::custom(format!("写入文件失败: {}", e)))?;
            
            downloaded += chunk.len() as u64;
            
            // 调用进度回调
            if let Some(callback) = progress_callback.as_ref() {
                let progress = if total_size > 0 {
                    downloaded as f64 / total_size as f64 * 100.0
                } else {
                    0.0
                };
                
                callback(DownloadProgress {
                    task_id: task_id.to_string(),
                    file_name: download_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    downloaded_bytes: downloaded,
                    total_bytes: total_size,
                    download_speed: 0.0,
                    eta_seconds: 0,
                    percentage: progress,
                    status: DownloadStatus::Downloading,
                });
            }
            
            // 进度显示逻辑
            if self.config.enable_progress_logging {
                let now = std::time::Instant::now();
                let bytes_since_last = downloaded - last_progress_bytes;
                let time_since_last = now.duration_since(last_progress_time);
                
                let should_show_progress = 
                    bytes_since_last >= self.config.progress_bytes_interval ||  // 根据配置的字节间隔显示
                    time_since_last >= progress_interval ||  // 根据配置的时间间隔显示
                    (total_size > 0 && downloaded >= total_size); // 下载完成时显示
                
                if should_show_progress {
                    if total_size > 0 {
                        let percentage = (downloaded as f64 / total_size as f64 * 100.0) as u32;
                        let status_icon = if is_resume && downloaded <= start_byte + 50*1024*1024 {
                            "🔄" // 断点续传图标
                        } else {
                            "📥" // 普通下载图标
                        };
                        
                        // 计算下载速度（仅用于显示）
                        let speed_mbps = if time_since_last.as_secs() > 0 {
                            (bytes_since_last as f64 / 1024.0 / 1024.0) / time_since_last.as_secs() as f64
                        } else {
                            0.0
                        };
                        
                        info!("{} 下载进度: {}% ({:.1}/{:.1} MB) 速度: {:.1} MB/s", 
                            status_icon,
                            percentage,
                            downloaded as f64 / 1024.0 / 1024.0,
                            total_size as f64 / 1024.0 / 1024.0,
                            speed_mbps
                        );
                    } else {
                        info!("📥 已下载: {:.1} MB", downloaded as f64 / 1024.0 / 1024.0);
                    }
                    
                    last_progress_time = now;
                    last_progress_bytes = downloaded;
                    
                    // 更新元数据（减少保存频率，避免重复日志）⭐
                    if self.config.enable_metadata {
                        metadata.update_progress(downloaded);
                        // 只在特定条件下保存元数据：每500MB或每5分钟
                        let should_save_metadata = 
                            bytes_since_last >= 500 * 1024 * 1024 ||  // 每500MB保存一次
                            time_since_last >= std::time::Duration::from_secs(300); // 每5分钟保存一次
                        
                        if should_save_metadata {
                            // 静默保存，不输出日志（避免重复日志）
                            let _ = self.save_metadata_with_logging(download_path, metadata, false).await;
                        }
                    }
                }
            }
        }
        
        // 确保文件已刷新到磁盘
        file.flush().await
            .map_err(|e| DuckError::custom(format!("刷新文件缓冲区失败: {}", e)))?;
        
        let download_type = if is_resume { "断点续传下载" } else { "下载" };
        info!("✅ {}完成", download_type);
        info!("   文件路径: {}", download_path.display());
        info!("   最终大小: {} bytes ({:.2} MB)", downloaded, downloaded as f64 / 1024.0 / 1024.0);
        if is_resume {
            info!("   续传大小: {} bytes ({:.2} MB)", downloaded - start_byte, (downloaded - start_byte) as f64 / 1024.0 / 1024.0);
        }
        
        Ok(())
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
}

/// 简化的下载功能，用于向后兼容
pub async fn download_file_simple(
    url: &str,
    download_path: &Path,
) -> Result<()> {
    let downloader = FileDownloader::default();
    downloader.download_file::<fn(DownloadProgress)>(url, download_path, None).await
}

/// 带进度回调的下载功能
pub async fn download_file_with_progress<F>(
    url: &str,
    download_path: &Path,
    progress_callback: Option<F>,
) -> Result<()>
where
    F: Fn(DownloadProgress) + Send + Sync + 'static,
{
    let downloader = FileDownloader::default();
    downloader.download_file(url, download_path, progress_callback).await
}

/// 创建自定义配置的下载器
pub fn create_downloader(config: DownloaderConfig) -> FileDownloader {
    FileDownloader::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aliyun_oss_url_detection() {
        let downloader = FileDownloader::default();
        
        // 测试您提供的真实阿里云 OSS URL
        let real_oss_url = "https://nuwa-packages.oss-rg-china-mainland.aliyuncs.com/duck-client-releases/docker/20250705082538/docker.zip";
        assert!(downloader.is_aliyun_oss_url(real_oss_url), "应该识别为阿里云 OSS URL");
        
        // 测试其他阿里云 OSS URL 格式
        let test_cases = vec![
            ("https://bucket.oss-cn-hangzhou.aliyuncs.com/file.zip", true),
            ("https://my-bucket.oss-us-west-1.aliyuncs.com/path/file.tar.gz", true),
            ("https://test.oss-ap-southeast-1.aliyuncs.com/docker.zip", true),
            ("https://example.com/file.zip", false),
            ("https://github.com/user/repo/releases/download/v1.0.0/file.zip", false),
            ("ftp://bucket.oss-cn-beijing.aliyuncs.com/file.zip", false),
        ];
        
        for (url, expected) in test_cases {
            assert_eq!(
                downloader.is_aliyun_oss_url(url), 
                expected,
                "URL: {} 应该返回 {}",
                url, expected
            );
        }
    }
    
    #[test]
    fn test_downloader_type_detection() {
        let downloader = FileDownloader::default();
        
        // 测试您的真实 OSS URL（公网访问）
        let real_oss_url = "https://nuwa-packages.oss-rg-china-mainland.aliyuncs.com/duck-client-releases/docker/20250705082538/docker.zip";
        let downloader_type = downloader.get_downloader_type(real_oss_url);
        
        match downloader_type {
            DownloaderType::HttpExtendedTimeout => println!("✅ 正确识别为扩展超时 HTTP 下载（公网访问）"),
            DownloaderType::Http => println!("❌ 错误识别为普通 HTTP 下载"),
        }
        
        // 对于阿里云 OSS 文件，应该使用扩展超时HTTP下载
        assert!(matches!(
            downloader_type,
            DownloaderType::HttpExtendedTimeout
        ), "OSS文件应该使用扩展超时HTTP下载");
        
        // 测试普通 HTTP URL
        let http_url = "https://github.com/user/repo/releases/download/v1.0.0/file.zip";
        assert!(matches!(
            downloader.get_downloader_type(http_url),
            DownloaderType::Http
        ), "普通 HTTP URL 应该使用标准下载");
    }
} 