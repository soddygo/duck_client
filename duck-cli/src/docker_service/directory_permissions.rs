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
    
    /// 渐进式权限管理（用户建议的简单方案）
    pub fn progressive_permission_management(&self) -> DockerServiceResult<()> {
        info!("🔧 开始渐进式权限管理...");
        
        // 第一步：整个docker目录设置为755权限
        self.set_docker_base_permissions()?;
        
        // 第二步：预处理MySQL目录为777权限
        self.prepare_mysql_directory()?;
        
        info!("✅ 渐进式权限管理完成");
        Ok(())
    }
    
    /// 第一步：设置Docker基础目录为755权限
    fn set_docker_base_permissions(&self) -> DockerServiceResult<()> {
        info!("📁 设置Docker基础目录权限为755...");
        
        let base_directories = [
            "config", 
            "logs",
            "app",
            "upload",
            "backups",
            "data",  // data目录也先设置为755
        ];
        
        for dir_name in &base_directories {
            let dir_path = self.work_dir.join(dir_name);
            
            // 确保目录存在
            if !dir_path.exists() {
                fs::create_dir_all(&dir_path)
                    .map_err(|e| DockerServiceError::FileSystem(format!("创建目录 {} 失败: {}", dir_path.display(), e)))?;
                info!("✅ 已创建目录: {}", dir_path.display());
            }
            
            // 设置为755权限（不递归，只设置顶级目录）
            self.set_directory_permission(&dir_path, 0o755)?;
            info!("✅ 已设置目录权限 {} → 755", dir_name);
        }
        
        Ok(())
    }
    
    /// 第二步：预处理MySQL目录为777权限
    fn prepare_mysql_directory(&self) -> DockerServiceResult<()> {
        info!("🔑 预处理MySQL目录权限为777...");
        
        let mysql_data_dir = self.work_dir.join("data/mysql");
        
        // 确保MySQL数据目录存在
        if !mysql_data_dir.exists() {
            fs::create_dir_all(&mysql_data_dir)
                .map_err(|e| DockerServiceError::FileSystem(format!("创建MySQL数据目录失败: {}", e)))?;
            info!("✅ 已创建MySQL数据目录");
        }
        
        // 递归设置MySQL目录及所有内容为777权限
        self.set_directory_permissions_recursive(&mysql_data_dir, 0o777)?;
        info!("🔑 已设置MySQL目录权限 → 777 (递归)");
        
        Ok(())
    }
    
    /// MySQL容器启动失败时的权限修复（安全版本 - 不删除用户数据）
    pub fn fix_mysql_permissions_on_failure(&self) -> DockerServiceResult<()> {
        warn!("🔧 MySQL容器启动失败，进行安全权限修复（不删除数据）...");
        
        let mysql_data_dir = self.work_dir.join("data/mysql");
        let mysql_logs_dir = self.work_dir.join("logs/mysql");
        
        // 1. 检查MySQL数据目录状态
        if mysql_data_dir.exists() {
            info!("📁 检测到现有MySQL数据目录，保护用户数据...");
            
            // 安全检查：判断是否为全新目录
            if let Ok(entries) = fs::read_dir(&mysql_data_dir) {
                let entries: Vec<_> = entries.collect();
                let entry_count = entries.len();
                
                if entry_count > 0 {
                    // 检查是否只包含损坏的初始化文件
                    let safe_to_clean = self.is_safe_to_clean_mysql_dir(&mysql_data_dir)?;
                    
                    if safe_to_clean {
                        warn!("🔍 检测到损坏的MySQL初始化文件（{}项），安全清理...", entry_count);
                        self.safe_cleanup_mysql_init_files(&mysql_data_dir)?;
                    } else {
                        warn!("⚠️  检测到可能的用户数据（{}项），仅修复权限，不删除数据", entry_count);
                        info!("🛡️  如果需要重新初始化，请手动备份并清理数据目录");
                        
                        // 仅修复权限，不删除数据
                        self.fix_existing_mysql_permissions(&mysql_data_dir)?;
                        return Ok(());
                    }
                }
            }
        }
        
        // 2. 确保目录存在并设置正确权限
        self.ensure_mysql_directories(&mysql_data_dir, &mysql_logs_dir)?;
        
        // 3. 设置最宽松的权限以确保容器访问
        self.set_directory_permissions_recursive(&mysql_data_dir, 0o777)?;
        self.set_directory_permissions_recursive(&mysql_logs_dir, 0o777)?;
        info!("🔑 已设置MySQL目录权限 → 777 (数据+日志)");
        
        // 4. 确保父目录权限正确
        if let Some(data_parent) = mysql_data_dir.parent() {
            self.set_directory_permission(data_parent, 0o755)?;
        }
        
        info!("✅ MySQL安全权限修复完成");
        Ok(())
    }
    
    /// 判断MySQL目录是否安全清理（只包含损坏的初始化文件）
    fn is_safe_to_clean_mysql_dir(&self, mysql_dir: &Path) -> DockerServiceResult<bool> {
        let entries = fs::read_dir(mysql_dir)
            .map_err(|e| DockerServiceError::FileSystem(format!("读取MySQL目录失败: {}", e)))?;
        
        let mut has_user_data = false;
        let mut has_init_files = false;
        
        for entry in entries {
            let entry = entry.map_err(|e| DockerServiceError::FileSystem(format!("读取目录项失败: {}", e)))?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            // 检查是否有用户数据表明真实使用
            if self.is_likely_user_data(&file_name) {
                has_user_data = true;
                break;
            }
            
            // 检查是否有初始化相关文件
            if self.is_mysql_init_file(&file_name) {
                has_init_files = true;
            }
        }
        
        // 只有当没有用户数据且只有初始化文件时才安全清理
        let safe_to_clean = !has_user_data && has_init_files;
        
        if safe_to_clean {
            info!("🔍 判断为安全清理：无用户数据，仅有损坏的初始化文件");
        } else if has_user_data {
            warn!("🛡️  检测到用户数据，拒绝自动清理");
        }
        
        Ok(safe_to_clean)
    }
    
    /// 判断文件名是否为可能的用户数据
    fn is_likely_user_data(&self, file_name: &str) -> bool {
        // 用户数据库文件特征
        let user_data_patterns = [
            // 用户创建的数据库目录
            "agent_platform",
            "agent_custom",
            "custom_",
            "app_",
            "user_",
            // 具有数据的系统表文件（大小检查在调用处）
            "mysql.ibd",
            // 事务日志文件（通常表明有用户操作）
            "undo_001",
            "undo_002",
            // 二进制日志
            "mysql-bin",
            "binlog",
        ];
        
        for pattern in &user_data_patterns {
            if file_name.contains(pattern) {
                return true;
            }
        }
        
        false
    }
    
    /// 判断文件名是否为MySQL初始化文件
    fn is_mysql_init_file(&self, file_name: &str) -> bool {
        let init_patterns = [
            "ib_buffer_pool",
            "#ib_",
            "auto.cnf",
            "mysql.sock",
            "ca-key.pem",
            "ca.pem",
            "client-cert.pem",
            "client-key.pem",
            "private_key.pem",
            "public_key.pem",
            "server-cert.pem",
            "server-key.pem",
            // 空的或很小的系统文件
            "ibdata1",
            "ibtmp1",
        ];
        
        for pattern in &init_patterns {
            if file_name.contains(pattern) {
                return true;
            }
        }
        
        false
    }
    
    /// 安全清理MySQL初始化文件
    fn safe_cleanup_mysql_init_files(&self, mysql_dir: &Path) -> DockerServiceResult<()> {
        info!("🗑️  安全清理损坏的MySQL初始化文件...");
        
        let entries = fs::read_dir(mysql_dir)
            .map_err(|e| DockerServiceError::FileSystem(format!("读取MySQL目录失败: {}", e)))?;
        
        let mut cleaned_count = 0;
        
        for entry in entries {
            let entry = entry.map_err(|e| DockerServiceError::FileSystem(format!("读取目录项失败: {}", e)))?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            // 只删除确认的初始化文件
            if self.is_mysql_init_file(&file_name) && !self.is_likely_user_data(&file_name) {
                if path.is_file() {
                    if let Err(e) = fs::remove_file(&path) {
                        warn!("删除文件 {} 失败: {}", path.display(), e);
                    } else {
                        cleaned_count += 1;
                        debug!("已删除初始化文件: {}", file_name);
                    }
                } else if path.is_dir() {
                    // 对于目录，更谨慎处理
                    if self.is_safe_init_directory(&file_name) {
                        if let Err(e) = fs::remove_dir_all(&path) {
                            warn!("删除目录 {} 失败: {}", path.display(), e);
                        } else {
                            cleaned_count += 1;
                            debug!("已删除初始化目录: {}", file_name);
                        }
                    }
                }
            }
        }
        
        info!("✅ 安全清理完成，删除了 {} 个损坏的初始化文件", cleaned_count);
        Ok(())
    }
    
    /// 判断目录是否为安全的初始化目录
    fn is_safe_init_directory(&self, dir_name: &str) -> bool {
        let safe_dirs = [
            "#innodb_redo",
            "#innodb_temp",
            "mysql", // 只有在确认为空的系统mysql目录时
            "performance_schema",
            "sys",
        ];
        
        safe_dirs.iter().any(|&pattern| dir_name == pattern)
    }
    
    /// 修复现有MySQL数据的权限（不删除数据）
    fn fix_existing_mysql_permissions(&self, mysql_dir: &Path) -> DockerServiceResult<()> {
        info!("🔧 修复现有MySQL数据权限（保护用户数据）...");
        
        // 递归修复所有文件和目录的权限
        for entry in WalkDir::new(mysql_dir) {
            let entry = entry.map_err(|e| DockerServiceError::FileSystem(format!("访问目录失败: {}", e)))?;
            let path = entry.path();
            
            if path.is_dir() {
                // 目录设置为777（drwxrwxrwx）
                self.set_directory_permission(path, 0o777)?;
            } else {
                // 文件设置为666（-rw-rw-rw-）
                let metadata = fs::metadata(path)
                    .map_err(|e| DockerServiceError::FileSystem(format!("获取文件元数据失败: {}", e)))?;
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o666);
                fs::set_permissions(path, permissions)
                    .map_err(|e| DockerServiceError::FileSystem(format!("设置文件权限失败: {}", e)))?;
            }
        }
        
        info!("✅ 现有数据权限修复完成，用户数据已保护");
        Ok(())
    }
    
    /// 确保MySQL相关目录存在
    fn ensure_mysql_directories(&self, mysql_data_dir: &Path, mysql_logs_dir: &Path) -> DockerServiceResult<()> {
        if !mysql_data_dir.exists() {
            fs::create_dir_all(mysql_data_dir)
                .map_err(|e| DockerServiceError::FileSystem(format!("创建MySQL数据目录失败: {}", e)))?;
            info!("✅ 已创建MySQL数据目录");
        }
        
        if !mysql_logs_dir.exists() {
            fs::create_dir_all(mysql_logs_dir)
                .map_err(|e| DockerServiceError::FileSystem(format!("创建MySQL日志目录失败: {}", e)))?;
            info!("✅ 已创建MySQL日志目录");
        }
        
        Ok(())
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