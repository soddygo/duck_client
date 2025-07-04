use client_core::{
    api::ApiClient, authenticated_client::AuthenticatedClient, backup::BackupManager,
    config::AppConfig, container::DockerManager, database::Database, error::Result,
    upgrade::UpgradeManager,
};
use std::path::PathBuf;

use crate::cli::Commands;
use crate::commands;

#[derive(Clone)]
pub struct CliApp {
    pub config: AppConfig,
    pub database: Database,
    pub api_client: ApiClient,
    pub authenticated_client: AuthenticatedClient,
    pub docker_manager: DockerManager,
    pub backup_manager: BackupManager,
    pub upgrade_manager: UpgradeManager,
}

impl CliApp {
    /// 使用智能配置查找初始化CLI应用
    pub async fn new_with_auto_config() -> Result<Self> {
        let config = AppConfig::find_and_load_config()?;

        // 确保缓存目录存在
        config.ensure_cache_dirs()?;

        // 初始化数据库
        let database = Database::connect("history.db").await?;

        // 创建认证客户端（自动处理注册和认证）
        let server_base_url = client_core::constants::api::DEFAULT_BASE_URL.to_string();
        let authenticated_client =
            AuthenticatedClient::new(database.clone(), server_base_url).await?;

        // 获取用于API请求的客户端ID（只使用服务端返回的client_id）
        let client_id = database.get_api_client_id().await?;
        let mut api_client = ApiClient::new(client_id.clone());

        // 将AuthenticatedClient设置到ApiClient中，这样ApiClient可以使用自动认证功能
        api_client.set_authenticated_client(authenticated_client.clone());

        // 创建其他管理器
        let docker_manager = DockerManager::new(PathBuf::from(&config.docker.compose_file))?;
        let backup_manager = BackupManager::new(
            PathBuf::from(&config.backup.storage_dir),
            database.clone(),
            docker_manager.clone(),
        )?;
        let upgrade_manager = UpgradeManager::new(
            config.clone(),
            PathBuf::from("config.toml"), // 使用默认配置路径
            docker_manager.clone(),
            backup_manager.clone(),
            api_client.clone(),
            database.clone(),
        );

        Ok(Self {
            config,
            database,
            api_client,
            authenticated_client,
            docker_manager,
            backup_manager,
            upgrade_manager,
        })
    }

    /// 运行应用命令
    pub async fn run_command(&mut self, command: Commands) -> Result<()> {
        match command {
            Commands::Status => commands::run_status(self).await,
            Commands::ApiInfo => commands::run_api_info(self).await,
            Commands::Init { .. } => unreachable!(), // 已经在 main.rs 中处理
            Commands::CheckUpdate(check_update_cmd) => {
                commands::handle_check_update_command(check_update_cmd)
                    .await
                    .map_err(|e| {
                        client_core::error::DuckError::custom(format!("检查更新失败: {e}"))
                    })
            }
            Commands::Upgrade { full, force, check } => commands::run_upgrade(self, full, force, check).await,
            Commands::Backup => commands::run_backup(self).await,
            Commands::ListBackups => commands::run_list_backups(self).await,
            Commands::Rollback { backup_id, force } => {
                commands::run_rollback(self, backup_id, force).await
            }
            Commands::DockerService(docker_cmd) => {
                commands::run_docker_service_command(self, docker_cmd).await
            }
            Commands::Ducker { args } => commands::run_ducker(args).await,
            Commands::AutoBackup(auto_backup_cmd) => {
                commands::handle_auto_backup_command(self, auto_backup_cmd).await
            }
            Commands::AutoUpgradeDeploy(auto_upgrade_deploy_cmd) => {
                commands::handle_auto_upgrade_deploy_command(self, auto_upgrade_deploy_cmd).await
            }
            Commands::Cache(cache_cmd) => {
                commands::handle_cache_command(self, cache_cmd).await
            }
        }
    }




}
