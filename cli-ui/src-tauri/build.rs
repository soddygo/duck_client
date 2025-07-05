use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    
    // 检查duck-cli二进制文件是否存在
    check_duck_cli_binary();
    
    // 运行Tauri的构建过程
    tauri_build::build()
}

fn check_duck_cli_binary() {
    // 根据目标平台确定二进制文件路径
    let (platform_dir, binary_name) = get_platform_binary_info();
    let binary_path = Path::new("binaries").join(platform_dir).join(binary_name);
    
    if !binary_path.exists() {
        println!("cargo:warning=Duck CLI binary not found at {}", binary_path.display());
        println!("cargo:warning=Please run './download-duck-cli.sh' to download the binary");
        println!("cargo:warning=Or download manually from: https://github.com/soddygo/duck_client/releases/latest");
        
        // 检查是否有任何平台的二进制文件
        check_all_platforms();
    } else {
        println!("cargo:warning=Duck CLI binary found: {}", binary_path.display());
    }
}

fn get_platform_binary_info() -> (&'static str, &'static str) {
    if cfg!(all(target_os = "macos")) {
        ("macos-universal", "duck-cli")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        ("linux-amd64", "duck-cli")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        ("linux-arm64", "duck-cli")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        ("windows-amd64", "duck-cli.exe")
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        ("windows-arm64", "duck-cli.exe")
    } else {
        // 默认使用 macos-universal 作为后备
        ("macos-universal", "duck-cli")
    }
}

fn check_all_platforms() {
    let platforms = [
        ("macos-universal", "duck-cli"),
        ("linux-amd64", "duck-cli"),
        ("linux-arm64", "duck-cli"),
        ("windows-amd64", "duck-cli.exe"),
        ("windows-arm64", "duck-cli.exe"),
    ];
    
    let mut found_any = false;
    println!("cargo:warning=Available platforms:");
    
    for (platform, binary) in &platforms {
        let path = Path::new("binaries").join(platform).join(binary);
        if path.exists() {
            println!("cargo:warning=  ✓ {} ({})", platform, path.display());
            found_any = true;
        } else {
            println!("cargo:warning=  ✗ {} ({})", platform, path.display());
        }
    }
    
    if !found_any {
        println!("cargo:warning=No duck-cli binaries found for any platform!");
        println!("cargo:warning=Run './download-duck-cli.sh' to download all platforms");
    }
} 