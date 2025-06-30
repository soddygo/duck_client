-- DuckDB 数据库初始化脚本
-- 支持Duck Client的自动备份、自动升级部署等功能

-- ========================================
-- 通用配置表
-- ========================================
CREATE TABLE IF NOT EXISTS config (
    key VARCHAR PRIMARY KEY,
    value VARCHAR NOT NULL,
    description VARCHAR, -- 配置项描述，便于人类理解
    category VARCHAR NOT NULL DEFAULT 'general', -- 配置分类：general/backup/upgrade/docker等
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 备份相关表
-- ========================================

-- 创建备份ID序列
CREATE SEQUENCE IF NOT EXISTS backup_id_seq START 1;

-- 备份记录表
CREATE TABLE IF NOT EXISTS backups (
    id INTEGER PRIMARY KEY DEFAULT nextval('backup_id_seq'),
    file_path VARCHAR NOT NULL UNIQUE,
    file_size INTEGER, -- 备份文件大小（字节）
    service_version VARCHAR NOT NULL,
    backup_type VARCHAR NOT NULL, -- "manual" | "auto" | "pre-upgrade" | "scheduled"
    trigger_source VARCHAR, -- 触发来源："user" | "cron" | "upgrade" | "system"
    status VARCHAR NOT NULL, -- "in_progress" | "completed" | "failed"
    error_message VARCHAR, -- 失败时的错误信息
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
);

-- 自动备份配置表
CREATE TABLE IF NOT EXISTS auto_backup_config (
    id INTEGER PRIMARY KEY DEFAULT 1, -- 只有一条记录
    enabled BOOLEAN NOT NULL DEFAULT true,
    cron_expression VARCHAR NOT NULL DEFAULT '0 2 * * *', -- 默认凌晨2点
    last_backup_at TIMESTAMP,
    consecutive_failures INTEGER DEFAULT 0, -- 连续失败次数
    max_failures INTEGER DEFAULT 3, -- 最大失败次数
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 自动升级部署相关表
-- ========================================

-- 创建升级任务ID序列
CREATE SEQUENCE IF NOT EXISTS upgrade_task_id_seq START 1;

-- 自动升级部署任务表
CREATE TABLE IF NOT EXISTS auto_upgrade_tasks (
    id INTEGER PRIMARY KEY DEFAULT nextval('upgrade_task_id_seq'),
    task_type VARCHAR NOT NULL, -- "immediate" | "delayed" | "scheduled"
    target_version VARCHAR, -- 目标版本（可为空，表示最新版本）
    scheduled_at TIMESTAMP NOT NULL, -- 计划执行时间
    delay_amount INTEGER, -- 延迟数量（用于delayed类型）
    delay_unit VARCHAR, -- 延迟单位："minutes" | "hours" | "days"
    status VARCHAR NOT NULL DEFAULT 'pending', -- "pending" | "in_progress" | "completed" | "failed" | "cancelled"
    progress INTEGER DEFAULT 0, -- 执行进度百分比
    error_message VARCHAR, -- 失败时的错误信息
    backup_created BOOLEAN DEFAULT false, -- 是否已创建备份
    backup_id INTEGER, -- 关联的备份ID（应用层保障数据完整性）
    details TEXT, -- 详细信息（JSON格式）
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TIMESTAMP,
    completed_at TIMESTAMP
);

-- ========================================
-- Cron任务管理表
-- ========================================

-- 创建Cron任务ID序列
CREATE SEQUENCE IF NOT EXISTS cron_job_id_seq START 1;

-- Cron任务配置表
CREATE TABLE IF NOT EXISTS cron_jobs (
    id INTEGER PRIMARY KEY DEFAULT nextval('cron_job_id_seq'),
    name VARCHAR NOT NULL UNIQUE, -- 任务名称，如："auto_backup"
    cron_expression VARCHAR NOT NULL, -- cron表达式
    command VARCHAR NOT NULL, -- 执行的命令
    description VARCHAR, -- 任务描述
    enabled BOOLEAN NOT NULL DEFAULT true,
    last_run_at TIMESTAMP, -- 上次执行时间
    last_run_status VARCHAR, -- "success" | "failed" | "timeout"
    last_run_output TEXT, -- 上次执行输出
    next_run_at TIMESTAMP, -- 下次执行时间（计算得出）
    run_count INTEGER DEFAULT 0, -- 执行次数
    success_count INTEGER DEFAULT 0, -- 成功次数
    failure_count INTEGER DEFAULT 0, -- 失败次数
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 任务执行日志表
-- ========================================

-- 创建日志ID序列
CREATE SEQUENCE IF NOT EXISTS task_log_id_seq START 1;

-- 任务执行日志表
CREATE TABLE IF NOT EXISTS task_logs (
    id INTEGER PRIMARY KEY DEFAULT nextval('task_log_id_seq'),
    task_type VARCHAR NOT NULL, -- "backup" | "upgrade" | "cron"
    task_id INTEGER, -- 关联的任务ID
    log_level VARCHAR NOT NULL, -- "info" | "warn" | "error" | "debug"
    message TEXT NOT NULL,
    details TEXT, -- 详细信息（JSON格式）
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 创建索引
-- ========================================

-- 配置表索引
CREATE INDEX IF NOT EXISTS idx_config_category ON config(category);

-- 备份表索引
CREATE INDEX IF NOT EXISTS idx_backups_type ON backups(backup_type);
CREATE INDEX IF NOT EXISTS idx_backups_status ON backups(status);
CREATE INDEX IF NOT EXISTS idx_backups_created_at ON backups(created_at);

-- 升级任务表索引
CREATE INDEX IF NOT EXISTS idx_upgrade_tasks_status ON auto_upgrade_tasks(status);
CREATE INDEX IF NOT EXISTS idx_upgrade_tasks_scheduled_at ON auto_upgrade_tasks(scheduled_at);

-- Cron任务表索引
CREATE INDEX IF NOT EXISTS idx_cron_jobs_enabled ON cron_jobs(enabled);
CREATE INDEX IF NOT EXISTS idx_cron_jobs_next_run ON cron_jobs(next_run_at);

-- 任务日志表索引
CREATE INDEX IF NOT EXISTS idx_task_logs_type ON task_logs(task_type);
CREATE INDEX IF NOT EXISTS idx_task_logs_created_at ON task_logs(created_at);

-- ========================================
-- 插入默认配置数据
-- ========================================

-- 插入默认自动备份配置
INSERT OR IGNORE INTO auto_backup_config (id, enabled, cron_expression) 
VALUES (1, true, '0 2 * * *');

-- 插入默认系统配置
INSERT OR IGNORE INTO config (key, value, description, category) VALUES
('app.version', '0.1.0', 'Duck Client应用版本', 'system'),
('docker.compose_file_path', './docker/docker-compose.yml', 'Docker Compose文件路径', 'docker'),
('backup.max_keep_days', '30', '备份文件保留天数', 'backup'),
('backup.auto_cleanup', 'true', '是否自动清理过期备份', 'backup'),
('upgrade.auto_backup_enabled', 'true', '升级前是否自动备份', 'upgrade'),
('upgrade.rollback_enabled', 'true', '是否启用升级回滚功能', 'upgrade'),
('system.timezone', 'Asia/Shanghai', '系统时区', 'system'),
('logging.level', 'info', '日志级别', 'system');

-- 插入默认Cron任务（自动备份）
INSERT OR IGNORE INTO cron_jobs (name, cron_expression, command, description, enabled) VALUES
('auto_backup', '0 2 * * *', 'duck-cli auto-backup run', '每天凌晨2点自动备份', true); 