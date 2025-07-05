use crate::DatabaseManager;
use crate::database::Database;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};
// chrono 相关导入由其他地方提供

/// 配置值类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigType {
    String,
    Number,
    Boolean,
    Object,
    Array,
}

impl ConfigType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigType::String => "STRING",
            ConfigType::Number => "NUMBER",
            ConfigType::Boolean => "BOOLEAN",
            ConfigType::Object => "OBJECT",
            ConfigType::Array => "ARRAY",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "STRING" => Some(ConfigType::String),
            "NUMBER" => Some(ConfigType::Number),
            "BOOLEAN" => Some(ConfigType::Boolean),
            "OBJECT" => Some(ConfigType::Object),
            "ARRAY" => Some(ConfigType::Array),
            _ => None,
        }
    }
}

/// 配置项结构
#[derive(Debug, Clone)]
pub struct ConfigItem {
    pub key: String,
    pub value: Value,
    pub config_type: ConfigType,
    pub category: String,
    pub description: Option<String>,
    pub is_system_config: bool,
    pub is_user_editable: bool,
    pub validation_rule: Option<String>,
    pub default_value: Option<Value>,
}

/// 配置更新请求
#[derive(Debug, Clone)]
pub struct ConfigUpdateRequest {
    pub key: String,
    pub value: Value,
    pub validate: bool,
}

/// 数据库连接枚举
pub enum DatabaseConnection {
    DatabaseManager(DatabaseManager),
    Database(Database),
}

impl DatabaseConnection {
    /// 执行读操作并支持重试
    pub async fn read_with_retry<F, R>(&self, operation: F) -> Result<R>
    where
        F: Fn(&duckdb::Connection) -> duckdb::Result<R> + Send + Sync,
        R: Send,
    {
        match self {
            DatabaseConnection::DatabaseManager(db) => db.read_with_retry(operation).await,
            DatabaseConnection::Database(_db) => {
                // 对于传统数据库，我们暂时返回默认值或错误
                Err(crate::error::DuckError::custom(
                    "传统数据库连接暂不支持配置管理功能",
                ))
            }
        }
    }

    /// 执行写操作并支持重试
    pub async fn write_with_retry<F, R>(&self, operation: F) -> Result<R>
    where
        F: Fn(&duckdb::Connection) -> duckdb::Result<R> + Send + Sync,
        R: Send,
    {
        match self {
            DatabaseConnection::DatabaseManager(db) => db.write_with_retry(operation).await,
            DatabaseConnection::Database(_db) => {
                // 对于传统数据库，我们暂时返回默认值或错误
                Err(crate::error::DuckError::custom(
                    "传统数据库连接暂不支持配置管理功能",
                ))
            }
        }
    }

    /// 执行批量写操作并支持重试
    pub async fn batch_write_with_retry<F, R>(&self, operations: F) -> Result<R>
    where
        F: Fn(&duckdb::Connection) -> duckdb::Result<R> + Send + Sync,
        R: Send,
    {
        match self {
            DatabaseConnection::DatabaseManager(db) => db.batch_write_with_retry(operations).await,
            DatabaseConnection::Database(_db) => {
                // 对于传统数据库，我们暂时返回默认值或错误
                Err(crate::error::DuckError::custom(
                    "传统数据库连接暂不支持配置管理功能",
                ))
            }
        }
    }
}

/// 统一配置管理器
///
/// 功能特性：
/// - 强类型配置读取方法
/// - 权限验证和类型验证  
/// - 内存缓存机制
/// - 批量配置更新
/// - 按分类查询配置
pub struct ConfigManager {
    db: DatabaseConnection,
    /// 内存缓存：key -> ConfigItem
    cache: Arc<RwLock<HashMap<String, ConfigItem>>>,
    /// 缓存是否已初始化
    cache_initialized: Arc<RwLock<bool>>,
}

impl ConfigManager {
    /// 创建新的配置管理器 (使用新的 DatabaseManager)
    pub fn new(db: DatabaseManager) -> Self {
        Self {
            db: DatabaseConnection::DatabaseManager(db),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// 创建新的配置管理器 (使用传统的 Database)
    pub fn new_with_database(db: Database) -> Self {
        Self {
            db: DatabaseConnection::Database(db),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// 初始化缓存（从数据库加载所有配置）
    pub async fn initialize_cache(&self) -> Result<()> {
        debug!("正在初始化配置缓存...");

        let configs = match &self.db {
            DatabaseConnection::DatabaseManager(db) => {
                db.read_with_retry(|conn| {
                    let mut stmt = conn.prepare(
                        "SELECT config_key, config_value, config_type, category, description, 
                                is_system_config, is_user_editable, validation_rule, default_value 
                         FROM app_config",
                    )?;

                    let config_iter = stmt.query_map([], |row| {
                        let key: String = row.get(0)?;
                        let value_str: String = row.get(1)?;
                        let type_str: String = row.get(2)?;
                        let category: String = row.get(3)?;
                        let description: Option<String> = row.get(4)?;
                        let is_system: bool = row.get(5)?;
                        let is_editable: bool = row.get(6)?;
                        let validation: Option<String> = row.get(7)?;
                        let default_str: Option<String> = row.get(8)?;

                        // 解析JSON值
                        let value: Value = serde_json::from_str(&value_str).map_err(|e| {
                            duckdb::Error::InvalidParameterName(format!("JSON解析失败: {}", e))
                        })?;

                        let default_value = if let Some(default_str) = default_str {
                            Some(serde_json::from_str(&default_str).map_err(|e| {
                                duckdb::Error::InvalidParameterName(format!(
                                    "默认值JSON解析失败: {}",
                                    e
                                ))
                            })?)
                        } else {
                            None
                        };

                        let config_type = ConfigType::from_str(&type_str).ok_or_else(|| {
                            duckdb::Error::InvalidParameterName(format!(
                                "无效的配置类型: {}",
                                type_str
                            ))
                        })?;

                        Ok(ConfigItem {
                            key: key.clone(),
                            value,
                            config_type,
                            category,
                            description,
                            is_system_config: is_system,
                            is_user_editable: is_editable,
                            validation_rule: validation,
                            default_value,
                        })
                    })?;

                    let mut configs = Vec::new();
                    for config in config_iter {
                        configs.push(config?);
                    }
                    Ok(configs)
                })
                .await?
            }
            DatabaseConnection::Database(_db) => {
                // 对于传统的 Database，我们暂时返回空的配置列表
                // 这是为了保持向后兼容性，避免破坏现有代码
                warn!("传统数据库连接暂不支持配置管理功能");
                Vec::new()
            }
        };

        // 更新缓存
        let mut cache = self.cache.write().await;
        cache.clear();
        for config in configs {
            cache.insert(config.key.clone(), config);
        }

        // 标记缓存已初始化
        *self.cache_initialized.write().await = true;

        debug!("配置缓存初始化完成，加载了 {} 个配置项", cache.len());
        Ok(())
    }

    /// 确保缓存已初始化
    async fn ensure_cache_initialized(&self) -> Result<()> {
        let initialized = *self.cache_initialized.read().await;
        if !initialized {
            self.initialize_cache().await?;
        }
        Ok(())
    }

    /// 获取字符串类型配置
    pub async fn get_string(&self, key: &str) -> Result<Option<String>> {
        self.ensure_cache_initialized().await?;

        let cache = self.cache.read().await;
        if let Some(config) = cache.get(key) {
            match &config.value {
                Value::String(s) => Ok(Some(s.clone())),
                _ => {
                    warn!("配置项 {} 不是字符串类型: {:?}", key, config.value);
                    Ok(None)
                }
            }
        } else {
            debug!("配置项 {} 不存在", key);
            Ok(None)
        }
    }

    /// 获取数字类型配置
    pub async fn get_number(&self, key: &str) -> Result<Option<f64>> {
        self.ensure_cache_initialized().await?;

        let cache = self.cache.read().await;
        if let Some(config) = cache.get(key) {
            match &config.value {
                Value::Number(n) => Ok(n.as_f64()),
                _ => {
                    warn!("配置项 {} 不是数字类型: {:?}", key, config.value);
                    Ok(None)
                }
            }
        } else {
            debug!("配置项 {} 不存在", key);
            Ok(None)
        }
    }

    /// 获取整数类型配置
    pub async fn get_integer(&self, key: &str) -> Result<Option<i64>> {
        self.ensure_cache_initialized().await?;

        let cache = self.cache.read().await;
        if let Some(config) = cache.get(key) {
            match &config.value {
                Value::Number(n) => Ok(n.as_i64()),
                _ => {
                    warn!("配置项 {} 不是数字类型: {:?}", key, config.value);
                    Ok(None)
                }
            }
        } else {
            debug!("配置项 {} 不存在", key);
            Ok(None)
        }
    }

    /// 获取布尔类型配置
    pub async fn get_bool(&self, key: &str) -> Result<Option<bool>> {
        self.ensure_cache_initialized().await?;

        let cache = self.cache.read().await;
        if let Some(config) = cache.get(key) {
            match &config.value {
                Value::Bool(b) => Ok(Some(*b)),
                _ => {
                    warn!("配置项 {} 不是布尔类型: {:?}", key, config.value);
                    Ok(None)
                }
            }
        } else {
            debug!("配置项 {} 不存在", key);
            Ok(None)
        }
    }

    /// 获取对象类型配置
    pub async fn get_object(&self, key: &str) -> Result<Option<Value>> {
        self.ensure_cache_initialized().await?;

        let cache = self.cache.read().await;
        if let Some(config) = cache.get(key) {
            match &config.value {
                Value::Object(_) => Ok(Some(config.value.clone())),
                _ => {
                    warn!("配置项 {} 不是对象类型: {:?}", key, config.value);
                    Ok(None)
                }
            }
        } else {
            debug!("配置项 {} 不存在", key);
            Ok(None)
        }
    }

    /// 获取数组类型配置
    pub async fn get_array(&self, key: &str) -> Result<Option<Vec<Value>>> {
        self.ensure_cache_initialized().await?;

        let cache = self.cache.read().await;
        if let Some(config) = cache.get(key) {
            match &config.value {
                Value::Array(arr) => Ok(Some(arr.clone())),
                _ => {
                    warn!("配置项 {} 不是数组类型: {:?}", key, config.value);
                    Ok(None)
                }
            }
        } else {
            debug!("配置项 {} 不存在", key);
            Ok(None)
        }
    }

    /// 获取原始配置项
    pub async fn get_config(&self, key: &str) -> Result<Option<ConfigItem>> {
        self.ensure_cache_initialized().await?;

        let cache = self.cache.read().await;
        Ok(cache.get(key).cloned())
    }

    /// 按分类获取配置
    pub async fn get_configs_by_category(&self, category: &str) -> Result<Vec<ConfigItem>> {
        self.ensure_cache_initialized().await?;

        let cache = self.cache.read().await;
        Ok(cache
            .values()
            .filter(|config| config.category == category)
            .cloned()
            .collect())
    }

    /// 获取用户可编辑的配置
    pub async fn get_user_editable_configs(&self) -> Result<Vec<ConfigItem>> {
        self.ensure_cache_initialized().await?;

        let cache = self.cache.read().await;
        Ok(cache
            .values()
            .filter(|config| config.is_user_editable)
            .cloned()
            .collect())
    }

    /// 更新单个配置
    pub async fn update_config(&self, key: &str, value: Value) -> Result<()> {
        self.ensure_cache_initialized().await?;

        // 检查权限
        let is_editable = {
            let cache = self.cache.read().await;
            if let Some(config) = cache.get(key) {
                if !config.is_user_editable {
                    return Err(crate::DuckError::Custom(format!("配置项 {} 不可编辑", key)).into());
                }
                config.is_user_editable
            } else {
                return Err(crate::DuckError::Custom(format!("配置项 {} 不存在", key)).into());
            }
        };

        if !is_editable {
            return Err(crate::DuckError::Custom(format!("配置项 {} 不可编辑", key)).into());
        }

        // 验证类型
        let expected_type = {
            let cache = self.cache.read().await;
            cache.get(key).map(|config| config.config_type.clone())
        };

        if let Some(expected_type) = expected_type {
            if !self.validate_value_type(&value, &expected_type) {
                return Err(crate::DuckError::Custom(format!(
                    "配置项 {} 的值类型不匹配，期望 {:?}，实际 {:?}",
                    key, expected_type, value
                ))
                .into());
            }
        }

        // 更新数据库
        let value_json = serde_json::to_string(&value)?;
        self.db.write_with_retry(|conn| {
            conn.execute(
                "UPDATE app_config SET config_value = ?, updated_at = CURRENT_TIMESTAMP WHERE config_key = ?",
                [&value_json, key]
            )?;
            Ok(())
        }).await?;

        // 更新缓存
        let mut cache = self.cache.write().await;
        if let Some(config) = cache.get_mut(key) {
            config.value = value;
        }

        debug!("配置项 {} 更新成功", key);
        Ok(())
    }

    /// 批量更新配置
    pub async fn update_configs(&self, updates: Vec<ConfigUpdateRequest>) -> Result<()> {
        self.ensure_cache_initialized().await?;

        // 验证所有更新请求
        for update in &updates {
            // 检查权限
            let cache = self.cache.read().await;
            if let Some(config) = cache.get(&update.key) {
                if !config.is_user_editable {
                    return Err(crate::DuckError::Custom(format!(
                        "配置项 {} 不可编辑",
                        update.key
                    ))
                    .into());
                }

                // 验证类型
                if update.validate && !self.validate_value_type(&update.value, &config.config_type)
                {
                    return Err(crate::DuckError::Custom(format!(
                        "配置项 {} 的值类型不匹配",
                        update.key
                    ))
                    .into());
                }
            } else {
                return Err(
                    crate::DuckError::Custom(format!("配置项 {} 不存在", update.key)).into(),
                );
            }
        }

        // 批量更新数据库
        self.db.batch_write_with_retry(|conn| {
            for update in &updates {
                let value_json = serde_json::to_string(&update.value)
                    .map_err(|e| duckdb::Error::InvalidParameterName(format!("JSON序列化失败: {}", e)))?;
                
                conn.execute(
                    "UPDATE app_config SET config_value = ?, updated_at = CURRENT_TIMESTAMP WHERE config_key = ?",
                    [&value_json, &update.key]
                )?;
            }
            Ok(())
        }).await?;

        // 批量更新缓存
        let mut cache = self.cache.write().await;
        for update in updates {
            if let Some(config) = cache.get_mut(&update.key) {
                config.value = update.value;
            }
        }

        debug!("批量配置更新成功");
        Ok(())
    }

    /// 重置配置为默认值
    pub async fn reset_config_to_default(&self, key: &str) -> Result<()> {
        self.ensure_cache_initialized().await?;

        let default_value = {
            let cache = self.cache.read().await;
            if let Some(config) = cache.get(key) {
                if !config.is_user_editable {
                    return Err(crate::DuckError::Custom(format!("配置项 {} 不可编辑", key)).into());
                }
                config.default_value.clone()
            } else {
                return Err(crate::DuckError::Custom(format!("配置项 {} 不存在", key)).into());
            }
        };

        if let Some(default_value) = default_value {
            self.update_config(key, default_value).await
        } else {
            Err(crate::DuckError::Custom(format!("配置项 {} 没有默认值", key)).into())
        }
    }

    /// 刷新缓存（重新从数据库加载）
    pub async fn refresh_cache(&self) -> Result<()> {
        *self.cache_initialized.write().await = false;
        self.initialize_cache().await
    }

    /// 获取配置统计信息
    pub async fn get_config_stats(&self) -> Result<ConfigStats> {
        self.ensure_cache_initialized().await?;

        let cache = self.cache.read().await;
        let total_count = cache.len();
        let editable_count = cache.values().filter(|c| c.is_user_editable).count();
        let system_count = cache.values().filter(|c| c.is_system_config).count();

        // 按分类统计
        let mut category_stats = HashMap::new();
        for config in cache.values() {
            *category_stats.entry(config.category.clone()).or_insert(0) += 1;
        }

        Ok(ConfigStats {
            total_count,
            editable_count,
            system_count,
            category_stats,
        })
    }

    // ==================== 业务特定方法 ====================

    /// 更新最后备份时间
    pub async fn update_last_backup_time(
        &self,
        backup_time: chrono::DateTime<chrono::Utc>,
        success: bool,
    ) -> Result<()> {
        let time_value = Value::String(backup_time.to_rfc3339());
        self.update_config("auto_backup_last_time", time_value)
            .await?;

        if success {
            let status_value = Value::String("success".to_string());
            self.update_config("auto_backup_last_status", status_value)
                .await?;
        } else {
            let status_value = Value::String("failed".to_string());
            self.update_config("auto_backup_last_status", status_value)
                .await?;
        }

        Ok(())
    }

    /// 设置自动备份cron表达式
    pub async fn set_auto_backup_cron(&self, cron_expr: &str) -> Result<()> {
        let value = Value::String(cron_expr.to_string());
        self.update_config("auto_backup_schedule", value).await
    }

    /// 设置自动备份开关
    pub async fn set_auto_backup_enabled(&self, enabled: bool) -> Result<()> {
        let value = Value::Bool(enabled);
        self.update_config("auto_backup_enabled", value).await
    }

    /// 获取自动备份配置
    pub async fn get_auto_backup_config(&self) -> Result<AutoBackupConfig> {
        let enabled = self.get_bool("auto_backup_enabled").await?.unwrap_or(false);
        let cron_expr = self
            .get_string("auto_backup_schedule")
            .await?
            .unwrap_or("0 2 * * *".to_string());
        let retention_days = self
            .get_integer("auto_backup_retention_days")
            .await?
            .unwrap_or(7) as i32;
        let backup_dir = self
            .get_string("auto_backup_directory")
            .await?
            .unwrap_or("./backups".to_string());

        let last_backup_time =
            if let Some(time_str) = self.get_string("auto_backup_last_time").await? {
                chrono::DateTime::parse_from_rfc3339(&time_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .ok()
            } else {
                None
            };

        Ok(AutoBackupConfig {
            enabled,
            cron_expression: cron_expr,
            last_backup_time,
            backup_retention_days: retention_days,
            backup_directory: backup_dir,
        })
    }

    /// 创建自动升级任务
    pub async fn create_auto_upgrade_task(&self, task: &AutoUpgradeTask) -> Result<()> {
        let _task_json = serde_json::to_value(task)?;

        // 将任务存储在数据库中（使用任务表或配置表）
        self.db.write_with_retry(|conn| {
            conn.execute(
                r#"INSERT OR REPLACE INTO auto_upgrade_tasks 
                   (task_id, task_name, schedule_time, upgrade_type, target_version, status, progress, error_message, created_at, updated_at)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
                [
                    &task.task_id,
                    &task.task_name,
                    &task.schedule_time.to_rfc3339(),
                    &task.upgrade_type,
task.target_version.as_deref().unwrap_or(""),
                    &task.status,
                    &task.progress.map(|p| p.to_string()).unwrap_or_default(),
                    &task.error_message.as_deref().unwrap_or(""),
                    &task.created_at.to_rfc3339(),
                    &task.updated_at.to_rfc3339(),
                ]
            )?;
            Ok(())
        }).await?;

        debug!("自动升级任务 {} 创建成功", task.task_id);
        Ok(())
    }

    /// 更新升级任务状态
    pub async fn update_upgrade_task_status(
        &self,
        task_id: &str,
        status: &str,
        progress: Option<i32>,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.db
            .write_with_retry(|conn| {
                conn.execute(
                    r#"UPDATE auto_upgrade_tasks 
                   SET status = ?1, progress = ?2, error_message = ?3, updated_at = ?4
                   WHERE task_id = ?5"#,
                    [
                        status,
                        &progress.map(|p| p.to_string()).unwrap_or_default(),
                        error_message.unwrap_or(""),
                        &chrono::Utc::now().to_rfc3339(),
                        task_id,
                    ],
                )?;
                Ok(())
            })
            .await?;

        debug!("升级任务 {} 状态更新为: {}", task_id, status);
        Ok(())
    }

    /// 获取待处理的升级任务
    pub async fn get_pending_upgrade_tasks(&self) -> Result<Vec<AutoUpgradeTask>> {
        self.db
            .read_with_retry(|conn| {
                let mut stmt = conn.prepare(
                    r#"SELECT task_id, task_name, schedule_time, upgrade_type, target_version, 
                          status, progress, error_message, created_at, updated_at
                   FROM auto_upgrade_tasks 
                   WHERE status IN ('pending', 'in_progress')
                   ORDER BY schedule_time ASC"#,
                )?;

                let tasks = stmt.query_map([], |row| {
                    let schedule_time_str: String = row.get("schedule_time")?;
                    let created_at_str: String = row.get("created_at")?;
                    let updated_at_str: String = row.get("updated_at")?;
                    let progress_str: String = row.get("progress")?;
                    let target_version: String = row.get("target_version")?;
                    let error_msg: String = row.get("error_message")?;

                    Ok(AutoUpgradeTask {
                        task_id: row.get("task_id")?,
                        task_name: row.get("task_name")?,
                        schedule_time: chrono::DateTime::parse_from_rfc3339(&schedule_time_str)
                            .map_err(|_| {
                                duckdb::Error::InvalidColumnType(
                                    0,
                                    "schedule_time".to_string(),
                                    duckdb::types::Type::Text,
                                )
                            })?
                            .with_timezone(&chrono::Utc),
                        upgrade_type: row.get("upgrade_type")?,
                        target_version: if target_version.is_empty() {
                            None
                        } else {
                            Some(target_version)
                        },
                        status: row.get("status")?,
                        progress: if progress_str.is_empty() {
                            None
                        } else {
                            progress_str.parse().ok()
                        },
                        error_message: if error_msg.is_empty() {
                            None
                        } else {
                            Some(error_msg)
                        },
                        created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                            .map_err(|_| {
                                duckdb::Error::InvalidColumnType(
                                    0,
                                    "created_at".to_string(),
                                    duckdb::types::Type::Text,
                                )
                            })?
                            .with_timezone(&chrono::Utc),
                        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                            .map_err(|_| {
                                duckdb::Error::InvalidColumnType(
                                    0,
                                    "updated_at".to_string(),
                                    duckdb::types::Type::Text,
                                )
                            })?
                            .with_timezone(&chrono::Utc),
                    })
                })?;

                let mut result = Vec::new();
                for task in tasks {
                    result.push(task?);
                }
                Ok(result)
            })
            .await
    }

    /// 验证值类型
    fn validate_value_type(&self, value: &Value, expected_type: &ConfigType) -> bool {
        match (value, expected_type) {
            (Value::String(_), ConfigType::String) => true,
            (Value::Number(_), ConfigType::Number) => true,
            (Value::Bool(_), ConfigType::Boolean) => true,
            (Value::Object(_), ConfigType::Object) => true,
            (Value::Array(_), ConfigType::Array) => true,
            _ => false,
        }
    }
}

/// 配置统计信息
#[derive(Debug, Clone)]
pub struct ConfigStats {
    pub total_count: usize,
    pub editable_count: usize,
    pub system_count: usize,
    pub category_stats: HashMap<String, usize>,
}

// ==================== 业务特定结构体 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoUpgradeTask {
    pub task_id: String,
    pub task_name: String,
    pub schedule_time: chrono::DateTime<chrono::Utc>,
    pub upgrade_type: String,
    pub target_version: Option<String>,
    pub status: String,
    pub progress: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoBackupConfig {
    pub enabled: bool,
    pub cron_expression: String,
    pub last_backup_time: Option<chrono::DateTime<chrono::Utc>>,
    pub backup_retention_days: i32,
    pub backup_directory: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DatabaseManager;
    use serde_json::json;

    async fn create_test_config_manager() -> ConfigManager {
        let db = DatabaseManager::new_memory().await.unwrap();
        ConfigManager::new(db)
    }

    #[tokio::test]
    async fn test_config_manager_creation() {
        let manager = create_test_config_manager().await;
        let stats = manager.get_config_stats().await.unwrap();

        // 应该有预配置的配置项
        assert!(stats.total_count > 0);
        assert!(stats.editable_count > 0);
    }

    #[tokio::test]
    async fn test_get_string_config() {
        let manager = create_test_config_manager().await;

        // 获取应用版本配置（应该存在）
        let version = manager.get_string("app.version").await.unwrap();
        assert!(version.is_some());
        assert_eq!(version.unwrap(), "0.1.0");
    }

    #[tokio::test]
    async fn test_get_number_config() {
        let manager = create_test_config_manager().await;

        // 获取窗口宽度配置
        let width = manager.get_integer("ui.window_width").await.unwrap();
        assert!(width.is_some());
        assert_eq!(width.unwrap(), 1200);
    }

    #[tokio::test]
    async fn test_get_bool_config() {
        let manager = create_test_config_manager().await;

        // 获取首次运行配置
        let first_run = manager.get_bool("app.first_run").await.unwrap();
        assert!(first_run.is_some());
        assert_eq!(first_run.unwrap(), true);
    }

    #[tokio::test]
    async fn test_update_config() {
        let manager = create_test_config_manager().await;

        // 更新主题配置
        let result = manager
            .update_config("ui.theme", serde_json::json!("dark"))
            .await;
        assert!(result.is_ok());

        // 验证更新
        let theme = manager.get_string("ui.theme").await.unwrap();
        assert_eq!(theme.unwrap(), "dark");
    }

    #[tokio::test]
    async fn test_batch_update_configs() {
        let manager = create_test_config_manager().await;

        let updates = vec![
            ConfigUpdateRequest {
                key: "ui.theme".to_string(),
                value: serde_json::json!("dark"),
                validate: true,
            },
            ConfigUpdateRequest {
                key: "ui.window_width".to_string(),
                value: serde_json::json!(1600),
                validate: true,
            },
        ];

        let result = manager.update_configs(updates).await;
        assert!(result.is_ok());

        // 验证更新
        let theme = manager.get_string("ui.theme").await.unwrap();
        let width = manager.get_integer("ui.window_width").await.unwrap();
        assert_eq!(theme.unwrap(), "dark");
        assert_eq!(width.unwrap(), 1600);
    }

    #[tokio::test]
    async fn test_get_configs_by_category() {
        let manager = create_test_config_manager().await;

        let ui_configs = manager.get_configs_by_category("ui").await.unwrap();
        assert!(!ui_configs.is_empty());

        // 所有配置都应该是ui分类
        for config in ui_configs {
            assert_eq!(config.category, "ui");
        }
    }

    #[tokio::test]
    async fn test_permission_check() {
        let manager = create_test_config_manager().await;

        // 尝试更新系统配置（应该失败）
        let result = manager
            .update_config("app.version", serde_json::json!("2.0.0"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_type_validation() {
        let manager = create_test_config_manager().await;

        // 尝试用错误类型更新配置（应该失败）
        let result = manager
            .update_config("ui.theme", serde_json::json!(123))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reset_to_default() {
        let manager = create_test_config_manager().await;

        // 先更新配置
        manager
            .update_config("ui.theme", serde_json::json!("dark"))
            .await
            .unwrap();

        // 重置为默认值
        let result = manager.reset_config_to_default("ui.theme").await;
        assert!(result.is_ok());

        // 验证重置
        let theme = manager.get_string("ui.theme").await.unwrap();
        assert_eq!(theme.unwrap(), "auto"); // 默认值
    }
}
