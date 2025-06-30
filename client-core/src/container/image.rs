use crate::{Result, DuckError};
use super::types::DockerManager;
use std::path::Path;

impl DockerManager {
    /// 加载 Docker 镜像
    pub async fn load_image<P: AsRef<Path>>(&self, image_path: P) -> Result<()> {
        self.check_prerequisites().await?;
        
        let image_path = image_path.as_ref();
        if !image_path.exists() {
            return Err(DuckError::Docker(format!(
                "镜像文件不存在: {}",
                image_path.display()
            )));
        }

        let output = self.run_docker_command(&[
            "load",
            "-i",
            &image_path.to_string_lossy(),
        ]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DuckError::Docker(format!("加载镜像失败: {}", stderr)));
        }

        Ok(())
    }

    /// 拉取最新镜像
    pub async fn pull_images(&self) -> Result<()> {
        self.check_prerequisites().await?;
        
        let output = self.run_compose_command(&["pull"]).await?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DuckError::Docker(format!("拉取镜像失败: {}", stderr)));
        }

        Ok(())
    }
} 