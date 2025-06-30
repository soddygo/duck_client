use crate::api_config::ApiConfig;
use crate::error::{DuckError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{error, info, warn};

/// API 客户端
#[derive(Debug, Clone)]
pub struct ApiClient {
    client: Client,
    config: ApiConfig,
    client_id: Option<String>,
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
        }
    }

    /// 设置客户端ID
    pub fn set_client_id(&mut self, client_id: String) {
        self.client_id = Some(client_id);
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
            Err(DuckError::Api(format!("注册失败: {} - {}", status, text)))
        }
    }

    /// 获取系统公告
    pub async fn get_announcements(&self, since: Option<&str>) -> Result<AnnouncementsResponse> {
        let mut url = self
            .config
            .get_endpoint_url(&self.config.endpoints.announcements);

        if let Some(since_time) = since {
            url = format!("{}?since={}", url, since_time);
        }

        let response = self.build_request(&url).send().await?;

        if response.status().is_success() {
            let announcements = response.json().await?;
            Ok(announcements)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("获取公告失败: {} - {}", status, text);
            Err(DuckError::Api(format!(
                "获取公告失败: {} - {}",
                status, text
            )))
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
                "检查Docker版本失败: {} - {}",
                status, text
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
                "获取Docker版本列表失败: {} - {}",
                status, text
            )))
        }
    }

    /// 下载Docker服务更新包
    pub async fn download_service_update<P: AsRef<Path>>(&self, save_path: P) -> Result<()> {
        let url = self
            .config
            .get_endpoint_url(&self.config.endpoints.docker_download_full);

        info!("开始下载Docker服务更新包: {}", url);

        let response = self.build_request(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("下载Docker服务更新包失败: {} - {}", status, text);
            return Err(DuckError::Api(format!("下载失败: {} - {}", status, text)));
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
                        "\r📦 下载进度: [{}] {:.1}% ({:.1}/{:.1} MB)",
                        progress_bar, percentage, downloaded_mb, total_mb
                    );
                    io::stdout().flush().unwrap();

                    last_update = std::time::Instant::now();
                } else {
                    // 没有总大小信息时，只显示已下载量
                    let downloaded_mb = downloaded as f64 / 1024.0 / 1024.0;
                    print!("\r📦 下载进度: {:.1} MB", downloaded_mb);
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

            print!(
                "\r📦 下载进度: [{}] 100.0% ({:.1}/{:.1} MB)",
                progress_bar, downloaded_mb, total_mb
            );
            io::stdout().flush().unwrap();
        } else {
            // 没有总大小信息时，显示最终下载量
            let downloaded_mb = downloaded as f64 / 1024.0 / 1024.0;
            print!("\r📦 下载进度: {:.1} MB (完成)", downloaded_mb);
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
