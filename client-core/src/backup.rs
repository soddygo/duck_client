use crate::{
    DuckError, Result,
    container::DockerManager,
    database::{BackupRecord, BackupStatus, BackupType, Database},
};
use chrono::Utc;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 备份管理器
#[derive(Debug, Clone)]
pub struct BackupManager {
    storage_dir: PathBuf,
    database: Database,
    docker_manager: DockerManager,
}

/// 备份选项
#[derive(Debug, Clone)]
pub struct BackupOptions {
    /// 备份类型
    pub backup_type: BackupType,
    /// 服务版本
    pub service_version: String,
    /// 要备份的目录列表
    pub source_dirs: Vec<PathBuf>,
    /// 压缩级别 (0-9)
    pub compression_level: u32,
}

/// 恢复选项
#[derive(Debug, Clone)]
pub struct RestoreOptions {
    /// 目标目录
    pub target_dir: PathBuf,
    /// 是否强制覆盖
    pub force_overwrite: bool,
}

impl BackupManager {
    /// 创建新的备份管理器
    pub fn new(
        storage_dir: PathBuf,
        database: Database,
        docker_manager: DockerManager,
    ) -> Result<Self> {
        if !storage_dir.exists() {
            std::fs::create_dir_all(&storage_dir)?;
        }

        Ok(Self {
            storage_dir,
            database,
            docker_manager,
        })
    }

    /// 创建备份
    pub async fn create_backup(&self, options: BackupOptions) -> Result<BackupRecord> {
        // 检查所有源目录是否存在
        for source_dir in &options.source_dirs {
            if !source_dir.exists() {
                return Err(DuckError::Backup(format!(
                    "源目录不存在: {}",
                    source_dir.display()
                )));
            }
        }

        // 生成备份文件名（人类易读格式）
        let timestamp = Utc::now().format("%Y-%m-%d_%H-%M-%S");
        let backup_type_str = match options.backup_type {
            BackupType::Manual => "manual",
            BackupType::PreUpgrade => "pre-upgrade",
        };

        let backup_filename = format!(
            "backup_{}_v{}_{}.tar.gz",
            backup_type_str, options.service_version, timestamp
        );

        let backup_path = self.storage_dir.join(&backup_filename);

        tracing::info!("开始创建备份: {}", backup_path.display());

        // 执行备份
        match self
            .perform_backup(
                &options.source_dirs,
                &backup_path,
                options.compression_level,
            )
            .await
        {
            Ok(_) => {
                tracing::info!("备份创建成功: {}", backup_path.display());

                // 记录到数据库
                let record_id = self
                    .database
                    .create_backup_record(
                        backup_path.to_string_lossy().to_string(),
                        options.service_version,
                        options.backup_type,
                        BackupStatus::Completed,
                    )
                    .await?;

                // 获取创建的记录
                self.database
                    .get_backup_by_id(record_id)
                    .await?
                    .ok_or_else(|| DuckError::Backup("无法获取刚创建的备份记录".to_string()))
            }
            Err(e) => {
                tracing::error!("备份创建失败: {}", e);

                // 记录失败到数据库
                self.database
                    .create_backup_record(
                        backup_path.to_string_lossy().to_string(),
                        options.service_version,
                        options.backup_type,
                        BackupStatus::Failed,
                    )
                    .await?;

                Err(e)
            }
        }
    }

    /// 执行实际的备份操作
    async fn perform_backup(
        &self,
        source_dirs: &[PathBuf],
        backup_path: &Path,
        compression_level: u32,
    ) -> Result<()> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::fs::File;
        use tar::Builder;

        // 确保备份目录存在
        if let Some(parent) = backup_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // 在后台线程中执行压缩操作，避免阻塞异步运行时
        let source_dirs = source_dirs.to_vec();
        let backup_path = backup_path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let file = File::create(&backup_path)?;
            let compression = Compression::new(compression_level);
            let encoder = GzEncoder::new(file, compression);
            let mut archive = Builder::new(encoder);

            // 遍历所有源目录并添加到归档中
            for source_dir in &source_dirs {
                let dir_name = source_dir
                    .file_name()
                    .ok_or_else(|| DuckError::Backup("无法获取目录名".to_string()))?
                    .to_string_lossy();

                for entry in WalkDir::new(source_dir) {
                    let entry =
                        entry.map_err(|e| DuckError::Backup(format!("遍历目录失败: {e}")))?;
                    let path = entry.path();

                    if path.is_file() {
                        let relative_path = path
                            .strip_prefix(source_dir)
                            .map_err(|e| DuckError::Backup(format!("计算相对路径失败: {e}")))?;

                        // 在归档中保持目录结构：{dir_name}/{relative_path}
                        // 注意：tar归档内部使用Unix风格路径（/）是标准做法，跨平台兼容
                        let archive_path = if cfg!(windows) {
                            // 在Windows上，确保使用Unix风格的路径分隔符用于tar归档
                            format!(
                                "{}/{}",
                                dir_name,
                                relative_path.display().to_string().replace('\\', "/")
                            )
                        } else {
                            format!("{}/{}", dir_name, relative_path.display())
                        };

                        archive
                            .append_path_with_name(path, archive_path)
                            .map_err(|e| DuckError::Backup(format!("添加文件到归档失败: {e}")))?;
                    }
                }
            }

            archive
                .finish()
                .map_err(|e| DuckError::Backup(format!("完成归档失败: {e}")))?;

            Ok::<(), DuckError>(())
        })
        .await??;

        Ok(())
    }

    /// 从备份恢复
    pub async fn restore_from_backup(&self, backup_id: i64, options: RestoreOptions) -> Result<()> {
        // 获取备份记录
        let backup_record = self
            .database
            .get_backup_by_id(backup_id)
            .await?
            .ok_or_else(|| DuckError::Backup(format!("备份记录不存在: {backup_id}")))?;

        let backup_path = PathBuf::from(&backup_record.file_path);
        if !backup_path.exists() {
            return Err(DuckError::Backup(format!(
                "备份文件不存在: {}",
                backup_path.display()
            )));
        }

        tracing::info!("开始从备份恢复: {}", backup_path.display());

        // 停止服务，准备恢复
        tracing::info!("正在停止服务...");
        self.docker_manager.stop_services().await?;

        // 检查目标目录
        if options.target_dir.exists() {
            if options.force_overwrite {
                tracing::warn!(
                    "目标目录 {} 已存在，将被清空和覆盖。",
                    options.target_dir.display()
                );
                tokio::fs::remove_dir_all(&options.target_dir).await?;
            } else {
                return Err(DuckError::Backup(
                    "目标目录已存在，请使用 force_overwrite 选项".to_string(),
                ));
            }
        }

        // 执行恢复
        self.perform_restore(&backup_path, &options.target_dir)
            .await?;

        // 启动服务
        tracing::info!("恢复完成，正在启动服务...");
        self.docker_manager.start_services().await?;

        tracing::info!("服务已成功恢复并启动: {}", options.target_dir.display());
        Ok(())
    }

    /// 执行实际的恢复操作
    async fn perform_restore(&self, backup_path: &Path, target_dir: &Path) -> Result<()> {
        use flate2::read::GzDecoder;
        use std::fs::File;
        use tar::Archive;

        // 确保目标目录存在
        tokio::fs::create_dir_all(target_dir).await?;

        let backup_path = backup_path.to_path_buf();
        let target_dir = target_dir.to_path_buf();

        // 在后台线程中执行解压操作
        tokio::task::spawn_blocking(move || {
            let file = File::open(&backup_path)?;
            let decoder = GzDecoder::new(file);
            let mut archive = Archive::new(decoder);

            archive
                .unpack(&target_dir)
                .map_err(|e| DuckError::Backup(format!("解压归档失败: {e}")))?;

            Ok::<(), DuckError>(())
        })
        .await??;

        Ok(())
    }

    /// 获取所有备份记录
    pub async fn list_backups(&self) -> Result<Vec<BackupRecord>> {
        self.database.get_all_backups().await
    }

    /// 删除备份
    pub async fn delete_backup(&self, backup_id: i64) -> Result<()> {
        // 获取备份记录
        let backup_record = self
            .database
            .get_backup_by_id(backup_id)
            .await?
            .ok_or_else(|| DuckError::Backup(format!("备份记录不存在: {backup_id}")))?;

        let backup_path = PathBuf::from(&backup_record.file_path);

        // 删除文件
        if backup_path.exists() {
            tokio::fs::remove_file(&backup_path).await?;
            tracing::info!("删除备份文件: {}", backup_path.display());
        }

        // 从数据库中删除记录
        self.database.delete_backup_record(backup_id).await?;

        Ok(())
    }

    /// 获取备份文件大小
    pub async fn get_backup_size(&self, backup_id: i64) -> Result<u64> {
        let backup_record = self
            .database
            .get_backup_by_id(backup_id)
            .await?
            .ok_or_else(|| DuckError::Backup(format!("备份记录不存在: {backup_id}")))?;

        let backup_path = PathBuf::from(&backup_record.file_path);
        if !backup_path.exists() {
            return Err(DuckError::Backup(format!(
                "备份文件不存在: {}",
                backup_path.display()
            )));
        }

        let metadata = tokio::fs::metadata(&backup_path).await?;
        Ok(metadata.len())
    }

    /// 验证备份文件完整性
    pub async fn verify_backup(&self, backup_id: i64) -> Result<bool> {
        let backup_record = self
            .database
            .get_backup_by_id(backup_id)
            .await?
            .ok_or_else(|| DuckError::Backup(format!("备份记录不存在: {backup_id}")))?;

        let backup_path = PathBuf::from(&backup_record.file_path);
        if !backup_path.exists() {
            return Ok(false);
        }

        // 尝试打开并验证归档文件
        let backup_path = backup_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            use flate2::read::GzDecoder;
            use std::fs::File;
            use tar::Archive;

            let file = File::open(&backup_path)?;
            let decoder = GzDecoder::new(file);
            let mut archive = Archive::new(decoder);

            // 尝试列出所有条目来验证归档完整性
            for entry in archive.entries()? {
                let _entry = entry?;
                // 如果能成功遍历所有条目，说明归档是完整的
            }

            Ok::<bool, DuckError>(true)
        })
        .await??;

        Ok(result)
    }

    /// 检查并迁移备份存储目录
    pub async fn migrate_storage_directory(&self, new_storage_dir: &Path) -> Result<()> {
        if new_storage_dir == self.storage_dir {
            return Ok(()); // 没有变化
        }

        tracing::info!(
            "开始迁移备份存储目录: {} -> {}",
            self.storage_dir.display(),
            new_storage_dir.display()
        );

        // 创建新目录
        tokio::fs::create_dir_all(new_storage_dir).await?;

        // 获取所有备份记录
        let backups = self.list_backups().await?;

        for backup in backups {
            let old_path = PathBuf::from(&backup.file_path);
            if old_path.exists() {
                let filename = old_path
                    .file_name()
                    .ok_or_else(|| DuckError::Backup("无法获取备份文件名".to_string()))?;
                let new_path = new_storage_dir.join(filename);

                // 移动文件
                tokio::fs::rename(&old_path, &new_path).await?;
                tracing::info!(
                    "迁移备份文件: {} -> {}",
                    old_path.display(),
                    new_path.display()
                );

                // 更新数据库中的路径
                self.database
                    .update_backup_file_path(backup.id, new_path.to_string_lossy().to_string())
                    .await?;
            }
        }

        tracing::info!("备份存储目录迁移完成");
        Ok(())
    }

    /// 获取存储目录
    pub fn get_storage_dir(&self) -> &Path {
        &self.storage_dir
    }

    /// 估算目录大小
    pub async fn estimate_backup_size(&self, source_dir: &Path) -> Result<u64> {
        let source_dir = source_dir.to_path_buf();

        let total_size = tokio::task::spawn_blocking(move || {
            let mut total = 0u64;

            for entry in WalkDir::new(&source_dir).into_iter().flatten() {
                if entry.path().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        total += metadata.len();
                    }
                }
            }

            total
        })
        .await?;

        // 考虑压缩率，估算压缩后大小约为原大小的 30-50%
        Ok(total_size / 2)
    }
}
