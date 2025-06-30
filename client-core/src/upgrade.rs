use crate::{
    api::ApiClient,
    backup::{BackupManager, BackupOptions, RestoreOptions},
    config::AppConfig,
    constants::timeout,
    database::{BackupType, Database},
    container::DockerManager,
    DuckError, Result,
};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tracing::{info, warn};

/// 升级管理器
#[derive(Debug, Clone)]
pub struct UpgradeManager {
    config: AppConfig,
    config_path: PathBuf,
    docker_manager: DockerManager,
    backup_manager: BackupManager,
    api_client: ApiClient,
    #[allow(dead_code)]
    database: Database,
}

/// 升级选项
#[derive(Debug, Clone, Default)]
pub struct UpgradeOptions {
    pub skip_backup: bool,
    pub force: bool,
    pub use_incremental: bool,
    pub backup_dir: Option<PathBuf>,
    pub download_only: bool,
}

pub type ProgressCallback = Box<dyn Fn(UpgradeStep, &str) + Send + Sync>;

#[derive(Debug, Clone)]
pub enum UpgradeStep {
    CheckingUpdates,
    CreatingBackup,
    StoppingServices,
    DownloadingUpdate,
    ExtractingUpdate,
    LoadingImages,
    StartingServices,
    VerifyingServices,
    CleaningUp,
    Completed,
    Failed(String),
}

#[derive(Debug)]
pub struct UpgradeResult {
    pub success: bool,
    pub from_version: String,
    pub to_version: String,
    pub error: Option<String>,
    pub backup_id: Option<i64>,
}

impl UpgradeManager {
    pub fn new(
        config: AppConfig,
        config_path: PathBuf,
        docker_manager: DockerManager,
        backup_manager: BackupManager,
        api_client: ApiClient,
        database: Database,
    ) -> Self {
        Self {
            config,
            config_path,
            docker_manager,
            backup_manager,
            api_client,
            database,
        }
    }

    pub async fn check_for_updates(&self) -> Result<bool> {
        info!("检查服务更新...");
        let current_version = &self.config.versions.docker_service;
        let version_info = self.api_client.check_docker_version(current_version).await?;
        Ok(version_info.has_update)
    }

    pub async fn upgrade_service(
        &mut self,
        options: UpgradeOptions,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<UpgradeResult> {
        let from_version = self.config.versions.docker_service.clone();
        let callback = progress_callback.as_ref();

        self.send_progress(callback, UpgradeStep::CheckingUpdates, "检查服务更新");
        let has_update = self.check_for_updates().await?;
        
        if !has_update {
            info!("服务已是最新版本");
            self.send_progress(callback, UpgradeStep::Completed, "服务已是最新版本");
            return Ok(UpgradeResult {
                success: true,
                from_version: from_version.clone(),
                to_version: from_version,
                error: None,
                backup_id: None,
            });
        }

        info!("发现新版本可用");
        let download_url = self.api_client.get_service_download_url();
        let to_version = "latest".to_string();

        self.perform_upgrade_flow(options, &from_version, &to_version, &download_url, callback)
            .await
    }

    pub async fn upgrade_docker_service_direct(
        &mut self,
        options: UpgradeOptions,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<UpgradeResult> {
        let from_version = self.config.versions.docker_service.clone();
        let to_version = "latest".to_string();
        let download_url = self.api_client.get_service_download_url();
        let callback = progress_callback.as_ref();

        self.perform_upgrade_flow(options, &from_version, &to_version, &download_url, callback)
            .await
    }

    async fn perform_upgrade_flow(
        &mut self,
        options: UpgradeOptions,
        from_version: &str,
        to_version: &str,
        download_url: &str,
        progress_callback: Option<&ProgressCallback>,
    ) -> Result<UpgradeResult> {
        let mut backup_id: Option<i64> = None;
        let mut services_stopped = false;
        let temp_dir = TempDir::new()?;

        let result: Result<()> = async {
            backup_id = self.create_backup_if_needed(&options, progress_callback).await?;

            if options.download_only {
                let package_filename = download_url.split('/').last().unwrap_or(crate::constants::upgrade::DEFAULT_UPDATE_PACKAGE);
                let download_path = temp_dir.path().join(package_filename);
                self.download_and_extract(download_url, &download_path, temp_dir.path(), progress_callback).await?;
                self.send_progress(progress_callback, UpgradeStep::Completed, "仅下载模式完成");
                return Ok(());
            }

            self.stop_services(progress_callback).await?;
            services_stopped = true;

            let package_filename = download_url.split('/').last().unwrap_or(crate::constants::upgrade::DEFAULT_UPDATE_PACKAGE);
            let download_path = temp_dir.path().join(package_filename);
            self.download_and_extract(download_url, &download_path, temp_dir.path(), progress_callback).await?;
            
            self.load_new_images(temp_dir.path(), progress_callback).await?;
            self.apply_files(temp_dir.path(), progress_callback).await?;
            self.start_services(progress_callback).await?;
            self.verify_services(progress_callback).await?;
            self.cleanup(&download_path, progress_callback).await?;
            
            Ok(())
        }
        .await;

        if let Err(e) = result {
            warn!("升级失败: {}. 正在尝试回滚...", e);
            self.send_progress(
                progress_callback,
                UpgradeStep::Failed("升级失败，正在回滚...".to_string()),
                &e.to_string(),
            );
            
            if let Some(id) = backup_id {
                match self.rollback_from_backup(id, progress_callback).await {
                    Ok(_) => {
                        let final_error_msg = format!("升级失败 ({})，但已成功回滚到备份 ID {}", e, id);
                        return Ok(UpgradeResult {
                            success: false, from_version: from_version.to_string(), to_version: to_version.to_string(),
                            error: Some(final_error_msg), backup_id,
                        });
                    }
                    Err(rollback_err) => {
                        let final_error_msg = format!("升级失败 ({})，且回滚操作也失败了: {}", e, rollback_err);
                        return Ok(UpgradeResult {
                            success: false, from_version: from_version.to_string(), to_version: to_version.to_string(),
                            error: Some(final_error_msg), backup_id,
                        });
                    }
                }
            } else if services_stopped {
                warn!("升级失败，且没有备份。正在尝试重启服务...");
                self.send_progress(progress_callback, UpgradeStep::Failed("正在重启服务...".to_string()), "");
                if let Err(restart_err) = self.docker_manager.start_services().await {
                    warn!("重启服务也失败了: {}", restart_err);
                    let final_error_msg = format!("升级失败 ({}), 并且无法重启原始服务: {}", e, restart_err);
                    return Ok(UpgradeResult {
                        success: false, from_version: from_version.to_string(), to_version: to_version.to_string(),
                        error: Some(final_error_msg), backup_id,
                    });
                } else {
                    let final_error_msg = format!("升级失败 ({})，服务已重启", e);
                     return Ok(UpgradeResult {
                        success: false, from_version: from_version.to_string(), to_version: to_version.to_string(),
                        error: Some(final_error_msg), backup_id,
                    });
                }
            }

             return Ok(UpgradeResult {
                success: false, from_version: from_version.to_string(), to_version: to_version.to_string(),
                error: Some(e.to_string()), backup_id,
            });
        }
        
        if options.download_only {
             return Ok(UpgradeResult {
                success: true, from_version: from_version.to_string(), to_version: to_version.to_string(),
                error: None, backup_id,
            });
        }

        // 配置文件版本更新功能暂时不实现，可以手动修改配置文件
        self.send_progress(progress_callback, UpgradeStep::Completed, "升级成功完成");

        Ok(UpgradeResult {
            success: true, from_version: from_version.to_string(), to_version: to_version.to_string(),
            error: None, backup_id,
        })
    }

    async fn rollback_from_backup(&self, backup_id: i64, progress_callback: Option<&ProgressCallback>) -> Result<()> {
        warn!("从备份 ID {} 进行回滚。", backup_id);
        self.send_progress(progress_callback, UpgradeStep::Failed(format!("正在从备份 {} 回滚...", backup_id)), "");

        let docker_dir = self.docker_manager.get_working_directory()
            .ok_or_else(|| DuckError::Custom("无法确定 Docker 工作目录".to_string()))?;

        let options = RestoreOptions {
            target_dir: docker_dir.to_path_buf(),
            force_overwrite: true,
        };

        self.docker_manager.stop_services().await.ok(); 
        self.backup_manager.restore_from_backup(backup_id, options).await?;
        
        info!("回滚成功。正在尝试重启旧服务...");
        self.send_progress(progress_callback, UpgradeStep::Failed("回滚成功，正在重启服务...".to_string()), "");
        self.docker_manager.start_services().await?;
        info!("回滚并重启服务成功。");
        Ok(())
    }

    async fn create_backup_if_needed(
        &self,
        options: &UpgradeOptions,
        progress_callback: Option<&ProgressCallback>,
    ) -> Result<Option<i64>> {
        if options.skip_backup {
            info!("跳过备份步骤");
            return Ok(None);
        }

        self.send_progress(progress_callback, UpgradeStep::CreatingBackup, "正在创建备份");
        let docker_dir = self.docker_manager.get_working_directory()
            .ok_or_else(|| DuckError::Custom("无法确定 Docker 工作目录".to_string()))?;

        let backup_options = BackupOptions {
            backup_type: BackupType::PreUpgrade,
            service_version: self.config.versions.docker_service.clone(),
            source_dirs: vec![docker_dir.to_path_buf()],
            compression_level: 6,
        };

        let backup_record = self.backup_manager.create_backup(backup_options).await?;
        info!("备份创建成功，ID: {}", backup_record.id);
        Ok(Some(backup_record.id))
    }

    async fn stop_services(&self, progress_callback: Option<&ProgressCallback>) -> Result<()> {
        self.send_progress(progress_callback, UpgradeStep::StoppingServices, "正在停止服务");
        self.docker_manager.stop_services().await
    }

    async fn start_services(&self, progress_callback: Option<&ProgressCallback>) -> Result<()> {
        self.send_progress(progress_callback, UpgradeStep::StartingServices, "正在启动服务");
        self.docker_manager.start_services().await
    }

    async fn download_and_extract(
        &self,
        download_url: &str,
        download_path: &Path,
        extract_dir: &Path,
        progress_callback: Option<&ProgressCallback>,
    ) -> Result<()> {
        self.send_progress(progress_callback, UpgradeStep::DownloadingUpdate, "正在下载更新");
        self.api_client.download_service_update(download_path).await?;
        
        self.send_progress(progress_callback, UpgradeStep::ExtractingUpdate, "正在解压文件");

        let path_for_blocking = download_path.to_path_buf();
        let extract_dir_for_blocking = extract_dir.to_path_buf();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let file = std::fs::File::open(path_for_blocking)?;
            zip_extract::extract(file, &extract_dir_for_blocking, true)
                .map_err(|e| DuckError::Custom(format!("解压失败: {}", e)))
        }).await??;

        Ok(())
    }

    async fn load_new_images(&self, temp_dir: &Path, progress_callback: Option<&ProgressCallback>) -> Result<()> {
        self.send_progress(progress_callback, UpgradeStep::LoadingImages, "正在加载镜像");
        let images_dir = temp_dir.join("images");
        if !images_dir.exists() {
            info!("未找到 'images' 目录，跳过加载镜像步骤。");
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(images_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "tar") {
                info!("加载镜像: {}", path.display());
                self.docker_manager.load_image(&path).await?;
            }
        }
        Ok(())
    }

    async fn apply_files(&self, temp_dir: &Path, _progress_callback: Option<&ProgressCallback>) -> Result<()> {
        let target_dir = self.docker_manager.get_working_directory()
            .ok_or_else(|| DuckError::Custom("无法确定 Docker 工作目录".to_string()))?;
        self.copy_directory_with_data_preservation(temp_dir, target_dir).await
    }

    async fn verify_services(&self, progress_callback: Option<&ProgressCallback>) -> Result<()> {
        self.send_progress(progress_callback, UpgradeStep::VerifyingServices, "正在验证服务状态");
        tokio::time::sleep(std::time::Duration::from_secs(timeout::SERVICE_VERIFY_WAIT)).await;
        self.docker_manager.check_services_health().await
    }

    async fn cleanup(&self, download_path: &Path, progress_callback: Option<&ProgressCallback>) -> Result<()> {
        self.send_progress(progress_callback, UpgradeStep::CleaningUp, "正在清理临时文件");
        if let Some(parent) = download_path.parent() {
             tokio::fs::remove_dir_all(parent).await?;
        }
        Ok(())
    }

    fn copy_directory_with_data_preservation<'a>(
        &'a self,
        src: &'a Path,
        dest: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            if !dest.exists() {
                tokio::fs::create_dir_all(dest).await?;
            }

            let mut entries = tokio::fs::read_dir(src).await?;
            while let Some(entry) = entries.next_entry().await? {
                let src_path = entry.path();
                let dest_path = dest.join(entry.file_name());

                if dest_path.strip_prefix(dest).map_or(false, |p| p.starts_with("data")) && dest_path.exists() {
                    info!("保留现有数据文件: {}", dest_path.display());
                    continue;
                }
                
                if src_path.is_dir() {
                    self.copy_directory_with_data_preservation(&src_path, &dest_path).await?;
                } else {
                    if let Some(parent) = dest_path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    tokio::fs::copy(&src_path, &dest_path).await?;
                }
            }

            Ok(())
        })
    }

    fn send_progress(
        &self,
        progress_callback: Option<&ProgressCallback>,
        step: UpgradeStep,
        message: &str,
    ) {
        if let Some(callback) = progress_callback {
            callback(step, message);
        }
    }
} 