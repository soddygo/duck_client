use crate::Result;
use chrono::{DateTime, Utc};
use tokio::sync::oneshot;

use super::models::{BackupRecord, ScheduledTask};

/// DuckDB数据库操作消息
#[derive(Debug)]
pub enum DbMessage {
    /// 初始化数据库表
    InitTables {
        respond_to: oneshot::Sender<Result<()>>,
    },
    /// 获取配置值
    GetConfig {
        key: String,
        respond_to: oneshot::Sender<Result<Option<String>>>,
    },
    /// 设置配置值
    SetConfig {
        key: String,
        value: String,
        respond_to: oneshot::Sender<Result<()>>,
    },
    /// 创建备份记录
    CreateBackupRecord {
        file_path: String,
        service_version: String,
        backup_type: String,
        status: String,
        respond_to: oneshot::Sender<Result<i64>>,
    },
    /// 获取所有备份记录
    GetAllBackups {
        respond_to: oneshot::Sender<Result<Vec<BackupRecord>>>,
    },
    /// 根据ID获取备份记录
    GetBackupById {
        id: i64,
        respond_to: oneshot::Sender<Result<Option<BackupRecord>>>,
    },
    /// 删除备份记录
    DeleteBackupRecord {
        backup_id: i64,
        respond_to: oneshot::Sender<Result<()>>,
    },
    /// 更新备份文件路径
    UpdateBackupFilePath {
        backup_id: i64,
        new_path: String,
        respond_to: oneshot::Sender<Result<()>>,
    },
    /// 创建计划任务
    CreateScheduledTask {
        task_type: String,
        target_version: String,
        scheduled_at: DateTime<Utc>,
        respond_to: oneshot::Sender<Result<i64>>,
    },
    /// 获取待执行任务
    GetPendingTasks {
        respond_to: oneshot::Sender<Result<Vec<ScheduledTask>>>,
    },
    /// 更新任务状态
    UpdateTaskStatus {
        task_id: i64,
        status: String,
        details: Option<String>,
        respond_to: oneshot::Sender<Result<()>>,
    },
    /// 取消待执行任务
    CancelPendingTasks {
        task_type: String,
        respond_to: oneshot::Sender<Result<()>>,
    },
}
