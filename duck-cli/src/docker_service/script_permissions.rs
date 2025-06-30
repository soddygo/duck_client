use crate::docker_service::error::{DockerServiceError, DockerServiceResult};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, error, info, warn};

/// 脚本权限管理器
pub struct ScriptPermissionManager {
    work_dir: PathBuf,
}

impl ScriptPermissionManager {
    /// 创建新的脚本权限管理器
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// 检查并修复 Docker Compose 相关脚本权限
    pub async fn check_and_fix_script_permissions(&self) -> DockerServiceResult<()> {
        info!("🔍 检查Docker相关脚本权限...");

        // 检测运行环境
        let is_windows = cfg!(target_os = "windows");
        if is_windows {
            info!("🪟 检测到Windows环境，将进行跨平台兼容性检查");

            // 执行Windows兼容性检查
            if let Ok(suggestions) = self.windows_compatibility_check().await {
                if !suggestions.is_empty() {
                    warn!("🪟 Windows环境建议:");
                    for suggestion in suggestions {
                        warn!("  • {}", suggestion);
                    }
                }
            }
        }

        let script_paths = self.find_docker_scripts()?;

        if script_paths.is_empty() {
            debug!("未找到需要检查权限的脚本文件");
            return Ok(());
        }

        info!("找到 {} 个脚本文件需要检查权限", script_paths.len());

        let mut fixed_count = 0;
        let mut converted_count = 0;
        let mut error_count = 0;

        for script_path in script_paths {
            // Windows环境下，先检查并修复行尾符
            if is_windows {
                match self.fix_line_endings(&script_path).await {
                    Ok(was_converted) => {
                        if was_converted {
                            converted_count += 1;
                            info!("🔄 已转换行尾符: {}", script_path.display());
                        }
                    }
                    Err(e) => {
                        warn!("⚠️  行尾符转换失败 {}: {}", script_path.display(), e);
                    }
                }
            }

            // 检查和修复权限
            match self.check_and_fix_file_permission(&script_path).await {
                Ok(was_fixed) => {
                    if was_fixed {
                        fixed_count += 1;
                        info!("✅ 已修复脚本权限: {}", script_path.display());
                    } else {
                        debug!("✓ 脚本权限正常: {}", script_path.display());
                    }
                }
                Err(e) => {
                    error_count += 1;
                    error!("❌ 修复脚本权限失败 {}: {}", script_path.display(), e);

                    // Windows环境提供额外建议
                    if is_windows {
                        warn!("💡 Windows环境建议:");
                        warn!("  - 确保Docker Desktop正在运行");
                        warn!("  - 尝试以管理员身份运行命令");
                        warn!("  - 检查文件是否被其他程序占用");
                    }
                }
            }
        }

        // 汇总结果
        if converted_count > 0 {
            info!("🔄 已转换 {} 个脚本的行尾符格式", converted_count);
        }

        if fixed_count > 0 {
            info!("🛠️  已修复 {} 个脚本的执行权限", fixed_count);
        }

        if error_count > 0 {
            warn!("⚠️  {} 个脚本处理失败，可能需要手动处理", error_count);
            if is_windows {
                warn!("🪟 Windows用户可以尝试:");
                warn!("  1. 在Git Bash中运行: chmod +x config/docker-entrypoint.sh");
                warn!("  2. 或在WSL中运行: chmod +x config/docker-entrypoint.sh");
                warn!("  3. 确保Docker设置中启用了文件共享");
            }
        } else {
            info!("✅ 脚本权限检查完成");
        }

        Ok(())
    }

    /// 查找Docker相关的脚本文件
    fn find_docker_scripts(&self) -> DockerServiceResult<Vec<PathBuf>> {
        let mut script_paths = Vec::new();

        // 常见的Docker脚本路径模式
        let script_patterns = vec![
            "config/docker-entrypoint.sh",
            "config/entrypoint.sh",
            "config/init.sh",
            "config/startup.sh",
            "config/video_analysis/entrypoint-master.sh",
            "config/video_analysis/entrypoint-worker.sh",
            "script/init-minio.sh",
            "scripts/init-minio.sh",
        ];

        for pattern in script_patterns {
            let script_path = self.work_dir.join(pattern);
            if script_path.exists() {
                script_paths.push(script_path);
            }
        }

        // 递归查找 config 目录下的所有 .sh 文件
        if let Ok(config_dir) = self.work_dir.join("config").canonicalize() {
            if config_dir.exists() {
                self.find_shell_scripts_recursive(&config_dir, &mut script_paths)?;
            }
        }

        // 递归查找 script/scripts 目录下的所有 .sh 文件
        for script_dir_name in ["script", "scripts"] {
            if let Ok(script_dir) = self.work_dir.join(script_dir_name).canonicalize() {
                if script_dir.exists() {
                    self.find_shell_scripts_recursive(&script_dir, &mut script_paths)?;
                }
            }
        }

        // 去重
        script_paths.sort();
        script_paths.dedup();

        Ok(script_paths)
    }

    /// 递归查找shell脚本文件
    fn find_shell_scripts_recursive(
        &self,
        dir: &Path,
        script_paths: &mut Vec<PathBuf>,
    ) -> DockerServiceResult<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir).map_err(|e| {
            DockerServiceError::FileSystem(format!("读取目录失败 {}: {}", dir.display(), e))
        })?;

        for entry in entries {
            let entry = entry
                .map_err(|e| DockerServiceError::FileSystem(format!("读取目录项失败: {e}")))?;
            let path = entry.path();

            if path.is_dir() {
                // 递归搜索子目录
                self.find_shell_scripts_recursive(&path, script_paths)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("sh") {
                script_paths.push(path);
            }
        }

        Ok(())
    }

    /// 检查并修复单个文件权限
    async fn check_and_fix_file_permission(&self, script_path: &Path) -> DockerServiceResult<bool> {
        // 检查文件是否存在
        if !script_path.exists() {
            return Err(DockerServiceError::FileSystem(format!(
                "脚本文件不存在: {}",
                script_path.display()
            )));
        }

        // 检查当前权限
        let metadata = std::fs::metadata(script_path).map_err(|e| {
            DockerServiceError::FileSystem(format!(
                "获取文件元数据失败 {}: {}",
                script_path.display(),
                e
            ))
        })?;

        if cfg!(unix) {
            // Unix/Linux/macOS 系统权限检查
            self.check_unix_permissions(script_path, &metadata).await
        } else if cfg!(windows) {
            // Windows 系统权限检查
            self.check_windows_permissions(script_path, &metadata).await
        } else {
            debug!("未知操作系统，跳过权限检查: {}", script_path.display());
            Ok(false)
        }
    }

    /// Unix系统权限检查
    #[cfg(unix)]
    async fn check_unix_permissions(
        &self,
        script_path: &Path,
        metadata: &std::fs::Metadata,
    ) -> DockerServiceResult<bool> {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        let is_executable = (mode & 0o111) != 0; // 检查是否有执行权限

        if is_executable {
            debug!("脚本已有执行权限: {}", script_path.display());
            return Ok(false);
        }

        // 添加执行权限
        info!("正在为脚本添加执行权限: {}", script_path.display());
        self.add_execute_permission(script_path).await?;
        Ok(true)
    }

    /// Windows系统权限检查
    #[cfg(not(unix))]
    async fn check_unix_permissions(
        &self,
        _script_path: &Path,
        _metadata: &std::fs::Metadata,
    ) -> DockerServiceResult<bool> {
        Ok(false)
    }

    /// Windows系统权限检查和修复
    async fn check_windows_permissions(
        &self,
        script_path: &Path,
        _metadata: &std::fs::Metadata,
    ) -> DockerServiceResult<bool> {
        info!("🪟 Windows环境下检查脚本权限: {}", script_path.display());

        // Windows下，我们假设脚本可能需要设置执行权限
        // 因为Windows文件系统挂载到Docker容器时可能丢失执行权限

        // 检查是否已经有执行权限（通过尝试chmod来验证）
        if self.verify_windows_execute_permission(script_path).await? {
            debug!("脚本在容器中应该有执行权限: {}", script_path.display());
            return Ok(false);
        }

        // 尝试设置执行权限
        info!("正在为脚本添加执行权限: {}", script_path.display());
        self.add_execute_permission(script_path).await?;
        Ok(true)
    }

    /// 验证Windows下的脚本执行权限
    async fn verify_windows_execute_permission(
        &self,
        script_path: &Path,
    ) -> DockerServiceResult<bool> {
        // 在Windows下，我们通过尝试chmod来验证权限
        // 如果chmod成功且没有实际改变，说明权限已经正确

        // 方法1: 尝试Git Bash验证
        if let Ok(result) = self.verify_with_git_bash(script_path).await {
            return Ok(result);
        }

        // 方法2: 尝试WSL验证
        if let Ok(result) = self.verify_with_wsl(script_path).await {
            return Ok(result);
        }

        // 默认假设需要设置权限
        debug!("无法验证Windows脚本权限，假设需要设置");
        Ok(false)
    }

    /// 使用Git Bash验证权限
    async fn verify_with_git_bash(&self, script_path: &Path) -> DockerServiceResult<bool> {
        let git_bash_paths = vec![
            "C:\\Program Files\\Git\\bin\\bash.exe",
            "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
            "bash",
        ];

        for bash_path in git_bash_paths {
            if let Ok(output) = Command::new(bash_path)
                .arg("-c")
                .arg(format!("test -x \"{}\"", script_path.display()))
                .output()
            {
                if output.status.success() {
                    debug!("Git Bash 验证: 脚本有执行权限");
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// 使用WSL验证权限
    async fn verify_with_wsl(&self, script_path: &Path) -> DockerServiceResult<bool> {
        let wsl_path = self.convert_to_wsl_path(script_path)?;

        if let Ok(output) = Command::new("wsl")
            .arg("test")
            .arg("-x")
            .arg(&wsl_path)
            .output()
        {
            if output.status.success() {
                debug!("WSL 验证: 脚本有执行权限");
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 为脚本添加执行权限（跨平台）
    async fn add_execute_permission(&self, script_path: &Path) -> DockerServiceResult<()> {
        if cfg!(unix) {
            // Unix/Linux/macOS系统
            self.add_execute_permission_unix(script_path).await
        } else if cfg!(windows) {
            // Windows系统
            self.add_execute_permission_windows(script_path).await
        } else {
            warn!("未知操作系统，跳过权限设置");
            Ok(())
        }
    }

    /// Unix系统下添加执行权限
    #[cfg(unix)]
    async fn add_execute_permission_unix(&self, script_path: &Path) -> DockerServiceResult<()> {
        let output = Command::new("chmod")
            .arg("+x")
            .arg(script_path)
            .output()
            .map_err(|e| DockerServiceError::Permission(format!("执行chmod命令失败: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerServiceError::Permission(format!(
                "chmod命令执行失败: {stderr}"
            )));
        }

        info!("✅ 已添加执行权限: {}", script_path.display());
        Ok(())
    }

    #[cfg(not(unix))]
    async fn add_execute_permission_unix(&self, _script_path: &Path) -> DockerServiceResult<()> {
        Ok(())
    }

    /// Windows系统下添加执行权限
    async fn add_execute_permission_windows(&self, script_path: &Path) -> DockerServiceResult<()> {
        info!("🪟 Windows环境下设置脚本权限: {}", script_path.display());

        // 方法1: 尝试使用Git Bash的chmod
        if let Ok(result) = self.try_git_bash_chmod(script_path).await {
            if result {
                info!("✅ 通过Git Bash设置权限成功");
                return Ok(());
            }
        }

        // 方法2: 尝试使用WSL的chmod
        if let Ok(result) = self.try_wsl_chmod(script_path).await {
            if result {
                info!("✅ 通过WSL设置权限成功");
                return Ok(());
            }
        }

        // 方法3: 尝试直接chmod（如果可用）
        if let Ok(result) = self.try_direct_chmod(script_path).await {
            if result {
                info!("✅ 直接chmod设置权限成功");
                return Ok(());
            }
        }

        // 所有方法都失败，提供手动操作指导
        warn!("⚠️  自动设置权限失败，请手动操作:");
        warn!("🪟 Windows用户请尝试以下任一方法:");
        warn!("  1. 在Git Bash中运行:");
        warn!("     chmod +x {}", script_path.display());
        warn!("  2. 在WSL中运行:");
        warn!("     chmod +x {}", script_path.display());
        warn!("  3. 在PowerShell中运行:");
        warn!("     bash -c \"chmod +x {}\"", script_path.display());

        // 不返回错误，让程序继续运行，用户可以手动修复
        Ok(())
    }

    /// 尝试使用Git Bash的chmod
    async fn try_git_bash_chmod(&self, script_path: &Path) -> DockerServiceResult<bool> {
        // 查找Git Bash路径
        let git_bash_paths = vec![
            "C:\\Program Files\\Git\\bin\\bash.exe",
            "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
            "bash", // 如果在PATH中
        ];

        for bash_path in git_bash_paths {
            if let Ok(output) = Command::new(bash_path)
                .arg("-c")
                .arg(format!("chmod +x \"{}\"", script_path.display()))
                .output()
            {
                if output.status.success() {
                    debug!("Git Bash chmod 成功: {}", bash_path);
                    return Ok(true);
                }
            }
        }

        debug!("Git Bash chmod 不可用");
        Ok(false)
    }

    /// 尝试使用WSL的chmod
    async fn try_wsl_chmod(&self, script_path: &Path) -> DockerServiceResult<bool> {
        // 转换Windows路径为WSL路径
        let wsl_path = self.convert_to_wsl_path(script_path)?;

        if let Ok(output) = Command::new("wsl")
            .arg("chmod")
            .arg("+x")
            .arg(&wsl_path)
            .output()
        {
            if output.status.success() {
                debug!("WSL chmod 成功");
                return Ok(true);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                debug!("WSL chmod 失败: {}", stderr);
            }
        }

        debug!("WSL chmod 不可用");
        Ok(false)
    }

    /// 尝试直接chmod
    async fn try_direct_chmod(&self, script_path: &Path) -> DockerServiceResult<bool> {
        if let Ok(output) = Command::new("chmod").arg("+x").arg(script_path).output() {
            if output.status.success() {
                debug!("直接 chmod 成功");
                return Ok(true);
            }
        }

        debug!("直接 chmod 不可用");
        Ok(false)
    }

    /// 转换Windows路径为WSL路径
    fn convert_to_wsl_path(&self, windows_path: &Path) -> DockerServiceResult<String> {
        let path_str = windows_path.to_string_lossy();

        // 简单的路径转换逻辑
        if path_str.starts_with("C:") {
            let wsl_path = path_str.replace("C:", "/mnt/c").replace("\\", "/");
            Ok(wsl_path)
        } else if path_str.starts_with("D:") {
            let wsl_path = path_str.replace("D:", "/mnt/d").replace("\\", "/");
            Ok(wsl_path)
        } else {
            // 相对路径，直接使用
            Ok(path_str.replace("\\", "/"))
        }
    }

    /// 手动修复特定脚本权限
    pub async fn fix_specific_script(&self, script_name: &str) -> DockerServiceResult<()> {
        let script_path = self.work_dir.join("config").join(script_name);

        if !script_path.exists() {
            return Err(DockerServiceError::FileSystem(format!(
                "脚本文件不存在: {}",
                script_path.display()
            )));
        }

        info!("🛠️  修复特定脚本权限: {}", script_name);
        self.check_and_fix_file_permission(&script_path).await?;
        Ok(())
    }

    /// 预检查常见问题脚本
    pub async fn precheck_common_script_issues(&self) -> DockerServiceResult<Vec<String>> {
        let mut issues = Vec::new();

        // 检查docker-entrypoint.sh权限
        let entrypoint_script = self.work_dir.join("config/docker-entrypoint.sh");
        if entrypoint_script.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(&entrypoint_script) {
                    let mode = metadata.permissions().mode();
                    if (mode & 0o111) == 0 {
                        issues.push(format!("脚本缺少执行权限: {}", entrypoint_script.display()));
                    }
                }
            }
        }

        // 检查其他常见脚本
        let common_scripts = vec![
            "config/video_analysis/entrypoint-master.sh",
            "config/video_analysis/entrypoint-worker.sh",
            "script/init-minio.sh",
        ];

        for script_name in common_scripts {
            let script_path = self.work_dir.join(script_name);
            if script_path.exists() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = std::fs::metadata(&script_path) {
                        let mode = metadata.permissions().mode();
                        if (mode & 0o111) == 0 {
                            issues.push(format!("脚本缺少执行权限: {}", script_path.display()));
                        }
                    }
                }
            }
        }

        Ok(issues)
    }

    /// 修复Windows行尾符问题（CRLF -> LF）
    async fn fix_line_endings(&self, script_path: &Path) -> DockerServiceResult<bool> {
        if !script_path.exists() {
            return Ok(false);
        }

        // 读取文件内容
        let content = std::fs::read_to_string(script_path).map_err(|e| {
            DockerServiceError::FileSystem(format!(
                "读取脚本文件失败 {}: {}",
                script_path.display(),
                e
            ))
        })?;

        // 检查是否包含Windows行尾符
        if !content.contains("\r\n") {
            debug!("脚本已是Unix行尾符格式: {}", script_path.display());
            return Ok(false);
        }

        info!("发现Windows行尾符，正在转换: {}", script_path.display());

        // 转换行尾符: CRLF -> LF
        let unix_content = content.replace("\r\n", "\n");

        // 创建备份文件
        let backup_path = script_path.with_extension("sh.bak");
        std::fs::copy(script_path, &backup_path).map_err(|e| {
            DockerServiceError::FileSystem(format!(
                "创建备份文件失败 {}: {}",
                backup_path.display(),
                e
            ))
        })?;

        debug!("已创建备份文件: {}", backup_path.display());

        // 写入转换后的内容
        std::fs::write(script_path, unix_content).map_err(|e| {
            DockerServiceError::FileSystem(format!(
                "写入转换后的脚本失败 {}: {}",
                script_path.display(),
                e
            ))
        })?;

        info!("✅ 行尾符转换完成: {}", script_path.display());
        info!("💾 备份文件: {}", backup_path.display());

        Ok(true)
    }

    /// 检查脚本编码问题
    pub async fn check_script_encoding(&self, script_path: &Path) -> DockerServiceResult<bool> {
        if !script_path.exists() {
            return Ok(false);
        }

        // 尝试以UTF-8读取文件
        match std::fs::read_to_string(script_path) {
            Ok(content) => {
                // 检查是否包含BOM
                if content.starts_with('\u{FEFF}') {
                    warn!("脚本包含BOM标记: {}", script_path.display());
                    warn!("建议: 使用文本编辑器去除BOM标记");
                    return Ok(false);
                }

                // 检查是否包含Windows行尾符
                if content.contains("\r\n") {
                    warn!("脚本使用Windows行尾符: {}", script_path.display());
                    return Ok(false);
                }

                debug!("脚本编码检查通过: {}", script_path.display());
                Ok(true)
            }
            Err(e) => {
                warn!("脚本编码检查失败 {}: {}", script_path.display(), e);
                warn!("可能不是有效的UTF-8编码");
                Ok(false)
            }
        }
    }

    /// Windows环境下的额外检查和建议
    pub async fn windows_compatibility_check(&self) -> DockerServiceResult<Vec<String>> {
        let mut suggestions = Vec::new();

        if !cfg!(target_os = "windows") {
            return Ok(suggestions);
        }

        info!("🪟 执行Windows兼容性检查...");

        // 检查Docker Desktop是否运行
        if let Err(_) = Command::new("docker").arg("version").output() {
            suggestions.push("Docker Desktop可能未运行，请启动Docker Desktop".to_string());
        }

        // 检查是否有WSL2
        if let Ok(output) = Command::new("wsl").arg("--list").arg("--verbose").output() {
            let wsl_output = String::from_utf8_lossy(&output.stdout);
            if wsl_output.contains("Version 2") {
                suggestions
                    .push("建议在WSL2环境中运行Docker相关操作以获得更好的兼容性".to_string());
            }
        }

        // 检查Git配置
        if let Ok(output) = Command::new("git")
            .arg("config")
            .arg("core.autocrlf")
            .output()
        {
            let git_config = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if git_config == "true" {
                suggestions.push(
                    "Git配置 core.autocrlf=true 可能导致脚本行尾符问题，建议设置为false"
                        .to_string(),
                );
            }
        }

        if suggestions.is_empty() {
            info!("✅ Windows兼容性检查通过");
        } else {
            warn!("⚠️  发现 {} 个Windows兼容性问题", suggestions.len());
        }

        Ok(suggestions)
    }
}
