use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use tracing::{info, debug};

#[derive(Clone, Debug)]
pub struct UserMapping {
    pub uid: u32,
    pub gid: u32,
}

impl UserMapping {
    /// 检测当前用户的UID和GID
    pub fn detect_current_user() -> Result<Self, String> {
        #[cfg(unix)]
        {
            use nix::unistd::{getuid, getgid};
            let uid = getuid().as_raw();
            let gid = getgid().as_raw();
            info!("检测到Unix系统用户 - UID: {}, GID: {}", uid, gid);
            Ok(UserMapping { uid, gid })
        }

        #[cfg(windows)]
        {
            // Windows系统使用默认值
            let uid = 1000u32;
            let gid = 1000u32;
            info!("检测到Windows系统 - 使用默认Docker用户 UID: {}, GID: {}", uid, gid);
            Ok(UserMapping { uid, gid })
        }
    }

    /// 设置环境变量
    pub fn set_environment_variables(&self) {
        unsafe {
            std::env::set_var("UID", self.uid.to_string());
            std::env::set_var("GID", self.gid.to_string());
        }
        debug!("设置环境变量: UID={}, GID={}", self.uid, self.gid);
    }

    /// 更新或创建.env文件
    pub fn update_env_file(&self, docker_dir: &Path) -> Result<(), String> {
        let env_file = docker_dir.join(".env");
        
        // 读取现有的.env文件内容（如果存在）
        let mut lines = Vec::new();
        let mut uid_found = false;
        let mut gid_found = false;

        if env_file.exists() {
            let file = std::fs::File::open(&env_file)
                .map_err(|e| format!("无法打开.env文件: {e}"))?;
            
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line.map_err(|e| format!("读取.env文件失败: {e}"))?;
                
                if line.starts_with("UID=") {
                    lines.push(format!("UID={}", self.uid));
                    uid_found = true;
                } else if line.starts_with("GID=") {
                    lines.push(format!("GID={}", self.gid));
                    gid_found = true;
                } else {
                    lines.push(line);
                }
            }
        }

        // 如果没有找到UID或GID，添加它们
        if !uid_found {
            lines.push(format!("UID={}", self.uid));
        }
        if !gid_found {
            lines.push(format!("GID={}", self.gid));
        }

        // 写入.env文件
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&env_file)
            .map_err(|e| format!("无法创建.env文件: {e}"))?;

        for line in lines {
            writeln!(file, "{line}")
                .map_err(|e| format!("写入.env文件失败: {e}"))?;
        }

        info!("已更新.env文件: UID={}, GID={}", self.uid, self.gid);
        Ok(())
    }

    /// 检查是否需要用户映射
    pub fn should_apply_user_mapping() -> bool {
        #[cfg(unix)]
        {
            true
        }

        #[cfg(windows)]
        {
            env::var("DOCKER_HOST").map(|host| host.contains("unix://")).unwrap_or(false)
        }
    }

    /// 获取用户映射字符串
    pub fn get_user_mapping_string(&self) -> String {
        format!("{}:{}", self.uid, self.gid)
    }
}

/// 用户映射管理器
pub struct UserMappingManager {
    work_dir: PathBuf,
    user_mapping: Option<UserMapping>,
}

impl UserMappingManager {
    pub fn new(work_dir: PathBuf) -> Self {
        Self {
            work_dir,
            user_mapping: None,
        }
    }

    /// 初始化用户映射
    pub fn initialize(&mut self) -> Result<(), String> {
        if UserMapping::should_apply_user_mapping() {
            let mapping = UserMapping::detect_current_user()?;
            info!("初始化用户映射: UID={}, GID={}", mapping.uid, mapping.gid);
            self.user_mapping = Some(mapping);
            Ok(())
        } else {
            info!("当前平台不需要用户映射");
            Ok(())
        }
    }

    /// 应用用户映射设置
    pub fn apply_user_mapping(&self) -> Result<(), String> {
        if let Some(mapping) = &self.user_mapping {
            mapping.set_environment_variables();
            mapping.update_env_file(&self.work_dir)?;
            info!("已应用用户映射设置: {}", mapping.get_user_mapping_string());
        } else {
            debug!("无需应用用户映射");
        }
        Ok(())
    }

    /// 显示用户映射信息
    pub fn show_mapping_info(&self) {
        if let Some(mapping) = &self.user_mapping {
            info!("当前用户映射: UID={}, GID={}", mapping.uid, mapping.gid);
            info!("Docker Compose将使用: user: \"{}:{}\"", mapping.uid, mapping.gid);
        } else {
            info!("当前平台无需用户映射");
        }
    }
} 