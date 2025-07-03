use crate::Result;
use chrono::{DateTime, Utc};
use duckdb::{Connection, params};
use serde_json;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{debug, info};

use super::messages::DbMessage;
use super::models::{BackupRecord, ScheduledTask};

/// DuckDB Actor - 确保单线程访问DuckDB
pub struct DuckDbActor {
    connection: Connection,
}

impl DuckDbActor {
    /// 创建新的DuckDB Actor
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let connection = Connection::open(db_path)?;
        Ok(Self { connection })
    }

    /// 创建内存DuckDB Actor
    pub fn new_memory() -> Result<Self> {
        let connection = Connection::open_in_memory()?;
        Ok(Self { connection })
    }

    /// 运行Actor消息循环
    pub async fn run(mut self, mut receiver: mpsc::Receiver<DbMessage>) {
        info!("DuckDB Actor 已启动");

        while let Some(message) = receiver.recv().await {
            self.handle_message(message).await;
        }

        info!("DuckDB Actor 已关闭");
    }

    /// 处理数据库消息
    async fn handle_message(&mut self, message: DbMessage) {
        match message {
            DbMessage::InitTables { respond_to } => {
                let result = self.init_tables();
                let _ = respond_to.send(result);
            }
            DbMessage::GetConfig { key, respond_to } => {
                let result = self.get_config(&key);
                let _ = respond_to.send(result);
            }
            DbMessage::SetConfig {
                key,
                value,
                respond_to,
            } => {
                let result = self.set_config(&key, &value);
                let _ = respond_to.send(result);
            }
            DbMessage::CreateBackupRecord {
                file_path,
                service_version,
                backup_type,
                status,
                respond_to,
            } => {
                let result =
                    self.create_backup_record(&file_path, &service_version, &backup_type, &status);
                let _ = respond_to.send(result);
            }
            DbMessage::GetAllBackups { respond_to } => {
                let result = self.get_all_backups();
                let _ = respond_to.send(result);
            }
            DbMessage::GetBackupById { id, respond_to } => {
                let result = self.get_backup_by_id(id);
                let _ = respond_to.send(result);
            }
            DbMessage::DeleteBackupRecord {
                backup_id,
                respond_to,
            } => {
                let result = self.delete_backup_record(backup_id);
                let _ = respond_to.send(result);
            }
            DbMessage::UpdateBackupFilePath {
                backup_id,
                new_path,
                respond_to,
            } => {
                let result = self.update_backup_file_path(backup_id, &new_path);
                let _ = respond_to.send(result);
            }
            DbMessage::CreateScheduledTask {
                task_type,
                target_version,
                scheduled_at,
                respond_to,
            } => {
                let result = self.create_scheduled_task(
                    &task_type,
                    &target_version,
                    scheduled_at,
                    "PENDING",
                );
                let _ = respond_to.send(result);
            }
            DbMessage::GetPendingTasks { respond_to } => {
                let result = self.get_pending_tasks();
                let _ = respond_to.send(result);
            }
            DbMessage::UpdateTaskStatus {
                task_id,
                status,
                details,
                respond_to,
            } => {
                let result = self.update_task_status(task_id, &status, details.as_deref());
                let _ = respond_to.send(result);
            }
            DbMessage::CancelPendingTasks {
                task_type,
                respond_to,
            } => {
                let result = self.cancel_pending_tasks(&task_type);
                let _ = respond_to.send(result);
            }
        }
    }

    /// 初始化数据库表
    fn init_tables(&mut self) -> Result<()> {
        debug!("正在初始化DuckDB表...");

        // 读取并执行SQL初始化脚本
        let sql_content = include_str!("../../migrations/init_duckdb.sql");

        // 按分号分割SQL语句并执行
        for statement in sql_content.split(';').filter(|s| !s.trim().is_empty()) {
            let trimmed = statement.trim();
            if !trimmed.is_empty() {
                self.connection.execute(trimmed, [])?;
            }
        }

        info!("DuckDB表初始化完成");
        Ok(())
    }

    /// 获取配置值
    fn get_config(&mut self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .connection
            .prepare("SELECT config_value FROM app_config WHERE config_key = ?")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            let json_value: String = row.get(0)?;
            // 尝试解析JSON，如果是字符串则去掉引号
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_value) {
                match parsed {
                    serde_json::Value::String(s) => Ok(Some(s)),
                    _ => Ok(Some(json_value)), // 非字符串类型直接返回JSON
                }
            } else {
                Ok(Some(json_value)) // JSON解析失败，返回原始值
            }
        } else {
            Ok(None)
        }
    }

    /// 设置配置值
    fn set_config(&mut self, key: &str, value: &str) -> Result<()> {
        // 首先尝试更新现有配置
        let updated = self.connection.execute(
            "UPDATE app_config SET config_value = ?, updated_at = CURRENT_TIMESTAMP WHERE config_key = ?",
            params![format!("\"{}\"", value), key],
        )?;
        
        // 如果没有更新任何行，则插入新配置
        if updated == 0 {
            self.connection.execute(
                "INSERT INTO app_config (config_key, config_value, config_type, category, is_system_config, is_user_editable) VALUES (?, ?, 'STRING', 'system', TRUE, TRUE)",
                params![key, format!("\"{}\"", value)],
            )?;
        }
        Ok(())
    }

    /// 创建备份记录
    fn create_backup_record(
        &mut self,
        file_path: &str,
        service_version: &str,
        backup_type: &str,
        status: &str,
    ) -> Result<i64> {
        // 插入记录，让数据库自动生成ID
        self.connection.execute(
            "INSERT INTO backups (file_path, service_version, backup_type, status) 
             VALUES (?, ?, ?, ?)",
            params![file_path, service_version, backup_type, status],
        )?;

        // 获取最后插入的ID
        let id: i64 = self
            .connection
            .query_row("SELECT currval('backup_id_seq')", [], |row| row.get(0))?;

        Ok(id)
    }

    /// 获取所有备份记录
    fn get_all_backups(&mut self) -> Result<Vec<BackupRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, file_path, service_version, backup_type, status, created_at 
             FROM backups ORDER BY created_at DESC",
        )?;

        let backup_iter = stmt.query_map([], |row| {
            Ok(BackupRecord {
                id: row.get(0)?,
                file_path: row.get(1)?,
                service_version: row.get(2)?,
                backup_type: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut backups = Vec::new();
        for backup in backup_iter {
            backups.push(backup?);
        }

        Ok(backups)
    }

    /// 根据ID获取备份记录
    fn get_backup_by_id(&mut self, id: i64) -> Result<Option<BackupRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, file_path, service_version, backup_type, status, created_at 
             FROM backups WHERE id = ?",
        )?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(BackupRecord {
                id: row.get(0)?,
                file_path: row.get(1)?,
                service_version: row.get(2)?,
                backup_type: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 删除备份记录
    fn delete_backup_record(&mut self, backup_id: i64) -> Result<()> {
        self.connection
            .execute("DELETE FROM backups WHERE id = ?", params![backup_id])?;
        Ok(())
    }

    /// 更新备份文件路径
    fn update_backup_file_path(&mut self, backup_id: i64, new_path: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE backups SET file_path = ? WHERE id = ?",
            params![new_path, backup_id],
        )?;
        Ok(())
    }

    /// 创建计划任务
    fn create_scheduled_task(
        &mut self,
        task_type: &str,
        target_version: &str,
        scheduled_at: DateTime<Utc>,
        status: &str,
    ) -> Result<i64> {
        // 插入记录，让数据库自动生成ID
        self.connection.execute(
            "INSERT INTO scheduled_tasks (task_type, target_version, scheduled_at, status) 
             VALUES (?, ?, ?, ?)",
            params![task_type, target_version, scheduled_at, status],
        )?;

        // 获取最后插入的ID
        let id: i64 = self
            .connection
            .query_row("SELECT currval('task_id_seq')", [], |row| row.get(0))?;

        Ok(id)
    }

    /// 获取待执行任务
    fn get_pending_tasks(&mut self) -> Result<Vec<ScheduledTask>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, task_type, target_version, scheduled_at, status, details, created_at, completed_at 
             FROM scheduled_tasks WHERE status = 'PENDING' ORDER BY scheduled_at"
        )?;

        let task_iter = stmt.query_map([], |row| {
            Ok(ScheduledTask {
                id: row.get(0)?,
                task_type: row.get(1)?,
                target_version: row.get(2)?,
                scheduled_at: row.get(3)?,
                status: row.get(4)?,
                details: row.get(5)?,
                created_at: row.get(6)?,
                completed_at: row.get(7)?,
            })
        })?;

        let mut tasks = Vec::new();
        for task in task_iter {
            tasks.push(task?);
        }

        Ok(tasks)
    }

    /// 更新任务状态
    fn update_task_status(
        &mut self,
        task_id: i64,
        status: &str,
        details: Option<&str>,
    ) -> Result<()> {
        if let Some(details) = details {
            self.connection.execute(
                "UPDATE scheduled_tasks SET status = ?, details = ?, completed_at = NOW() WHERE id = ?",
                params![status, details, task_id],
            )?;
        } else {
            self.connection.execute(
                "UPDATE scheduled_tasks SET status = ?, completed_at = NOW() WHERE id = ?",
                params![status, task_id],
            )?;
        }
        Ok(())
    }

    /// 取消待执行任务
    fn cancel_pending_tasks(&mut self, task_type: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE scheduled_tasks SET status = 'CANCELLED', completed_at = NOW() 
             WHERE task_type = ? AND status = 'PENDING'",
            params![task_type],
        )?;
        Ok(())
    }
}
