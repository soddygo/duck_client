use crate::constants::{backup, config, docker, updates, version};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use toml;

/// 应用配置结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub versions: Versions,
    pub docker: DockerConfig,
    pub backup: BackupConfig,
    pub cache: CacheConfig,
    pub updates: UpdatesConfig,
}

/// 版本信息配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Versions {
    pub docker_service: String,
}

/// Docker相关配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DockerConfig {
    pub compose_file: String,
}

/// 备份相关配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackupConfig {
    pub storage_dir: String,
}

/// 缓存相关配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheConfig {
    pub cache_dir: String,
    pub download_dir: String,
}

/// 更新相关配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdatesConfig {
    pub check_frequency: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            versions: Versions {
                docker_service: version::version_info::DEFAULT_DOCKER_SERVICE_VERSION.to_string(),
            },
            docker: DockerConfig {
                compose_file: docker::get_compose_file_path_str(),
            },
            backup: BackupConfig {
                storage_dir: backup::get_default_storage_dir()
                    .to_string_lossy()
                    .to_string(),
            },
            cache: CacheConfig {
                cache_dir: config::get_default_cache_dir()
                    .to_string_lossy()
                    .to_string(),
                download_dir: config::get_default_download_dir()
                    .to_string_lossy()
                    .to_string(),
            },
            updates: UpdatesConfig {
                check_frequency: updates::DEFAULT_CHECK_FREQUENCY.to_string(),
            },
        }
    }
}

impl AppConfig {
    /// 智能查找并加载配置文件
    /// 按优先级查找：config.toml -> duck-client.toml -> .duck-client.toml
    pub fn find_and_load_config() -> Result<Self> {
        let config_files = ["config.toml", "duck-client.toml", ".duck-client.toml"];

        for config_file in &config_files {
            if Path::new(config_file).exists() {
                tracing::info!("找到配置文件: {}", config_file);
                return Self::load_from_file(config_file);
            }
        }

        // 如果没找到配置文件，创建默认配置
        tracing::warn!("未找到配置文件，创建默认配置: config.toml");
        let default_config = Self::default();
        default_config.save_to_file("config.toml")?;
        Ok(default_config)
    }

    /// 从指定文件加载配置
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)?;
        let config: AppConfig = toml::from_str(&content)?;

        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = self.to_toml_with_comments();
        fs::write(&path, content)?;
        Ok(())
    }

    /// 生成带注释的TOML配置
    fn to_toml_with_comments(&self) -> String {
        const TEMPLATE: &str = include_str!("../templates/config.toml.template");

        TEMPLATE
            .replace("{docker_service_version}", &self.versions.docker_service)
            .replace("{compose_file}", &self.docker.compose_file)
            .replace("{backup_storage_dir}", &self.backup.storage_dir)
            .replace("{cache_dir}", &self.cache.cache_dir)
            .replace("{download_dir}", &self.cache.download_dir)
            .replace("{check_frequency}", &self.updates.check_frequency)
    }

    /// 确保缓存目录存在
    pub fn ensure_cache_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.cache.cache_dir)?;
        fs::create_dir_all(&self.cache.download_dir)?;
        Ok(())
    }

    /// 获取下载目录路径
    pub fn get_download_dir(&self) -> PathBuf {
        PathBuf::from(&self.cache.download_dir)
    }

    /// 获取指定版本的全量下载目录路径
    pub fn get_version_download_dir(&self, version: &str, download_type: &str) -> PathBuf {
        PathBuf::from(&self.cache.download_dir)
            .join(version)
            .join(download_type)
    }

    /// 获取指定版本的全量下载文件路径
    pub fn get_version_download_file_path(
        &self,
        version: &str,
        download_type: &str,
        filename: &str,
    ) -> PathBuf {
        self.get_version_download_dir(version, download_type)
            .join(filename)
    }

    /// 确保指定版本的下载目录存在
    pub fn ensure_version_download_dir(
        &self,
        version: &str,
        download_type: &str,
    ) -> Result<PathBuf> {
        let dir = self.get_version_download_dir(version, download_type);
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// 获取备份目录路径
    pub fn get_backup_dir(&self) -> PathBuf {
        PathBuf::from(&self.backup.storage_dir)
    }
}
