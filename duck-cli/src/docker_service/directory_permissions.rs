use crate::docker_service::error::{DockerServiceError, DockerServiceResult};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn, error};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use walkdir::WalkDir;
use ducker::docker::{container::DockerContainer, util::new_local_docker_connection};
use tokio::time::{sleep, Duration};

/// 目录权限管理器 - 专注于统一用户ID映射
#[derive(Debug, Clone)]
pub struct DirectoryPermissionManager {
    work_dir: PathBuf,
    current_uid: u32,
    current_gid: u32,
}



impl DirectoryPermissionManager {
    /// 创建新的目录权限管理器
    pub fn new(work_dir: PathBuf) -> Self {
        // 获取当前用户的UID和GID
        let current_uid = unsafe { libc::getuid() };
        let current_gid = unsafe { libc::getgid() };
        
        info!("🔧 初始化权限管理器，当前用户: {}:{}", current_uid, current_gid);
        
        Self {
            work_dir,
            current_uid,
            current_gid,
        }
    }

    /// 智能权限管理 - 基于业界最佳实践的简化方案
    pub fn smart_permission_management(&self) -> DockerServiceResult<()> {
        info!("🔧 开始智能权限管理（基于Docker最佳实践）");
        
        // 第一步：确保数据目录存在（总是执行）
        match self.ensure_data_directories() {
            Ok(_) => {
                info!("✅ 目录创建和基础权限设置成功");
            }
            Err(e) => {
                error!("❌ 目录创建失败: {}", e);
                return Err(e);
            }
        }
        
        // 第二步：使用环境变量方式设置用户映射（业界推荐）
        match self.apply_env_based_user_mapping() {
            Ok(_) => {
                info!("✅ 环境变量用户映射设置成功");
            }
            Err(e) => {
                warn!("⚠️ 环境变量用户映射失败，回退到基础权限: {}", e);
                // 回退：设置宽松权限
                self.set_basic_permissions()?;
            }
        }
        
        info!("✅ 智能权限管理完成");
        Ok(())
    }
    
    /// 基于环境变量的用户映射（业界最佳实践）
    fn apply_env_based_user_mapping(&self) -> DockerServiceResult<()> {
        info!("📝 应用基于环境变量的用户映射（Docker最佳实践）");
        
        if let Some(compose_file) = self.find_compose_file() {
            let content = fs::read_to_string(&compose_file)
                .map_err(|e| DockerServiceError::FileSystem(format!("读取docker-compose.yml失败: {}", e)))?;
            
            // 检查是否已经有.env文件
            let env_file = compose_file.parent().unwrap().join(".env");
            if !env_file.exists() {
                self.create_env_file(&env_file)?;
            }
            
            // 确保docker-compose.yml使用环境变量
            let modified_content = self.ensure_env_variables_in_compose(&content)?;
            
            // 备份并写入
            let backup_file = compose_file.with_extension("yml.backup");
            fs::copy(&compose_file, &backup_file)
                .map_err(|e| DockerServiceError::FileSystem(format!("备份文件失败: {}", e)))?;
            
            fs::write(&compose_file, modified_content)
                .map_err(|e| DockerServiceError::FileSystem(format!("写入docker-compose.yml失败: {}", e)))?;
            
            info!("✅ 环境变量用户映射配置完成");
            Ok(())
        } else {
            Err(DockerServiceError::Configuration("未找到docker-compose.yml文件".to_string()))
        }
    }
    
    /// 创建.env文件（Docker官方推荐方式）
    fn create_env_file(&self, env_file: &Path) -> DockerServiceResult<()> {
        let env_content = format!(
            "# Docker Compose环境变量（自动生成）
UID={}
GID={}
MYSQL_ROOT_PASSWORD={}
MYSQL_USER=admin
MYSQL_PASSWORD={}

# 数据目录权限
DATA_DIR_PERMISSIONS=755
",
            self.current_uid,
            self.current_gid,
            self.generate_random_password(),
            self.generate_random_password()
        );
        
        fs::write(env_file, env_content)
            .map_err(|e| DockerServiceError::FileSystem(format!("创建.env文件失败: {}", e)))?;
        
        info!("✅ 已创建.env文件: {}", env_file.display());
        Ok(())
    }
    
    /// 确保docker-compose.yml使用环境变量
    fn ensure_env_variables_in_compose(&self, content: &str) -> DockerServiceResult<String> {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut modified_lines = Vec::new();
        let mut in_mysql_service = false;
        let mut has_user_mapping = false;
        
        for line in lines {
            if line.trim().starts_with("mysql:") || line.trim().starts_with("db:") {
                in_mysql_service = true;
                modified_lines.push(line);
            } else if in_mysql_service && line.starts_with("  ") && line.trim().ends_with(':') && !line.starts_with("    ") {
                // 下一个服务开始
                in_mysql_service = false;
                modified_lines.push(line);
            } else if in_mysql_service && line.trim().starts_with("user:") {
                // 更新现有的user配置
                modified_lines.push("    user: \"${UID:-1000}:${GID:-1000}\"".to_string());
                has_user_mapping = true;
            } else if in_mysql_service && line.trim().starts_with("image:") && !has_user_mapping {
                // 在image后添加user配置
                modified_lines.push(line);
                modified_lines.push("    user: \"${UID:-1000}:${GID:-1000}\"".to_string());
                has_user_mapping = true;
            } else {
                modified_lines.push(line);
            }
        }
        
        Ok(modified_lines.join("\n"))
    }
    
    /// 设置基础权限（回退方案）
    fn set_basic_permissions(&self) -> DockerServiceResult<()> {
        info!("🔧 应用基础权限设置（回退方案）");
        
        let data_dir = self.work_dir.join("data");
        if data_dir.exists() {
            // 设置775权限（稍微宽松一些）
            self.set_directory_permissions_recursive(&data_dir, 0o775)?;
            info!("✅ 数据目录权限设置为775");
        }
        
        let logs_dir = self.work_dir.join("logs");
        if logs_dir.exists() {
            self.set_directory_permissions_recursive(&logs_dir, 0o775)?;
            info!("✅ 日志目录权限设置为775");
        }
        
        Ok(())
    }
    
    /// 确保数据目录存在并设置合适权限（动态扫描volume映射）
    fn ensure_data_directories(&self) -> DockerServiceResult<()> {
        info!("📁 动态扫描docker-compose.yml获取volume映射...");
        
        if let Some(compose_file) = self.find_compose_file() {
            let content = fs::read_to_string(&compose_file)
                .map_err(|e| DockerServiceError::FileSystem(format!("读取docker-compose.yml失败: {}", e)))?;
            
            // 动态提取所有bind mount目录
            let bind_mount_dirs = self.extract_bind_mount_directories(&content)?;
            info!("🔍 发现 {} 个bind mount目录: {:?}", bind_mount_dirs.len(), bind_mount_dirs);
            
            // 只确保bind mount的父目录存在，让容器自己创建子目录
            for bind_mount in bind_mount_dirs {
                self.ensure_bind_mount_parent_directory(&bind_mount)?;
            }
            
            info!("✅ bind mount父目录检查完成");
        } else {
            warn!("⚠️ 未找到docker-compose.yml文件，使用默认data目录");
            // 确保基础data目录存在
            let data_dir = self.work_dir.join("data");
            if !data_dir.exists() {
                fs::create_dir_all(&data_dir)
                    .map_err(|e| DockerServiceError::FileSystem(format!("创建基础data目录失败: {}", e)))?;
                info!("✅ 已创建基础data目录");
            }
            self.set_directory_permission(&data_dir, 0o755)?;
        }
        
        Ok(())
    }
    
    /// 从docker-compose.yml中提取所有bind mount目录
    fn extract_bind_mount_directories(&self, content: &str) -> DockerServiceResult<Vec<String>> {
        let lines: Vec<&str> = content.lines().collect();
        let mut bind_mounts = Vec::new();
        let mut in_volumes_section = false;
        
        for line in lines {
            let trimmed = line.trim();
            
            // 检查是否在volumes部分
            if trimmed == "volumes:" {
                in_volumes_section = true;
                continue;
            }
            
            // 如果遇到其他配置项，退出volumes部分
            if in_volumes_section && !line.starts_with(' ') && !line.trim().is_empty() {
                in_volumes_section = false;
            }
            
            // 在volumes部分中查找bind mount（以 ./或/开头的路径映射）
            if in_volumes_section && trimmed.contains(':') {
                if let Some(host_path) = self.extract_host_path_from_volume(trimmed) {
                    // 只处理相对路径的bind mount（./data/xxx 这种）
                    if host_path.starts_with("./") {
                        debug!("📂 发现bind mount: {}", host_path);
                        bind_mounts.push(host_path);
                    }
                }
            }
        }
        
        // 去重并排序
        bind_mounts.sort();
        bind_mounts.dedup();
        
        Ok(bind_mounts)
    }
    
    /// 从volume配置行中提取主机路径
    fn extract_host_path_from_volume(&self, volume_line: &str) -> Option<String> {
        // 支持多种格式：
        // - ./data/mysql:/var/lib/mysql
        // - "./data/redis:/data"  
        // - ./logs/app:/app/logs:rw
        
        let cleaned = volume_line.trim_start_matches('-').trim().trim_matches('"');
        
        if let Some(colon_pos) = cleaned.find(':') {
            let host_path = cleaned[..colon_pos].trim();
            if host_path.starts_with("./") || host_path.starts_with('/') {
                return Some(host_path.to_string());
            }
        }
        
        None
    }
    
    /// 确保bind mount的父目录存在并有正确权限
    fn ensure_bind_mount_parent_directory(&self, bind_mount_path: &str) -> DockerServiceResult<()> {
        // 移除 "./" 前缀，获取相对路径
        let relative_path = bind_mount_path.strip_prefix("./").unwrap_or(bind_mount_path);
        let full_path = self.work_dir.join(relative_path);
        
        // 获取父目录路径
        if let Some(parent_dir) = full_path.parent() {
            // 确保父目录存在
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir)
                    .map_err(|e| DockerServiceError::FileSystem(format!("创建父目录 {} 失败: {}", parent_dir.display(), e)))?;
                info!("✅ 已创建父目录: {}", parent_dir.display());
            }
            
            // 设置父目录权限为755（读写执行 for owner，读执行 for group/others）
            self.set_directory_permission(parent_dir, 0o755)?;
        }
        
        // 如果bind mount目录本身已存在，也设置权限
        if full_path.exists() {
            self.set_directory_permission(&full_path, 0o755)?;
            info!("✅ 已设置bind mount目录权限: {}", relative_path);
        } else {
            info!("📋 bind mount目录将由容器创建: {}", relative_path);
        }
        
        Ok(())
    }
    
    /// 设置单个目录权限
    fn set_directory_permission(&self, path: &Path, mode: u32) -> DockerServiceResult<()> {
        let metadata = fs::metadata(path)
            .map_err(|e| DockerServiceError::FileSystem(format!("获取文件元数据失败: {}", e)))?;
            
        let mut permissions = metadata.permissions();
        permissions.set_mode(mode);
        
        fs::set_permissions(path, permissions)
            .map_err(|e| DockerServiceError::FileSystem(format!("设置权限失败: {}", e)))?;
            
        Ok(())
    }
    
    /// 查找docker-compose文件
    fn find_compose_file(&self) -> Option<PathBuf> {
        let candidates = vec![
            self.work_dir.join("docker-compose.yml"),
            self.work_dir.join("docker").join("docker-compose.yml"), 
            self.work_dir.parent()?.join("docker-compose.yml"),
            self.work_dir.parent()?.join("docker").join("docker-compose.yml"),
        ];
        
        for candidate in candidates {
            if candidate.exists() {
                debug!("找到docker-compose文件: {}", candidate.display());
                return Some(candidate);
            }
        }
        
        warn!("未找到docker-compose.yml文件");
        None
    }
    
    /// 递归设置目录权限
    fn set_directory_permissions_recursive(&self, dir: &Path, mode: u32) -> DockerServiceResult<()> {
        for entry in WalkDir::new(dir) {
            let entry = entry.map_err(|e| DockerServiceError::FileSystem(format!("访问目录失败: {}", e)))?;
            let path = entry.path();
            
            if path.is_dir() {
                self.set_directory_permission(path, mode)?;
            }
        }
        
        Ok(())
    }
    
    /// 生成随机密码
    fn generate_random_password(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("secure_{}_pwd", timestamp)
    }
    
    /// 基础权限修复（兼容性方法）
    pub fn basic_permission_fix(&self) -> DockerServiceResult<()> {
        info!("🔧 执行基础权限修复...");
        self.set_basic_permissions()
    }
    
    /// 容器启动后的权限维护（兼容性方法）
    pub async fn post_container_start_maintenance(&self) -> DockerServiceResult<()> {
        info!("🔧 执行容器启动后权限维护...");
        
        // 简化版本：只做基础的权限修复
        self.set_basic_permissions()?;
        
        info!("✅ 容器启动后权限维护完成");
        Ok(())
    }
}



/// 简化的智能权限管理入口函数
pub fn smart_permission_management(data_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let manager = DirectoryPermissionManager::new(data_dir.to_path_buf());
    manager.smart_permission_management()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// 基础权限修复入口函数  
pub fn basic_permission_fix(data_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let manager = DirectoryPermissionManager::new(data_dir.to_path_buf());
    manager.set_basic_permissions()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
} 