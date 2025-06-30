use clap::Parser;
use client_core::DuckError;
use duck_cli::{Cli, CliApp, Commands, run_init, setup_logging};
use std::error::Error;
use tracing::error;

#[tokio::main]
async fn main() {
    // 解析命令行参数
    let cli = Cli::parse();

    // 设置日志记录
    setup_logging(cli.verbose);

    // `init` 命令是特例，它不需要预先加载配置
    if let Commands::Init { force } = cli.command {
        if let Err(e) = run_init(force).await {
            error!("❌ 初始化失败: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // 对于其他所有命令，我们需要加载配置并初始化App
    let mut app = match CliApp::new_with_auto_config().await {
        Ok(app) => app,
        Err(e) => {
            // 检查错误的根本原因是否是ConfigNotFound
            let mut source = e.source();
            let mut is_config_not_found = false;
            while let Some(err) = source {
                if err.downcast_ref::<DuckError>().is_some() {
                    if let Some(DuckError::ConfigNotFound) = err.downcast_ref::<DuckError>() {
                        is_config_not_found = true;
                        break;
                    }
                }
                source = err.source();
            }

            if is_config_not_found {
                error!("❌ 配置文件 '{}' 未找到。", cli.config.display());
                error!("👉 请先运行 'duck-cli init' 命令来创建配置文件。");
            } else {
                error!("❌ 应用初始化失败: {}", e);
            }
            std::process::exit(1);
        }
    };

    // 运行命令
    if let Err(e) = app.run(cli.command).await {
        error!("❌ 操作失败: {}", e);
        std::process::exit(1);
    }
}
