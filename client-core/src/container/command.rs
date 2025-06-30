use super::types::DockerManager;
use crate::{DuckError, Result};
use std::process::Stdio;
use tokio::process::Command;

impl DockerManager {
    /// 检查 Docker 状态
    pub async fn check_docker_status(&self) -> Result<()> {
        // 检查 docker 命令
        if which::which("docker").is_err() {
            return Err(DuckError::Docker("Docker 未安装或不在 PATH 中".to_string()));
        }

        // 检查 Docker 服务是否运行
        let output = self.run_docker_command(&["info"]).await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DuckError::Docker(format!("Docker 服务未运行: {}", stderr)));
        }

        Ok(())
    }

    /// 检查 Docker 和 Docker Compose 是否可用
    pub async fn check_prerequisites(&self) -> Result<()> {
        // 首先检查 Docker Compose 文件是否存在
        if !self.compose_file.exists() {
            return Err(DuckError::Docker(format!(
                "Docker Compose 文件不存在: {}",
                self.compose_file.display()
            )));
        }

        // 检查 Docker 状态
        self.check_docker_status().await?;

        // 检查 docker-compose 或 docker compose 命令
        let compose_available = which::which("docker-compose").is_ok()
            || self
                .run_docker_command(&["compose", "--version"])
                .await
                .is_ok();

        if !compose_available {
            return Err(DuckError::Docker(
                "Docker Compose 未安装或不可用".to_string(),
            ));
        }

        Ok(())
    }

    /// 执行 docker-compose 命令
    pub(crate) async fn run_compose_command(&self, args: &[&str]) -> Result<std::process::Output> {
        // 尝试使用 docker compose（新语法）
        if let Ok(output) = self.run_docker_compose_subcommand(args).await {
            return Ok(output);
        }

        // 回退到 docker-compose（旧语法）
        self.run_docker_compose_standalone(args).await
    }

    /// 使用 docker compose 子命令
    async fn run_docker_compose_subcommand(&self, args: &[&str]) -> Result<std::process::Output> {
        let compose_path = self.compose_file.to_string_lossy().to_string();
        let mut cmd_args = vec!["compose", "-f", &compose_path];
        cmd_args.extend(args);

        self.run_docker_command(&cmd_args).await
    }

    /// 使用独立的 docker-compose 命令
    async fn run_docker_compose_standalone(&self, args: &[&str]) -> Result<std::process::Output> {
        let compose_path = self.compose_file.to_string_lossy().to_string();
        let mut cmd_args = vec!["-f", &compose_path];
        cmd_args.extend(args);

        let output = Command::new("docker-compose")
            .args(&cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        Ok(output)
    }

    /// 执行 docker 命令
    pub(crate) async fn run_docker_command(&self, args: &[&str]) -> Result<std::process::Output> {
        let output = Command::new("docker")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        Ok(output)
    }
}
