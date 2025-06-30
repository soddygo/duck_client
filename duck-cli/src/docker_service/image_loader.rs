use crate::docker_service::architecture::{Architecture, detect_architecture};
use crate::docker_service::error::{DockerServiceError, DockerServiceResult};
use client_core::container::DockerManager;
use std::path::PathBuf;
use tracing::{error, info, warn};

/// 镜像类型
#[derive(Debug, Clone, PartialEq)]
pub enum ImageType {
    /// 业务镜像
    Business,
    /// 基础组件镜像
    Infrastructure,
}

/// 镜像信息
#[derive(Debug, Clone)]
pub struct ImageInfo {
    /// 镜像文件路径
    pub file_path: PathBuf,
    /// 镜像类型
    #[allow(dead_code)]
    pub image_type: ImageType,
    /// 原始标签（带架构后缀）
    pub original_tag: String,
    /// 目标标签（去除架构后缀）
    pub target_tag: String,
    /// 镜像架构
    #[allow(dead_code)]
    pub architecture: Architecture,
    /// 文件大小
    pub file_size: u64,
}

impl ImageInfo {
    /// 从文件路径推断镜像信息
    pub fn from_file_path(
        file_path: PathBuf,
        architecture: Architecture,
    ) -> DockerServiceResult<Self> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| DockerServiceError::ImageLoading("无效的文件名".to_string()))?;

        // 推断镜像类型
        let image_type = if file_name.contains("mysql")
            || file_name.contains("redis")
            || file_name.contains("nginx")
            || file_name.contains("postgres")
        {
            ImageType::Infrastructure
        } else {
            ImageType::Business
        };

        // 提取原始标签和目标标签
        let arch_suffix = format!("-{}", architecture.as_str());
        let (original_tag, target_tag) =
            if let Some(name_without_ext) = file_name.strip_suffix(".tar") {
                if name_without_ext.ends_with(&arch_suffix) {
                    let target = &name_without_ext[..name_without_ext.len() - arch_suffix.len()];
                    (name_without_ext.to_string(), target.to_string())
                } else {
                    (name_without_ext.to_string(), name_without_ext.to_string())
                }
            } else {
                (file_name.to_string(), file_name.to_string())
            };

        // 获取文件大小
        let file_size = std::fs::metadata(&file_path)
            .map_err(|e| DockerServiceError::FileSystem(e.to_string()))?
            .len();

        Ok(Self {
            file_path,
            image_type,
            original_tag,
            target_tag,
            architecture,
            file_size,
        })
    }
}

/// 镜像加载结果
#[derive(Debug, Clone)]
pub struct LoadResult {
    pub success_count: usize,
    pub failure_count: usize,
    pub loaded_images: Vec<String>,
    pub failed_images: Vec<(String, String)>, // (文件名, 错误信息)
}

impl LoadResult {
    pub fn new() -> Self {
        Self {
            success_count: 0,
            failure_count: 0,
            loaded_images: Vec::new(),
            failed_images: Vec::new(),
        }
    }

    pub fn add_success(&mut self, image_name: String) {
        self.success_count += 1;
        self.loaded_images.push(image_name);
    }

    pub fn add_failure(&mut self, image_name: String, error: String) {
        self.failure_count += 1;
        self.failed_images.push((image_name, error));
    }

    pub fn is_all_successful(&self) -> bool {
        self.failure_count == 0
    }

    pub fn success_count(&self) -> usize {
        self.success_count
    }

    pub fn failure_count(&self) -> usize {
        self.failure_count
    }
}

impl Default for LoadResult {
    fn default() -> Self {
        Self::new()
    }
}

/// 标签设置结果
#[derive(Debug, Clone)]
pub struct TagResult {
    pub success_count: usize,
    pub failure_count: usize,
    pub tagged_images: Vec<(String, String)>, // (原始标签, 目标标签)
    pub failed_tags: Vec<(String, String, String)>, // (原始标签, 目标标签, 错误信息)
}

impl TagResult {
    pub fn new() -> Self {
        Self {
            success_count: 0,
            failure_count: 0,
            tagged_images: Vec::new(),
            failed_tags: Vec::new(),
        }
    }

    pub fn add_success(&mut self, original: String, target: String) {
        self.success_count += 1;
        self.tagged_images.push((original, target));
    }

    pub fn add_failure(&mut self, original: String, target: String, error: String) {
        self.failure_count += 1;
        self.failed_tags.push((original, target, error));
    }

    pub fn is_all_successful(&self) -> bool {
        self.failure_count == 0
    }

    pub fn success_count(&self) -> usize {
        self.success_count
    }

    pub fn failure_count(&self) -> usize {
        self.failure_count
    }
}

impl Default for TagResult {
    fn default() -> Self {
        Self::new()
    }
}

/// 镜像加载器
pub struct ImageLoader {
    docker_manager: DockerManager,
    #[allow(dead_code)]
    work_dir: PathBuf,
    architecture: Architecture,
    images_dir: PathBuf,
}

impl ImageLoader {
    /// 创建新的镜像加载器
    pub fn new(docker_manager: DockerManager, work_dir: PathBuf) -> DockerServiceResult<Self> {
        let architecture = detect_architecture();
        let images_dir = work_dir.join("images");

        Ok(Self {
            docker_manager,
            work_dir,
            architecture,
            images_dir,
        })
    }

    /// 扫描并获取当前架构的镜像列表
    pub fn scan_architecture_images(&self) -> DockerServiceResult<Vec<ImageInfo>> {
        if !self.images_dir.exists() {
            return Err(DockerServiceError::ImageLoading(format!(
                "镜像目录不存在: {}",
                self.images_dir.display()
            )));
        }

        let arch_suffix = format!("-{}.tar", self.architecture.as_str());
        let mut images = Vec::new();

        for entry in std::fs::read_dir(&self.images_dir)
            .map_err(|e| DockerServiceError::ImageLoading(e.to_string()))?
        {
            let entry = entry.map_err(|e| DockerServiceError::ImageLoading(e.to_string()))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.ends_with(&arch_suffix) {
                        match ImageInfo::from_file_path(path.clone(), self.architecture) {
                            Ok(image_info) => {
                                info!(
                                    "发现镜像文件: {} ({})",
                                    file_name,
                                    format_file_size(image_info.file_size)
                                );
                                images.push(image_info);
                            }
                            Err(e) => {
                                warn!("解析镜像文件失败: {} - {}", file_name, e);
                            }
                        }
                    }
                }
            }
        }

        if images.is_empty() {
            return Err(DockerServiceError::ImageLoading(format!(
                "未找到 {} 架构的镜像文件",
                self.architecture.as_str()
            )));
        }

        info!(
            "共发现 {} 个 {} 架构的镜像文件",
            images.len(),
            self.architecture.as_str()
        );
        Ok(images)
    }

    /// 加载所有镜像
    pub async fn load_all_images(&self) -> DockerServiceResult<LoadResult> {
        let images = self.scan_architecture_images()?;
        let mut result = LoadResult::new();

        info!("开始加载 {} 个镜像文件...", images.len());

        for (index, image) in images.iter().enumerate() {
            let progress = format!("[{}/{}]", index + 1, images.len());
            let file_name = image
                .file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            info!(
                "{} 加载镜像: {} ({})",
                progress,
                file_name,
                format_file_size(image.file_size)
            );

            match self.docker_manager.load_image(&image.file_path).await {
                Ok(_) => {
                    info!("{} ✓ 镜像加载成功: {}", progress, file_name);
                    result.add_success(file_name.to_string());
                }
                Err(e) => {
                    error!("{} ✗ 镜像加载失败: {} - {}", progress, file_name, e);
                    result.add_failure(file_name.to_string(), e.to_string());
                }
            }
        }

        info!(
            "镜像加载完成: 成功 {}, 失败 {}",
            result.success_count, result.failure_count
        );
        Ok(result)
    }

    /// 设置镜像标签
    pub async fn setup_image_tags(&self) -> DockerServiceResult<TagResult> {
        let images = self.scan_architecture_images()?;
        let mut result = TagResult::new();

        info!("开始设置镜像标签...");

        for image in images {
            if image.original_tag != image.target_tag {
                let original_with_latest =
                    format!("{}:latest-{}", image.target_tag, self.architecture.as_str());
                let target_with_latest = format!("{}:latest", image.target_tag);

                info!(
                    "设置标签: {} -> {}",
                    original_with_latest, target_with_latest
                );

                // 使用 docker tag 命令设置标签
                match self
                    .tag_image(&original_with_latest, &target_with_latest)
                    .await
                {
                    Ok(_) => {
                        info!("✓ 标签设置成功: {}", target_with_latest);
                        result.add_success(original_with_latest, target_with_latest);
                    }
                    Err(e) => {
                        error!("✗ 标签设置失败: {} - {}", target_with_latest, e);
                        result.add_failure(original_with_latest, target_with_latest, e.to_string());
                    }
                }
            }
        }

        info!(
            "标签设置完成: 成功 {}, 失败 {}",
            result.success_count, result.failure_count
        );
        Ok(result)
    }

    /// 为单个镜像设置标签
    async fn tag_image(&self, source_tag: &str, target_tag: &str) -> DockerServiceResult<()> {
        use tokio::process::Command;

        let output = Command::new("docker")
            .args(["tag", source_tag, target_tag])
            .output()
            .await
            .map_err(|e| DockerServiceError::DockerCommand(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerServiceError::DockerCommand(format!(
                "设置镜像标签失败: {stderr}"
            )));
        }

        Ok(())
    }
}

/// 格式化文件大小显示
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_image_info_from_file_path() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("agent-platform-front-amd64.tar");
        std::fs::write(&file_path, b"fake image data").unwrap();

        let image_info = ImageInfo::from_file_path(file_path, Architecture::Amd64).unwrap();

        assert_eq!(image_info.image_type, ImageType::Business);
        assert_eq!(image_info.architecture, Architecture::Amd64);
        assert!(image_info.original_tag.contains("agent-platform-front"));
        assert!(image_info.original_tag.contains("amd64"));
        assert!(!image_info.target_tag.contains("amd64"));
    }
}
