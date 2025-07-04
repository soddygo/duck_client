use tauri::command;
use std::path::PathBuf;
use duck_cli::get_system_info;
use super::types::{SystemRequirements, PlatformSpecificChecks, StorageInfo};

/// 检查系统要求
#[command]
pub async fn check_system_requirements(
    directory: Option<String>,
) -> Result<SystemRequirements, String> {
    let system_info = get_system_info();
    
    // 检查存储空间
    let available_space_gb = if let Some(dir) = directory {
        let _path = PathBuf::from(dir);
        system_info.disk_space.available as f64 / (1024.0 * 1024.0 * 1024.0)
    } else {
        system_info.disk_space.available as f64 / (1024.0 * 1024.0 * 1024.0)
    };
    
    let required_space_gb = 60.0; // 60GB最小要求
    
    // 平台特定检查
    let platform_specific = PlatformSpecificChecks {
        docker_desktop_installed: system_info.docker_version.is_some(),
        wsl_enabled: check_wsl_status(),
        homebrew_docker: check_homebrew_docker(),
        docker_group_member: check_docker_group_membership(),
    };
    
    Ok(SystemRequirements {
        os_supported: matches!(system_info.os.as_str(), "windows" | "macos" | "linux"),
        docker_available: system_info.docker_version.is_some(),
        storage_sufficient: available_space_gb >= required_space_gb,
        available_space_gb,
        required_space_gb,
        platform_specific,
    })
}

/// 获取平台信息
#[command]
pub async fn get_platform() -> Result<String, String> {
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    };
    
    Ok(platform.to_string())
}

/// 检查系统存储空间
#[command]
pub async fn check_system_storage() -> Result<StorageInfo, String> {
    let system_info = get_system_info();
    
    let total_bytes = system_info.disk_space.total;
    let available_bytes = system_info.disk_space.available;
    let used_bytes = total_bytes - available_bytes;
    let available_space_gb = available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let required_space_gb = 60.0;
    
    Ok(StorageInfo {
        path: "系统磁盘".to_string(),
        total_bytes,
        available_bytes,
        used_bytes,
        available_space_gb,
        required_space_gb,
        sufficient: available_space_gb >= required_space_gb,
    })
}

/// 检查指定路径的存储空间
#[command]
pub async fn check_storage_space(path: String) -> Result<StorageInfo, String> {
    let path_buf = PathBuf::from(&path);
    
    // 确保路径存在，如果不存在则创建父目录用于检测
    let check_path = if path_buf.exists() {
        path_buf
    } else {
        // 如果路径不存在，使用父目录或根目录进行检测
        path_buf.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| {
                #[cfg(target_os = "windows")]
                { PathBuf::from("C:\\") }
                #[cfg(not(target_os = "windows"))]
                { PathBuf::from("/") }
            })
    };
    
    let required_bytes = 60u64 * 1024 * 1024 * 1024; // 60GB
    
    // 使用跨平台的方法获取磁盘空间
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use winapi::um::fileapi::GetDiskFreeSpaceExW;
        
        let path_wide: Vec<u16> = check_path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mut free_bytes_available = 0u64;
        let mut total_number_of_bytes = 0u64;
        let mut total_number_of_free_bytes = 0u64;
        
        let result = unsafe {
            GetDiskFreeSpaceExW(
                path_wide.as_ptr(),
                &mut free_bytes_available,
                &mut total_number_of_bytes,
                &mut total_number_of_free_bytes,
            )
        };
        
        if result == 0 {
            return Err("无法获取磁盘空间信息".to_string());
        }
        
        let total_bytes = total_number_of_bytes;
        let available_bytes = free_bytes_available;
        let used_bytes = total_bytes - available_bytes;
        let available_space_gb = available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let required_space_gb = required_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let sufficient = available_bytes >= required_bytes;
        
        Ok(StorageInfo {
            path,
            total_bytes,
            available_bytes,
            used_bytes,
            available_space_gb,
            required_space_gb,
            sufficient,
        })
    }
    
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::ffi::CString;
        use libc::statvfs;
        
        let path_c = CString::new(check_path.to_string_lossy().as_bytes())
            .map_err(|e| format!("路径转换失败: {}", e))?;
        
        let mut stat: statvfs = unsafe { std::mem::zeroed() };
        let result = unsafe { statvfs(path_c.as_ptr(), &mut stat) };
        
        if result != 0 {
            return Err("无法获取磁盘空间信息".to_string());
        }
        
        let total_bytes = (stat.f_blocks as u64) * (stat.f_frsize as u64);
        let available_bytes = (stat.f_bavail as u64) * (stat.f_frsize as u64);
        let used_bytes = total_bytes - available_bytes;
        let available_space_gb = available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let required_space_gb = required_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let sufficient = available_bytes >= required_bytes;
        
        Ok(StorageInfo {
            path,
            total_bytes,
            available_bytes,
            used_bytes,
            available_space_gb,
            required_space_gb,
            sufficient,
        })
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        // 对于其他平台，返回错误
        Err("当前平台不支持磁盘空间检测".to_string())
    }
}

/// 打开文件管理器
#[command]
pub async fn open_file_manager(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    
    if !path_buf.exists() {
        return Err("指定的路径不存在".to_string());
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("无法打开文件管理器: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("无法打开文件管理器: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("无法打开文件管理器: {}", e))?;
    }
    
    Ok(())
}

// ================== 平台特定函数 ==================

#[cfg(target_os = "windows")]
fn check_wsl_status() -> bool {
    use std::process::Command;
    Command::new("wsl")
        .arg("--status")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn check_wsl_status() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn check_homebrew_docker() -> bool {
    use std::process::Command;
    Command::new("brew")
        .args(&["list", "docker"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn check_homebrew_docker() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn check_docker_group_membership() -> bool {
    use std::process::Command;
    Command::new("groups")
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .contains("docker")
        })
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn check_docker_group_membership() -> bool {
    false
} 