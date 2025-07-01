use crate::docker_service::error::{DockerServiceError, DockerServiceResult};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

/// 目录权限管理器
pub struct DirectoryPermissionManager {
    work_dir: PathBuf,
}

/// 目录权限配置
struct DirectoryPermissionConfig {
    /// 目录路径（相对于工作目录）
    path: &'static str,
    /// 目录用途描述
    description: &'static str,
}

impl DirectoryPermissionManager {
    /// 创建新的目录权限管理器
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// 获取需要管理权限的目录配置
    fn get_directory_configs() -> Vec<DirectoryPermissionConfig> {
        vec![
            DirectoryPermissionConfig {
                path: "docker",
                description: "Docker根目录 (包含所有子目录和文件)",
            },
        ]
    }

    /// 检查并修复Docker相关目录权限
    pub async fn check_and_fix_directory_permissions(&self) -> DockerServiceResult<()> {
        info!("🔍 检查Docker相关目录权限...");

        let is_windows = cfg!(target_os = "windows");
        if is_windows {
            info!("🪟 检测到Windows环境，跳过权限检查");
            return Ok(());
        }

        let directory_configs = Self::get_directory_configs();
        let mut fixed_count = 0;
        let mut error_count = 0;

        for config in directory_configs {
            let dir_path = self.work_dir.join(config.path);
            
            match self.check_and_fix_directory_permission_recursive(&dir_path, &config).await {
                Ok(was_fixed) => {
                    if was_fixed {
                        fixed_count += 1;
                        info!("✅ 已修复目录权限: {} ({})", 
                              dir_path.display(), config.description);
                    } else {
                        debug!("✓ 目录权限正常: {} ({})", 
                               dir_path.display(), config.description);
                    }
                }
                Err(e) => {
                    error_count += 1;
                    error!("❌ 修复目录权限失败 {} ({}): {}", 
                           dir_path.display(), config.description, e);
                }
            }
        }

        // 汇总结果
        if fixed_count > 0 {
            info!("🛠️  已修复 {} 个目录的权限", fixed_count);
        }

        if error_count > 0 {
            warn!("⚠️  {} 个目录权限修复失败，可能需要手动处理", error_count);
        } else {
            info!("✅ 目录权限检查完成");
        }

        Ok(())
    }

    /// 递归检查并修复目录权限
    async fn check_and_fix_directory_permission_recursive(
        &self,
        dir_path: &Path,
        _config: &DirectoryPermissionConfig,
    ) -> DockerServiceResult<bool> {
        // 确保目录存在
        if !dir_path.exists() {
            debug!("创建目录: {}", dir_path.display());
            tokio::fs::create_dir_all(dir_path).await.map_err(|e| {
                DockerServiceError::FileSystem(format!(
                    "创建目录失败 {}: {}",
                    dir_path.display(),
                    e
                ))
            })?;
        }

        info!("🔧 设置目录权限: {} -> 755 (递归)", dir_path.display());
        
        // 使用 chmod -R 递归设置权限
        let output = std::process::Command::new("chmod")
            .arg("-R")
            .arg("755")
            .arg(dir_path)
            .output()
            .map_err(|e| {
                DockerServiceError::Permission(format!(
                    "执行chmod -R命令失败: {}",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerServiceError::Permission(format!(
                "chmod -R命令执行失败: {}",
                stderr
            )));
        }

        info!("✅ Docker目录权限递归设置完成: {}", dir_path.display());
        Ok(true)
    }
} 