use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Emitter};
use tauri_plugin_shell::{process::CommandEvent, ShellExt};
use std::process::{Command as StdCommand, Stdio};

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CliVersion {
    pub version: String,
    pub available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub command: String,
    pub running: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessCheckResult {
    pub processes_found: Vec<ProcessInfo>,
    pub processes_killed: Vec<u32>,
    pub success: bool,
    pub message: String,
}

/// 执行duck-cli命令（Sidecar方式）
#[command]
pub async fn execute_duck_cli_sidecar(
    app: AppHandle,
    args: Vec<String>,
    working_dir: Option<String>,
) -> Result<CommandResult, String> {
    let shell = app.shell();

    let mut cmd = shell
        .sidecar("duck-cli")
        .map_err(|e| format!("创建sidecar命令失败: {e}"))?;

    if !args.is_empty() {
        cmd = cmd.args(&args);
    }

    if let Some(dir) = working_dir {
        cmd = cmd.current_dir(dir);
    }

    let (mut rx, mut _child) = cmd.spawn().map_err(|e| format!("执行命令失败: {e}"))?;

    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut exit_code = 0;

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(data) => {
                let output = String::from_utf8_lossy(&data);
                stdout.push_str(&output);
                // 实时发送输出到前端
                let _ = app.emit("cli-output", &output);
            }
            CommandEvent::Stderr(data) => {
                let output = String::from_utf8_lossy(&data);
                stderr.push_str(&output);
                // 实时发送错误到前端
                let _ = app.emit("cli-error", &output);
            }
            CommandEvent::Terminated(payload) => {
                exit_code = payload.code.unwrap_or(-1);
                let _ = app.emit("cli-complete", exit_code);
                break;
            }
            _ => {}
        }
    }

    Ok(CommandResult {
        success: exit_code == 0,
        exit_code,
        stdout,
        stderr,
    })
}

/// 执行系统duck-cli命令（Shell方式）
#[command]
pub async fn execute_duck_cli_system(
    app: AppHandle,
    args: Vec<String>,
    working_dir: Option<String>,
) -> Result<CommandResult, String> {
    let shell = app.shell();

    let mut cmd = shell.command("duck-cli");

    if !args.is_empty() {
        cmd = cmd.args(&args);
    }

    if let Some(dir) = working_dir {
        cmd = cmd.current_dir(dir);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("执行系统命令失败: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    // 发送输出到前端
    if !stdout.is_empty() {
        let _ = app.emit("cli-output", &stdout);
    }
    if !stderr.is_empty() {
        let _ = app.emit("cli-error", &stderr);
    }
    let _ = app.emit("cli-complete", exit_code);

    Ok(CommandResult {
        success: exit_code == 0,
        exit_code,
        stdout,
        stderr,
    })
}

/// 智能执行duck-cli命令（混合策略）
#[command]
pub async fn execute_duck_cli_smart(
    app: AppHandle,
    args: Vec<String>,
    working_dir: Option<String>,
) -> Result<CommandResult, String> {
    // 优先使用Sidecar方式
    match execute_duck_cli_sidecar(app.clone(), args.clone(), working_dir.clone()).await {
        Ok(result) => {
            // Sidecar成功，直接返回结果（已发送事件）
            Ok(result)
        },
        Err(sidecar_error) => {
            println!("Sidecar执行失败，尝试系统命令: {sidecar_error}");

            // 发送降级通知
            let _ = app.emit("cli-output", "⚠️ Sidecar方式失败，使用系统命令...");

            // 降级到系统命令
            match execute_duck_cli_system(app.clone(), args, working_dir).await {
                Ok(result) => {
                    // System成功，返回结果（已发送事件）
                    Ok(result)
                },
                Err(system_error) => {
                    // 发送失败通知
                    let _ = app.emit("cli-error", "❌ 所有CLI执行方式都失败");
                    let _ = app.emit("cli-complete", -1);
                    
                    Err(format!(
                        "所有CLI执行方式都失败 - Sidecar: {sidecar_error} | System: {system_error}"
                    ))
                }
            }
        }
    }
}

/// 检查CLI工具版本
#[command]
pub async fn get_cli_version(app: AppHandle) -> Result<CliVersion, String> {
    match execute_duck_cli_smart(app, vec!["--version".to_string()], None).await {
        Ok(result) => {
            if result.success {
                // 从输出中提取版本号
                let version = result
                    .stdout
                    .lines()
                    .find(|line| line.contains("duck-cli"))
                    .and_then(|line| line.split_whitespace().last())
                    .unwrap_or("unknown")
                    .to_string();

                Ok(CliVersion {
                    version,
                    available: true,
                })
            } else {
                Ok(CliVersion {
                    version: "error".to_string(),
                    available: false,
                })
            }
        }
        Err(error) => {
            println!("获取CLI版本失败: {error}");
            Ok(CliVersion {
                version: "unavailable".to_string(),
                available: false,
            })
        }
    }
}

/// 检查CLI工具是否可用
#[command]
pub async fn check_cli_available(app: AppHandle) -> Result<bool, String> {
    let version_info = get_cli_version(app).await?;
    Ok(version_info.available)
}

/// 检查并清理运行中的duck-cli进程
#[command]
pub async fn check_and_cleanup_duck_processes() -> Result<ProcessCheckResult, String> {
    let mut processes_found = Vec::new();
    let mut processes_killed = Vec::new();
    
    // 检查运行中的duck-cli进程
    #[cfg(target_os = "macos")]
    let output = StdCommand::new("ps")
        .args(&["aux"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    
    #[cfg(target_os = "linux")]
    let output = StdCommand::new("ps")
        .args(&["aux"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    
    #[cfg(target_os = "windows")]
    let output = StdCommand::new("tasklist")
        .args(&["/FI", "IMAGENAME eq duck-cli.exe", "/FO", "CSV"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // 解析进程信息
            for line in stdout.lines() {
                if line.contains("duck-cli") && !line.contains("grep") {
                    // 提取PID和命令信息
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    
                    #[cfg(any(target_os = "macos", target_os = "linux"))]
                    let (pid_str, command) = if parts.len() >= 11 {
                        (parts[1], parts[10..].join(" "))
                    } else {
                        continue;
                    };
                    
                    #[cfg(target_os = "windows")]
                    let (pid_str, command) = if parts.len() >= 2 {
                        // Windows tasklist CSV格式: "ImageName","PID","SessionName",...
                        let csv_parts: Vec<&str> = line.split(',').collect();
                        if csv_parts.len() >= 2 {
                            let pid = csv_parts[1].trim_matches('"');
                            let cmd = csv_parts[0].trim_matches('"');
                            (pid, cmd.to_string())
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    };
                    
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        processes_found.push(ProcessInfo {
                            pid,
                            command: command.to_string(),
                            running: true,
                        });
                        
                        // 尝试终止进程
                        #[cfg(any(target_os = "macos", target_os = "linux"))]
                        let kill_result = StdCommand::new("kill")
                            .args(&["-TERM", &pid.to_string()])
                            .output();
                        
                        #[cfg(target_os = "windows")]
                        let kill_result = StdCommand::new("taskkill")
                            .args(&["/PID", &pid.to_string(), "/F"])
                            .output();
                        
                        match kill_result {
                            Ok(_) => {
                                processes_killed.push(pid);
                                
                                // 等待一下，然后检查进程是否真的被终止
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                
                                // 验证进程是否被终止
                                #[cfg(any(target_os = "macos", target_os = "linux"))]
                                let check_result = StdCommand::new("kill")
                                    .args(&["-0", &pid.to_string()])
                                    .output();
                                
                                #[cfg(target_os = "windows")]
                                let check_result = StdCommand::new("tasklist")
                                    .args(&["/FI", &format!("PID eq {}", pid)])
                                    .output();
                                
                                // 如果进程仍在运行，尝试强制终止
                                match check_result {
                                    Ok(output) if output.status.success() => {
                                        #[cfg(any(target_os = "macos", target_os = "linux"))]
                                        let _ = StdCommand::new("kill")
                                            .args(&["-KILL", &pid.to_string()])
                                            .output();
                                        
                                        #[cfg(target_os = "windows")]
                                        let _ = StdCommand::new("taskkill")
                                            .args(&["/PID", &pid.to_string(), "/F", "/T"])
                                            .output();
                                    }
                                    _ => {
                                        // 进程已经被终止
                                    }
                                }
                            }
                            Err(e) => {
                                println!("终止进程 {} 失败: {}", pid, e);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            return Err(format!("检查进程失败: {}", e));
        }
    }
    
    let success = !processes_found.is_empty();
    let message = if processes_found.is_empty() {
        "没有发现运行中的 duck-cli 进程".to_string()
    } else {
        format!(
            "发现 {} 个 duck-cli 进程，已终止 {} 个",
            processes_found.len(),
            processes_killed.len()
        )
    };
    
    Ok(ProcessCheckResult {
        processes_found,
        processes_killed,
        success,
        message,
    })
}

/// 检查数据库文件是否被锁定
#[command]
pub async fn check_database_lock(_app: AppHandle, working_dir: String) -> Result<bool, String> {
    use std::path::PathBuf;
    use std::fs::OpenOptions;
    
    let db_path = PathBuf::from(&working_dir).join("data").join("duck_client.db");
    
    if !db_path.exists() {
        return Ok(false); // 文件不存在，没有锁定问题
    }
    
    // 尝试以独占模式打开文件来检测锁定
    match OpenOptions::new()
        .read(true)
        .write(true)
        .open(&db_path)
    {
        Ok(_file) => Ok(false), // 能够打开，说明没有被锁定
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("Resource busy") || 
               error_msg.contains("locked") ||
               error_msg.contains("being used") {
                Ok(true) // 文件被锁定
            } else {
                Err(format!("检查数据库锁定状态失败: {}", error_msg))
            }
        }
    }
}
