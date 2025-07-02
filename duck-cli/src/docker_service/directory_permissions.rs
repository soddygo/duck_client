use crate::docker_service::error::{DockerServiceError, DockerServiceResult};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// 目录权限管理器 - 专注于统一用户ID映射
#[derive(Debug, Clone)]
pub struct DirectoryPermissionManager {
    work_dir: PathBuf,
}

impl DirectoryPermissionManager {
    /// 创建新的目录权限管理器
    pub fn new(work_dir: PathBuf) -> Self {
        info!("🔧 初始化权限管理器");
        Self { work_dir }
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

    /// 设置目录权限（跨平台兼容）
    fn set_directory_permission(&self, path: &Path, mode: u32) -> DockerServiceResult<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let metadata = fs::metadata(path)
                .map_err(|e| DockerServiceError::FileSystem(format!("获取文件元数据失败: {e}")))?;

            let mut permissions = metadata.permissions();
            permissions.set_mode(mode);

            fs::set_permissions(path, permissions)
                .map_err(|e| DockerServiceError::FileSystem(format!("设置权限失败: {e}")))?;
        }

        #[cfg(windows)]
        {
            // Windows上跳过权限设置，仅记录日志
            tracing::debug!(
                "Windows系统跳过权限设置: {} (mode: {:o})",
                path.display(),
                mode
            );
        }

        Ok(())
    }

    /// 递归设置目录权限
    fn set_directory_permissions_recursive(
        &self,
        dir: &Path,
        mode: u32,
    ) -> DockerServiceResult<()> {
        for entry in WalkDir::new(dir) {
            let entry =
                entry.map_err(|e| DockerServiceError::FileSystem(format!("访问目录失败: {e}")))?;
            let path = entry.path();

            if path.is_dir() {
                self.set_directory_permission(path, mode)?;
            }
        }

        Ok(())
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
            "config", "logs", "app", "upload", "backups",
            "data", // data目录也先设置为755
        ];

        for dir_name in &base_directories {
            let dir_path = self.work_dir.join(dir_name);

            // 确保目录存在
            if !dir_path.exists() {
                fs::create_dir_all(&dir_path).map_err(|e| {
                    DockerServiceError::FileSystem(format!(
                        "创建目录 {} 失败: {}",
                        dir_path.display(),
                        e
                    ))
                })?;
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
            fs::create_dir_all(&mysql_data_dir).map_err(|e| {
                DockerServiceError::FileSystem(format!("创建MySQL数据目录失败: {e}"))
            })?;
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
                        warn!(
                            "🔍 检测到损坏的MySQL初始化文件（{}项），安全清理...",
                            entry_count
                        );
                        self.safe_cleanup_mysql_init_files(&mysql_data_dir)?;
                    } else {
                        warn!(
                            "⚠️  检测到可能的用户数据（{}项），仅修复权限，不删除数据",
                            entry_count
                        );
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
            .map_err(|e| DockerServiceError::FileSystem(format!("读取MySQL目录失败: {e}")))?;

        let mut has_user_data = false;
        let mut has_init_files = false;

        for entry in entries {
            let entry = entry
                .map_err(|e| DockerServiceError::FileSystem(format!("读取目录项失败: {e}")))?;
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
            .map_err(|e| DockerServiceError::FileSystem(format!("读取MySQL目录失败: {e}")))?;

        let mut cleaned_count = 0;

        for entry in entries {
            let entry = entry
                .map_err(|e| DockerServiceError::FileSystem(format!("读取目录项失败: {e}")))?;
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

        info!(
            "✅ 安全清理完成，删除了 {} 个损坏的初始化文件",
            cleaned_count
        );
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

        safe_dirs.contains(&dir_name)
    }

    /// 修复现有MySQL数据的权限（不删除数据）
    fn fix_existing_mysql_permissions(&self, mysql_dir: &Path) -> DockerServiceResult<()> {
        info!("🔧 修复现有MySQL数据权限（保护用户数据）...");

        // 递归修复所有文件和目录的权限
        for entry in WalkDir::new(mysql_dir) {
            let entry =
                entry.map_err(|e| DockerServiceError::FileSystem(format!("访问目录失败: {e}")))?;
            let path = entry.path();

            if path.is_dir() {
                // 目录设置为777（drwxrwxrwx）
                self.set_directory_permission(path, 0o777)?;
            } else {
                // 文件设置为666（-rw-rw-rw-）
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;

                    let metadata = fs::metadata(path).map_err(|e| {
                        DockerServiceError::FileSystem(format!("获取文件元数据失败: {e}"))
                    })?;
                    let mut permissions = metadata.permissions();
                    permissions.set_mode(0o666);
                    fs::set_permissions(path, permissions).map_err(|e| {
                        DockerServiceError::FileSystem(format!("设置文件权限失败: {e}"))
                    })?;
                }

                #[cfg(windows)]
                {
                    // Windows上跳过文件权限设置
                    tracing::debug!("Windows系统跳过文件权限设置: {}", path.display());
                }
            }
        }

        info!("✅ 现有数据权限修复完成，用户数据已保护");
        Ok(())
    }

    /// 确保MySQL相关目录存在
    fn ensure_mysql_directories(
        &self,
        mysql_data_dir: &Path,
        mysql_logs_dir: &Path,
    ) -> DockerServiceResult<()> {
        if !mysql_data_dir.exists() {
            fs::create_dir_all(mysql_data_dir).map_err(|e| {
                DockerServiceError::FileSystem(format!("创建MySQL数据目录失败: {e}"))
            })?;
            info!("✅ 已创建MySQL数据目录");
        }

        if !mysql_logs_dir.exists() {
            fs::create_dir_all(mysql_logs_dir).map_err(|e| {
                DockerServiceError::FileSystem(format!("创建MySQL日志目录失败: {e}"))
            })?;
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
