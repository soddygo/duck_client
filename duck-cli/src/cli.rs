use crate::project_info::{metadata, version_info};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// 自动备份相关命令
#[derive(Subcommand, Debug)]
pub enum AutoBackupCommand {
    /// 立即执行一次自动备份
    Run,
    /// 设置定时备份的 cron 表达式
    Cron {
        /// cron 表达式，例如 "0 2 * * *" 表示每天凌晨2点
        #[arg(help = "cron 表达式，例如 '0 2 * * *' 表示每天凌晨2点")]
        expression: Option<String>,
    },
    /// 启用或禁用自动备份
    Enabled {
        /// 是否启用自动备份
        #[arg(help = "true 启用，false 禁用")]
        enabled: Option<bool>,
    },
    /// 显示当前自动备份配置
    Status,
}

/// 自动升级部署相关命令
#[derive(Subcommand, Debug)]
pub enum AutoUpgradeDeployCommand {
    /// 立即执行自动升级部署
    Run {
        /// 指定frontend服务的端口号（默认80端口）
        #[arg(
            long,
            help = "指定frontend服务的端口号，对应docker-compose.yml中的FRONTEND_HOST_PORT变量（默认: 80端口）"
        )]
        port: Option<u16>,
    },
    /// 延迟执行自动升级部署
    DelayTimeDeploy {
        /// 延迟时间数值
        #[arg(help = "延迟时间数值，例如 2")]
        time: u32,
        /// 时间单位 (hours, minutes, days)
        #[arg(
            long,
            default_value = "hours",
            help = "时间单位：hours(小时), minutes(分钟), days(天)"
        )]
        unit: String,
    },
    /// 显示当前自动升级配置
    Status,
}

/// 客户端更新相关命令
#[derive(Subcommand, Debug)]
pub enum CheckUpdateCommand {
    /// 检查最新版本信息
    Check,
    /// 安装指定版本或最新版本
    Install {
        /// 指定版本号（如不指定则安装最新版本）
        #[arg(long)]
        version: Option<String>,
        /// 强制重新安装（即使当前已是最新版本）
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum DockerServiceCommand {
    /// 启动Docker服务
    Start,
    /// 停止Docker服务
    Stop,
    /// 重启Docker服务
    Restart,
    /// 检查服务状态
    Status,
    /// 重启指定容器
    RestartContainer {
        /// 容器名称
        container_name: String,
    },
    /// 解压Docker服务包
    Extract {
        /// 指定docker.zip文件路径（可选，默认使用当前版本的下载文件）
        #[arg(long)]
        file: Option<String>,
        /// 目标版本（可选，默认使用当前配置版本）
        #[arg(long)]
        version: Option<String>,
    },
    /// 加载Docker镜像
    LoadImages,
    /// 设置镜像标签
    SetupTags,
    /// 显示架构信息
    ArchInfo,
    /// 列出Docker镜像（使用ducker）
    ListImages,
}

/// 缓存管理相关命令
#[derive(Subcommand, Debug)]
pub enum CacheCommand {
    /// 清理所有缓存文件
    Clear,
    /// 显示缓存使用情况
    Status,
    /// 清理下载缓存（保留最新版本）
    CleanDownloads {
        /// 保留的版本数量
        #[arg(long, default_value = "3", help = "保留的版本数量")]
        keep: u32,
    },
}

/// Duck Client CLI - Docker 服务管理和升级工具
#[derive(Parser)]
#[command(name = "duck-cli")]
#[command(about = metadata::PROJECT_DESCRIPTION)]
#[command(version = version_info::CLI_VERSION)]
#[command(long_about = metadata::display::DESCRIPTION_LONG)]
#[command(author = metadata::PROJECT_AUTHORS)]
pub struct Cli {
    /// 配置文件路径
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,

    /// 详细输出
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 显示服务状态和版本信息
    Status,
    /// 首次使用时初始化客户端，创建配置文件和数据库
    Init {
        /// 如果配置文件已存在，强制覆盖
        #[arg(long)]
        force: bool,
    },
    /// 检查客户端更新
    #[command(subcommand)]
    CheckUpdate(CheckUpdateCommand),
    /// 显示当前API配置信息
    ApiInfo,
    /// 下载Docker服务文件
    Upgrade {
        /// 全量下载（下载完整的服务包）
        #[arg(long)]
        full: bool,
        /// 强制重新下载（用于文件损坏时）
        #[arg(long)]
        force: bool,
    },
    /// 手动创建备份
    Backup,
    /// 列出所有备份
    ListBackups,
    /// 从备份恢复
    Rollback {
        /// 备份 ID
        backup_id: i64,
        /// 强制覆盖
        #[arg(long)]
        force: bool,
    },
    /// Docker服务相关命令
    #[command(subcommand)]
    DockerService(DockerServiceCommand),

    /// 🐋 一个用于管理 Docker 容器的终端应用
    Ducker {
        /// 传递给ducker的参数
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// 自动备份管理
    #[command(subcommand)]
    AutoBackup(AutoBackupCommand),

    /// 自动升级部署
    #[command(subcommand)]
    AutoUpgradeDeploy(AutoUpgradeDeployCommand),

    /// 缓存管理
    #[command(subcommand)]
    Cache(CacheCommand),
}
