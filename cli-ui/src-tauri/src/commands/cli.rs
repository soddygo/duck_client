use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Emitter};
use tauri_plugin_shell::{process::CommandEvent, ShellExt};

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
        .map_err(|e| format!("创建sidecar命令失败: {}", e))?;

    if !args.is_empty() {
        cmd = cmd.args(&args);
    }

    if let Some(dir) = working_dir {
        cmd = cmd.current_dir(dir);
    }

    // 设置GUI模式环境变量，禁用duck-cli的tracing日志输出
    cmd = cmd.env("DUCK_GUI_MODE", "1");

    let (mut rx, mut _child) = cmd.spawn().map_err(|e| format!("执行命令失败: {}", e))?;

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

    // 设置GUI模式环境变量，禁用duck-cli的tracing日志输出
    cmd = cmd.env("DUCK_GUI_MODE", "1");

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("执行系统命令失败: {}", e))?;

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
        Ok(result) => Ok(result),
        Err(sidecar_error) => {
            println!("Sidecar执行失败，尝试系统命令: {}", sidecar_error);

            // 降级到系统命令
            match execute_duck_cli_system(app.clone(), args, working_dir).await {
                Ok(result) => Ok(result),
                Err(system_error) => Err(format!(
                    "所有CLI执行方式都失败 - Sidecar: {} | System: {}",
                    sidecar_error, system_error
                )),
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
            println!("获取CLI版本失败: {}", error);
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
