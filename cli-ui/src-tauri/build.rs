use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // 检查并自动构建duck-cli二进制文件
    ensure_duck_cli_binary();

    // 运行Tauri的构建过程
    tauri_build::build()
}

fn ensure_duck_cli_binary() {
    // 获取当前目标三元组
    let target_triple = get_target_triple();
    let binary_name = get_binary_name(&target_triple);
    let binary_path = Path::new("binaries").join(&binary_name);

    if binary_path.exists() {
        println!(
            "cargo:warning=Duck CLI binary found: {}",
            binary_path.display()
        );
        return;
    }

    println!(
        "cargo:warning=Duck CLI binary not found: {}",
        binary_path.display()
    );
    println!(
        "cargo:warning=Auto-building duck-cli for target: {}",
        target_triple
    );

    // 自动构建 duck-cli
    if build_duck_cli(&target_triple) {
        // 复制构建好的二进制文件到正确位置
        copy_binary_to_sidecar_location(&target_triple, &binary_name);
    } else {
        println!("cargo:warning=Failed to build duck-cli automatically");
        println!("cargo:warning=Please build manually: cargo build --release -p duck-cli");
    }
}

fn get_target_triple() -> String {
    // 获取目标三元组，优先使用环境变量，回退到主机三元组
    std::env::var("CARGO_CFG_TARGET_TRIPLE")
        .or_else(|_| std::env::var("TARGET"))
        .unwrap_or_else(|_| {
            // 如果环境变量不可用，使用编译时检测
            if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
                "aarch64-apple-darwin".to_string()
            } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
                "x86_64-apple-darwin".to_string()
            } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
                "x86_64-unknown-linux-gnu".to_string()
            } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
                "aarch64-unknown-linux-gnu".to_string()
            } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
                "x86_64-pc-windows-msvc".to_string()
            } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
                "aarch64-pc-windows-msvc".to_string()
            } else {
                "aarch64-apple-darwin".to_string() // 默认回退
            }
        })
}

fn get_binary_name(target_triple: &str) -> String {
    if target_triple.contains("windows") {
        format!("duck-cli-{}.exe", target_triple)
    } else {
        format!("duck-cli-{}", target_triple)
    }
}

fn build_duck_cli(target_triple: &str) -> bool {
    println!(
        "cargo:warning=Building duck-cli with: cargo build --release -p duck-cli --target {}",
        target_triple
    );

    let mut cmd = Command::new("cargo");
    cmd.args(&[
        "build",
        "--release",
        "-p",
        "duck-cli",
        "--target",
        target_triple,
    ]);

    // 确保在工作区根目录执行
    if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let workspace_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
        cmd.current_dir(workspace_root);
    }

    match cmd.status() {
        Ok(status) if status.success() => {
            println!("cargo:warning=Successfully built duck-cli");
            true
        }
        Ok(status) => {
            println!(
                "cargo:warning=Failed to build duck-cli (exit code: {})",
                status.code().unwrap_or(-1)
            );
            false
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute cargo build: {}", e);
            false
        }
    }
}

fn copy_binary_to_sidecar_location(target_triple: &str, binary_name: &str) {
    let workspace_root = match std::env::var_os("CARGO_MANIFEST_DIR") {
        Some(manifest_dir) => {
            let path = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
            path.to_path_buf()
        }
        None => PathBuf::from("../.."),
    };

    let source_binary_name = if target_triple.contains("windows") {
        "duck-cli.exe"
    } else {
        "duck-cli"
    };

    let source_path = workspace_root
        .join("target")
        .join(target_triple)
        .join("release")
        .join(source_binary_name);

    let dest_dir = Path::new("binaries");
    let dest_path = dest_dir.join(binary_name);

    // 确保目标目录存在
    if let Err(e) = std::fs::create_dir_all(dest_dir) {
        println!("cargo:warning=Failed to create binaries directory: {}", e);
        return;
    }

    // 复制文件
    match std::fs::copy(&source_path, &dest_path) {
        Ok(_) => {
            println!(
                "cargo:warning=Copied {} to {}",
                source_path.display(),
                dest_path.display()
            );
        }
        Err(e) => {
            println!("cargo:warning=Failed to copy binary: {}", e);
            println!("cargo:warning=Source: {}", source_path.display());
            println!("cargo:warning=Dest: {}", dest_path.display());
        }
    }
}
