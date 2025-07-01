use crate::docker_service::error::{DockerServiceError, DockerServiceResult};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use std::collections::HashMap;
use std::fs;

/// 目录权限管理器
pub struct DirectoryPermissionManager {
    work_dir: PathBuf,
}

/// 目录权限配置
struct DirectoryConfig {
    path: &'static str,
    owner_uid: Option<u32>,  // None表示保持当前所有者
    owner_gid: Option<u32>,  // None表示保持当前所有者
    permission: &'static str,
    description: &'static str,
}

/// Docker Compose卷转换器
pub struct DockerComposeVolumeConverter {
    compose_file_path: PathBuf,
}

impl DockerComposeVolumeConverter {
    pub fn new(compose_file_path: PathBuf) -> Self {
        Self { compose_file_path }
    }

    /// 自动转换bind mount为named volumes
    pub fn convert_to_named_volumes(&self) -> DockerServiceResult<()> {
        info!("🔄 开始分析docker-compose.yml文件进行卷转换...");
        
        if !self.compose_file_path.exists() {
            return Err(DockerServiceError::FileSystem(format!(
                "docker-compose.yml文件不存在: {}",
                self.compose_file_path.display()
            )));
        }

        let content = fs::read_to_string(&self.compose_file_path)
            .map_err(|e| DockerServiceError::FileSystem(format!(
                "无法读取docker-compose.yml: {e}"
            )))?;

        let data_mount_patterns = self.identify_data_mounts(&content);
        
        if data_mount_patterns.is_empty() {
            info!("✅ 没有发现需要转换的数据挂载");
            return Ok(());
        }

        info!("🔍 发现 {} 个数据挂载需要转换为Named Volumes:", data_mount_patterns.len());
        for (service, mounts) in &data_mount_patterns {
            for mount in mounts {
                info!("  - {}: {} -> {}", service, mount.host_path, mount.container_path);
            }
        }

        // 生成转换后的compose文件
        let converted_content = self.convert_content(&content, &data_mount_patterns)?;
        
        // 备份原文件
        let backup_path = self.compose_file_path.with_extension("yml.backup");
        fs::copy(&self.compose_file_path, &backup_path)
            .map_err(|e| DockerServiceError::FileSystem(format!(
                "备份原文件失败: {e}"
            )))?;
        info!("📋 已备份原文件到: {}", backup_path.display());

        // 写入转换后的内容
        fs::write(&self.compose_file_path, converted_content)
            .map_err(|e| DockerServiceError::FileSystem(format!(
                "写入转换后的文件失败: {e}"
            )))?;

        info!("🎉 docker-compose.yml转换完成！");
        info!("💡 使用Named Volumes后将不再有权限问题");
        Ok(())
    }

    /// 识别需要转换的数据挂载
    fn identify_data_mounts(&self, content: &str) -> HashMap<String, Vec<VolumeMount>> {
        let mut result = HashMap::new();
        let mut current_service = String::new();
        let mut in_volumes_section = false;
        let mut indent_level = 0;

        for line in content.lines() {
            let trimmed = line.trim();
            let line_indent = line.len() - line.trim_start().len();
            
            // 跳过空行和注释
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            
            // 检测顶级服务定义（只在services部分）
            if line_indent == 2 && trimmed.ends_with(':') && !trimmed.starts_with('-') {
                if let Some(service_name) = trimmed.strip_suffix(':') {
                    // 确保是有效的服务名（不包含特殊字符，不是特殊关键字）
                    if service_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') 
                        && !["volumes", "networks", "environment", "ports", "depends_on", "command", "entrypoint"].contains(&service_name) {
                        current_service = service_name.to_string();
                        in_volumes_section = false;
                        debug!("发现服务: {}", current_service);
                        continue;
                    }
                }
            }
            
            // 检测服务内的volumes部分
            if line_indent == 4 && trimmed == "volumes:" && !current_service.is_empty() {
                in_volumes_section = true;
                indent_level = line_indent;
                debug!("进入服务 {} 的volumes部分", current_service);
                continue;
            }
            
            // 当遇到同级别或更高级别的配置时，退出volumes部分
            if in_volumes_section && line_indent <= indent_level && !trimmed.starts_with('-') {
                in_volumes_section = false;
                debug!("退出服务 {} 的volumes部分", current_service);
            }
            
            // 处理volume挂载
            if in_volumes_section && line_indent > indent_level && trimmed.starts_with("- ./data/") {
                if let Some(mount) = self.parse_volume_mount(trimmed) {
                    if self.is_data_volume(&mount.host_path) {
                        debug!("发现数据挂载: {} -> {} (服务: {})", mount.host_path, mount.container_path, current_service);
                        result.entry(current_service.clone())
                            .or_insert_with(Vec::new)
                            .push(mount);
                    }
                }
            }
        }
        
        result
    }

    /// 解析volume挂载行
    fn parse_volume_mount(&self, line: &str) -> Option<VolumeMount> {
        let line = line.trim_start_matches("- ");
        if let Some(colon_pos) = line.find(':') {
            let host_path = line[..colon_pos].trim().to_string();
            let rest = &line[colon_pos + 1..];
            let container_path = if let Some(second_colon) = rest.find(':') {
                rest[..second_colon].trim().to_string()
            } else {
                rest.trim().to_string()
            };
            
            Some(VolumeMount {
                host_path,
                container_path,
                original_line: line.to_string(),
            })
        } else {
            None
        }
    }

    /// 判断是否为数据卷
    fn is_data_volume(&self, host_path: &str) -> bool {
        host_path.starts_with("./data/") && 
        !host_path.contains(".conf") && 
        !host_path.contains(".yml") && 
        !host_path.contains(".yaml") &&
        !host_path.contains(".json") &&
        !host_path.contains(".sql")
    }

    /// 转换文件内容
    fn convert_content(&self, content: &str, data_mounts: &HashMap<String, Vec<VolumeMount>>) -> DockerServiceResult<String> {
        let mut result = content.to_string();
        let mut named_volumes = Vec::new();

        // 替换bind mount为named volumes
        for (service, mounts) in data_mounts {
            for mount in mounts {
                let volume_name = self.generate_volume_name(service, &mount.host_path);
                let new_mount = format!("{}:{}", volume_name, mount.container_path);
                
                // 精确替换原始挂载行
                let original_line = format!("      - {}", mount.original_line);
                let new_line = format!("      - {new_mount}");
                
                if result.contains(&original_line) {
                    result = result.replace(&original_line, &new_line);
                    named_volumes.push(volume_name.clone());
                    info!("✅ 已转换: {} -> {}", mount.original_line, new_mount);
                } else {
                    warn!("⚠️ 未找到要替换的行: {}", original_line);
                }
            }
        }

        // 添加volumes定义到文件末尾
        if !named_volumes.is_empty() {
            named_volumes.sort();
            named_volumes.dedup();
            
            let volumes_section = self.generate_volumes_section(&named_volumes);
            
            // 检查是否已存在顶级volumes部分
            if !result.contains("\nvolumes:\n") && !result.ends_with("volumes:") {
                // 如果不存在，在文件末尾添加
                if !result.ends_with('\n') {
                    result.push('\n');
                }
                result.push_str(&volumes_section);
            } else {
                // 如果存在，需要更智能的合并（这里简化处理）
                warn!("⚠️ 检测到已存在volumes部分，请手动检查生成的文件");
                result.push_str("\n# 自动生成的Named Volumes（请合并到existing volumes部分）:\n");
                result.push_str(&format!("# {}", volumes_section.replace('\n', "\n# ")));
            }
        }

        Ok(result)
    }

    /// 生成volume名称
    fn generate_volume_name(&self, service: &str, host_path: &str) -> String {
        let path_part = host_path
            .strip_prefix("./data/")
            .unwrap_or(host_path)
            .replace('/', "_");
        format!("{service}_data_{path_part}")
    }

    /// 生成volumes部分
    fn generate_volumes_section(&self, volume_names: &[String]) -> String {
        let mut section = String::from("volumes:\n");
        for volume_name in volume_names {
            section.push_str(&format!("  {volume_name}:\n"));
        }
        section
    }
}

#[derive(Debug, Clone)]
struct VolumeMount {
    host_path: String,
    container_path: String,
    original_line: String,
}

impl DirectoryPermissionManager {
    /// 创建新的目录权限管理器
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// 获取目录权限配置
    fn get_directory_configs() -> Vec<DirectoryConfig> {
        vec![
            DirectoryConfig {
                path: "data/mysql",
                owner_uid: Some(999),  // MySQL容器用户
                owner_gid: Some(999),  // MySQL容器组
                permission: "755",
                description: "MySQL数据目录",
            },
            DirectoryConfig {
                path: "data",
                owner_uid: None,  // 保持当前所有者
                owner_gid: None,  
                permission: "755",
                description: "通用数据目录",
            },
            DirectoryConfig {
                path: "config",
                owner_uid: None,
                owner_gid: None,
                permission: "755",
                description: "配置文件目录",
            },
            DirectoryConfig {
                path: "logs",
                owner_uid: None,
                owner_gid: None,
                permission: "755",
                description: "日志目录",
            },
            DirectoryConfig {
                path: "app",
                owner_uid: None,
                owner_gid: None,
                permission: "755",
                description: "应用程序目录",
            },
            DirectoryConfig {
                path: "upload",
                owner_uid: None,
                owner_gid: None,
                permission: "755",
                description: "上传文件目录",
            },
            DirectoryConfig {
                path: "backups",
                owner_uid: None,
                owner_gid: None,
                permission: "755",
                description: "备份目录",
            },
        ]
    }

    /// 检查并设置目录权限 (传统方案)
    pub fn ensure_permissions(&self) -> DockerServiceResult<()> {
        info!("🔧 开始检查和设置Docker目录权限...");

        // Windows系统跳过权限检查
        if cfg!(target_os = "windows") {
            info!("✅ Windows系统，跳过权限检查");
            return Ok(());
        }

        let configs = Self::get_directory_configs();
        
        for config in configs {
            let full_path = self.work_dir.join(config.path);
            
            // 确保目录存在
            if let Err(e) = std::fs::create_dir_all(&full_path) {
                warn!("⚠️  创建目录失败 {}: {}", full_path.display(), e);
                continue;
            }

            // 设置所有者（如果指定）
            if let (Some(uid), Some(gid)) = (config.owner_uid, config.owner_gid) {
                match self.set_directory_owner(&full_path, uid, gid) {
                    Ok(_) => {
                        info!("✅ 设置所有者成功: {} -> {}:{}", full_path.display(), uid, gid);
                    }
                    Err(e) => {
                        warn!("⚠️ 无法设置目录所有者 {} -> {}:{}: {}", full_path.display(), uid, gid, e);
                        warn!("🔧 回退到777权限方案以确保容器正常运行");
                        
                        // 回退到777权限
                        if let Err(e) = self.set_directory_permissions(&full_path, "777") {
                            error!("❌ 设置777权限也失败 {}: {}", full_path.display(), e);
                            return Err(DockerServiceError::Permission(format!(
                                "无法设置目录权限 {}: {}",
                                full_path.display(),
                                e
                            )));
                        }
                        info!("✅ 已设置777权限: {} (安全权限方案失败后的回退)", full_path.display());
                        continue; // 跳过正常的权限设置，因为已经设置了777
                    }
                }
            }

            // 设置权限
            if let Err(e) = self.set_directory_permissions(&full_path, config.permission) {
                error!("❌ 设置目录权限失败 {}: {}", full_path.display(), e);
                return Err(DockerServiceError::Permission(format!(
                    "无法设置目录权限 {}: {}",
                    full_path.display(),
                    e
                )));
            }

            info!("✅ 设置权限成功: {} -> {} ({})", 
                  full_path.display(), config.permission, config.description);
        }

        info!("🎉 所有Docker目录权限设置完成");
        Ok(())
    }

    /// 智能权限管理：使用bind mount + 安全权限配置方案
    pub fn smart_permission_management(&self) -> DockerServiceResult<()> {
        info!("🧠 启动智能权限管理 (bind mount + 权限配置)...");
        
        // 直接使用传统权限设置方案，确保用户可以直接操作文件
        info!("📁 使用bind mount方案，确保宿主机可直接访问文件");
        self.ensure_permissions()
    }

    /// 设置目录所有者
    fn set_directory_owner(&self, path: &Path, uid: u32, gid: u32) -> DockerServiceResult<()> {
        let path_str = path.to_string_lossy();
        
        debug!("设置所有者: chown -R {}:{} {}", uid, gid, path_str);
        
        let output = std::process::Command::new("chown")
            .args(["-R", &format!("{uid}:{gid}"), &path_str])
            .output()
            .map_err(|e| DockerServiceError::FileSystem(format!("执行chown命令失败: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerServiceError::FileSystem(format!(
                "chown命令执行失败: {stderr}"
            )));
        }

        Ok(())
    }

    /// 设置目录权限（递归）
    fn set_directory_permissions(&self, path: &Path, permission: &str) -> DockerServiceResult<()> {
        let path_str = path.to_string_lossy();
        
        debug!("设置权限: chmod -R {} {}", permission, path_str);
        
        let output = std::process::Command::new("chmod")
            .args(["-R", permission, &path_str])
            .output()
            .map_err(|e| DockerServiceError::FileSystem(format!("执行chmod命令失败: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerServiceError::FileSystem(format!(
                "chmod命令执行失败: {stderr}"
            )));
        }

        Ok(())
    }

    /// 检查目录权限
    pub fn check_permissions(&self) -> DockerServiceResult<()> {
        if cfg!(target_os = "windows") {
            return Ok(());
        }

        info!("🔍 检查Docker目录权限状态...");
        
        let configs = Self::get_directory_configs();
        
        for config in configs {
            let full_path = self.work_dir.join(config.path);
            
            if !full_path.exists() {
                warn!("⚠️  目录不存在: {}", full_path.display());
                continue;
            }

            // 检查权限
            let output = std::process::Command::new("ls")
                .args(["-ld", &full_path.to_string_lossy()])
                .output()
                .map_err(|e| DockerServiceError::FileSystem(format!("检查权限失败: {e}")))?;

            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                info!("📋 {}: {}", config.path, output_str.trim());
            }
        }
        
        Ok(())
    }
} 