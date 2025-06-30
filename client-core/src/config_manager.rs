use crate::database::Database;
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;

/// 自动备份配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoBackupConfig {
    pub enabled: bool,
    pub cron_expression: String,
    pub last_backup_at: Option<DateTime<Utc>>,
    pub consecutive_failures: i32,
    pub max_failures: i32,
}

/// 自动升级任务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoUpgradeTask {
    pub id: Option<i32>,
    pub task_type: String, // "immediate" | "delayed" | "scheduled"
    pub target_version: Option<String>,
    pub scheduled_at: DateTime<Utc>,
    pub delay_amount: Option<i32>,
    pub delay_unit: Option<String>,
    pub status: String,
    pub progress: i32,
    pub error_message: Option<String>,
    pub backup_created: bool,
    pub backup_id: Option<i32>,
}

/// 配置管理器
pub struct ConfigManager<'a> {
    db: &'a Database,
}

impl<'a> ConfigManager<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    // ========================================
    // 自动备份配置管理
    // ========================================

    /// 获取自动备份配置
    pub async fn get_auto_backup_config(&self) -> Result<AutoBackupConfig> {
        // 获取各个配置项
        let enabled = self
            .db
            .get_config("auto_backup.enabled")
            .await?
            .unwrap_or_else(|| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        let cron_expression = self
            .db
            .get_config("auto_backup.cron_expression")
            .await?
            .unwrap_or_else(|| "0 2 * * *".to_string());

        let last_backup_at = self
            .db
            .get_config("auto_backup.last_backup_at")
            .await?
            .and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

        let consecutive_failures = self
            .db
            .get_config("auto_backup.consecutive_failures")
            .await?
            .unwrap_or_else(|| "0".to_string())
            .parse::<i32>()
            .unwrap_or(0);

        let max_failures = self
            .db
            .get_config("auto_backup.max_failures")
            .await?
            .unwrap_or_else(|| "3".to_string())
            .parse::<i32>()
            .unwrap_or(3);

        Ok(AutoBackupConfig {
            enabled,
            cron_expression,
            last_backup_at,
            consecutive_failures,
            max_failures,
        })
    }

    /// 更新自动备份启用状态
    pub async fn set_auto_backup_enabled(&self, enabled: bool) -> Result<()> {
        self.db
            .set_config("auto_backup.enabled", &enabled.to_string())
            .await?;
        tracing::info!("自动备份启用状态已更新: {}", enabled);
        Ok(())
    }

    /// 更新自动备份cron表达式
    pub async fn set_auto_backup_cron(&self, cron_expression: &str) -> Result<()> {
        self.db
            .set_config("auto_backup.cron_expression", cron_expression)
            .await?;
        tracing::info!("自动备份cron表达式已更新: {}", cron_expression);
        Ok(())
    }

    /// 记录备份执行时间
    pub async fn update_last_backup_time(
        &self,
        backup_time: DateTime<Utc>,
        success: bool,
    ) -> Result<()> {
        self.db
            .set_config("auto_backup.last_backup_at", &backup_time.to_rfc3339())
            .await?;

        if success {
            self.db
                .set_config("auto_backup.consecutive_failures", "0")
                .await?;
        } else {
            let current_failures = self
                .db
                .get_config("auto_backup.consecutive_failures")
                .await?
                .unwrap_or_else(|| "0".to_string())
                .parse::<i32>()
                .unwrap_or(0);
            self.db
                .set_config(
                    "auto_backup.consecutive_failures",
                    &(current_failures + 1).to_string(),
                )
                .await?;
        }

        Ok(())
    }

    // ========================================
    // 自动升级任务管理
    // ========================================

    /// 创建自动升级任务
    pub async fn create_auto_upgrade_task(&self, task: &AutoUpgradeTask) -> Result<String> {
        // 如果指定了backup_id，验证其有效性（应用层数据完整性保障）
        if let Some(backup_id) = task.backup_id {
            self.validate_backup_id(backup_id).await?;
        }

        let task_id = uuid::Uuid::new_v4().to_string();
        let task_json = serde_json::to_string(task)
            .map_err(|e| crate::error::DuckError::custom(format!("序列化任务失败: {}", e)))?;

        let key = format!("auto_upgrade_task.{}", task_id);
        self.db.set_config(&key, &task_json).await?;

        tracing::info!("创建自动升级任务，ID: {}", task_id);
        Ok(task_id)
    }

    /// 更新升级任务状态
    pub async fn update_upgrade_task_status(
        &self,
        task_id: &str,
        status: &str,
        progress: Option<i32>,
        error_message: Option<&str>,
    ) -> Result<()> {
        let key = format!("auto_upgrade_task.{}", task_id);

        if let Some(task_json) = self.db.get_config(&key).await? {
            let mut task: AutoUpgradeTask = serde_json::from_str(&task_json)
                .map_err(|e| crate::error::DuckError::custom(format!("反序列化任务失败: {}", e)))?;

            task.status = status.to_string();
            if let Some(progress) = progress {
                task.progress = progress;
            }
            if let Some(error_message) = error_message {
                task.error_message = Some(error_message.to_string());
            }

            let updated_json = serde_json::to_string(&task)
                .map_err(|e| crate::error::DuckError::custom(format!("序列化任务失败: {}", e)))?;
            self.db.set_config(&key, &updated_json).await?;

            tracing::info!("升级任务 {} 状态已更新: {}", task_id, status);
        }

        Ok(())
    }

    /// 获取待执行的升级任务
    pub async fn get_pending_upgrade_tasks(&self) -> Result<Vec<(String, AutoUpgradeTask)>> {
        // 注意：这是一个简化实现，在真实场景中，您可能想要一个更高效的方式来查询任务
        // 这里我们需要获取所有以 "auto_upgrade_task." 开头的配置项
        // 由于当前的Database接口不支持前缀查询，这里返回空列表
        // 在实际应用中，您可能需要扩展Database接口来支持这种查询

        tracing::warn!(
            "get_pending_upgrade_tasks: 当前实现不支持查询所有任务，需要扩展Database接口"
        );
        Ok(vec![])
    }

    /// 删除升级任务
    pub async fn delete_upgrade_task(&self, task_id: &str) -> Result<()> {
        let key = format!("auto_upgrade_task.{}", task_id);
        // 注意：当前Database接口没有删除配置的方法，这里通过设置空值来"删除"
        self.db.set_config(&key, "").await?;
        tracing::info!("升级任务 {} 已删除", task_id);
        Ok(())
    }

    // ========================================
    // 通用配置管理
    // ========================================

    /// 获取配置项
    pub async fn get_config(&self, key: &str) -> Result<Option<String>> {
        self.db.get_config(key).await
    }

    /// 设置配置项
    pub async fn set_config(&self, key: &str, value: &str) -> Result<()> {
        self.db.set_config(key, value).await
    }

    /// 获取所有自动备份相关配置
    pub async fn get_all_auto_backup_configs(&self) -> Result<Vec<(String, String)>> {
        let keys = [
            "auto_backup.enabled",
            "auto_backup.cron_expression",
            "auto_backup.last_backup_at",
            "auto_backup.consecutive_failures",
            "auto_backup.max_failures",
        ];

        let mut configs = Vec::new();
        for key in keys.iter() {
            if let Some(value) = self.db.get_config(key).await? {
                configs.push((key.to_string(), value));
            }
        }

        Ok(configs)
    }

    /// 显示自动备份状态信息
    pub async fn get_auto_backup_status_info(&self) -> Result<String> {
        let config = self.get_auto_backup_config().await?;

        let status_info = format!(
            "自动备份配置状态:\n\
            - 启用状态: {}\n\
            - Cron表达式: {}\n\
            - 上次备份: {}\n\
            - 连续失败次数: {}/{}\n",
            if config.enabled {
                "已启用"
            } else {
                "已禁用"
            },
            config.cron_expression,
            config
                .last_backup_at
                .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "从未执行".to_string()),
            config.consecutive_failures,
            config.max_failures
        );

        Ok(status_info)
    }

    // ========================================
    // 内部辅助方法 - 数据完整性验证
    // ========================================

    /// 验证backup_id的有效性（应用层数据完整性保障）
    async fn validate_backup_id(&self, backup_id: i32) -> Result<()> {
        // 检查backup_id是否为正数
        if backup_id <= 0 {
            return Err(crate::error::DuckError::custom(format!(
                "无效的backup_id: {}，必须为正数",
                backup_id
            )));
        }

        // 从数据库查询备份记录
        let backup_record = match self.db.get_backup_by_id(backup_id as i64).await? {
            Some(record) => record,
            None => {
                return Err(crate::error::DuckError::custom(format!(
                    "备份记录不存在: backup_id = {}",
                    backup_id
                )));
            }
        };

        // 检查备份状态
        if backup_record.status != crate::database::BackupStatus::Completed {
            return Err(crate::error::DuckError::custom(format!(
                "备份记录状态不正确: backup_id = {}，状态 = {:?}，只有完成状态的备份才能使用",
                backup_id, backup_record.status
            )));
        }

        // 检查备份文件是否真实存在
        let backup_file_path = std::path::Path::new(&backup_record.file_path);
        if !backup_file_path.exists() {
            return Err(crate::error::DuckError::custom(format!(
                "备份文件不存在: backup_id = {}，文件路径 = {}。\n可能文件已被移动或删除，请检查备份文件的完整性",
                backup_id, backup_record.file_path
            )));
        }

        // 检查文件是否为.zip格式（额外的安全检查）
        if !backup_record
            .file_path
            .to_lowercase()
            .ends_with(crate::constants::file_format::ZIP_EXTENSION)
        {
            tracing::warn!(
                "备份文件 {} 不是.zip格式，可能存在问题",
                backup_record.file_path
            );
        }

        // 简单验证ZIP文件头（验证文件确实是ZIP格式）
        if let Err(e) = Self::verify_zip_file_header(&backup_file_path) {
            return Err(crate::error::DuckError::custom(format!(
                "备份文件格式验证失败: backup_id = {}，文件路径 = {}，错误: {}",
                backup_id, backup_record.file_path, e
            )));
        }

        // 可选：检查文件大小是否合理（避免空文件或损坏文件）
        if let Ok(metadata) = backup_file_path.metadata() {
            let file_size = metadata.len();
            if file_size == 0 {
                return Err(crate::error::DuckError::custom(format!(
                    "备份文件为空: backup_id = {}，文件路径 = {}",
                    backup_id, backup_record.file_path
                )));
            }

            // 检查文件大小是否合理（至少应该有一些内容）
            if file_size < crate::constants::backup::MIN_ZIP_FILE_SIZE {
                // 至少100字节，一个有效的zip文件应该比这大
                tracing::warn!(
                    "备份文件 {} 大小过小 ({} bytes)，可能存在问题",
                    backup_record.file_path,
                    file_size
                );
            }

            tracing::debug!(
                "备份文件检查通过: backup_id = {}，文件路径 = {}，文件大小 = {} bytes",
                backup_id,
                backup_record.file_path,
                file_size
            );
        }

        tracing::info!(
            "备份ID {} 验证通过，文件路径: {}",
            backup_id,
            backup_record.file_path
        );
        Ok(())
    }

    /// 验证ZIP文件头部魔术字节
    fn verify_zip_file_header(file_path: &std::path::Path) -> Result<()> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(file_path)
            .map_err(|e| crate::error::DuckError::custom(format!("无法打开文件进行验证: {}", e)))?;

        let mut buffer = [0u8; 4];
        match file.read_exact(&mut buffer) {
            Ok(_) => {
                // ZIP文件的魔术字节验证
                use crate::constants::file_format;

                // 检查是否以PK开头（ZIP文件通用前缀）
                if buffer[0..2] == file_format::ZIP_MAGIC_PK_PREFIX {
                    // 检查是否是常见的ZIP文件头格式
                    if buffer == file_format::ZIP_MAGIC_LOCAL_HEADER {
                        tracing::debug!("ZIP文件头验证通过: 本地文件头格式");
                        Ok(())
                    } else if buffer == file_format::ZIP_MAGIC_CENTRAL_DIR_END {
                        tracing::debug!("ZIP文件头验证通过: 中央目录结束记录格式");
                        Ok(())
                    } else if buffer == file_format::ZIP_MAGIC_DATA_DESCRIPTOR {
                        tracing::debug!("ZIP文件头验证通过: 数据描述符格式");
                        Ok(())
                    } else {
                        // 还是PK开头，但可能是其他ZIP变体，也认为有效
                        tracing::debug!(
                            "ZIP文件头验证通过: PK格式但子类型未知 ({:#04x} {:#04x})",
                            buffer[2],
                            buffer[3]
                        );
                        Ok(())
                    }
                } else {
                    Err(crate::error::DuckError::custom(format!(
                        "文件不是有效的ZIP格式，文件头: {:#04x} {:#04x} {:#04x} {:#04x}",
                        buffer[0], buffer[1], buffer[2], buffer[3]
                    )))
                }
            }
            Err(e) => Err(crate::error::DuckError::custom(format!(
                "无法读取文件头进行验证: {}",
                e
            ))),
        }
    }
}
