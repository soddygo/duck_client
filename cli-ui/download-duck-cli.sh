#!/bin/bash

# Duck CLI 多平台自动下载脚本
# 从 GitHub Releases 下载所有平台版本的 duck-cli 二进制文件

set -e

REPO="soddygo/duck_client"
GITHUB_API="https://api.github.com/repos/${REPO}/releases/latest"
DOWNLOAD_BASE="https://github.com/${REPO}/releases/latest/download"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印带颜色的消息
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 定义所有支持的平台（兼容 bash 3.2）
PLATFORMS="macos-universal linux-amd64 linux-arm64 windows-amd64 windows-arm64"

# 获取平台配置信息
get_platform_info() {
    local platform="$1"
    case "$platform" in
        "macos-universal")
            echo "duck-cli-macos-universal.tar.gz|tar.gz|duck-cli"
            ;;
        "linux-amd64")
            echo "duck-cli-linux-amd64.tar.gz|tar.gz|duck-cli"
            ;;
        "linux-arm64")
            echo "duck-cli-linux-arm64.tar.gz|tar.gz|duck-cli"
            ;;
        "windows-amd64")
            echo "duck-cli-windows-amd64.zip|zip|duck-cli.exe"
            ;;
        "windows-arm64")
            echo "duck-cli-windows-arm64.zip|zip|duck-cli.exe"
            ;;
        *)
            echo ""
            ;;
    esac
}

# 创建目录结构
create_directories() {
    local binaries_dir="src-tauri/binaries"
    
    log_info "创建目录结构..."
    
    for platform in $PLATFORMS; do
        local platform_dir="$binaries_dir/$platform"
        if [ ! -d "$platform_dir" ]; then
            mkdir -p "$platform_dir"
            log_info "创建目录: $platform_dir"
        fi
    done
}

# 检查是否已存在二进制文件
check_existing_binaries() {
    local binaries_dir="src-tauri/binaries"
    local existing_count=0
    
    for platform in $PLATFORMS; do
        local platform_info=$(get_platform_info "$platform")
        IFS='|' read -r archive_name file_ext binary_name <<< "$platform_info"
        local binary_path="$binaries_dir/$platform/$binary_name"
        
        if [ -f "$binary_path" ]; then
            existing_count=$((existing_count + 1))
        fi
    done
    
    if [ $existing_count -gt 0 ]; then
        log_warn "发现 $existing_count 个已存在的二进制文件"
        read -p "是否重新下载所有平台? (y/N): " response
        case "$response" in
            [yY][eE][sS]|[yY])
                log_info "清理现有文件..."
                for platform in $PLATFORMS; do
                    local platform_info=$(get_platform_info "$platform")
                    IFS='|' read -r archive_name file_ext binary_name <<< "$platform_info"
                    local binary_path="$binaries_dir/$platform/$binary_name"
                    rm -f "$binary_path"
                done
                ;;
            *)
                log_success "保留现有的二进制文件"
                exit 0
                ;;
        esac
    fi
}

# 获取最新版本信息
get_latest_version() {
    log_info "获取最新版本信息..."
    
    if command -v curl >/dev/null 2>&1; then
        VERSION=$(curl -s "$GITHUB_API" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    elif command -v wget >/dev/null 2>&1; then
        VERSION=$(wget -qO- "$GITHUB_API" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        log_error "需要安装 curl 或 wget"
        exit 1
    fi
    
    if [ -z "$VERSION" ]; then
        log_error "无法获取最新版本信息"
        exit 1
    fi
    
    log_success "最新版本: $VERSION"
}

# 下载单个平台的文件
download_platform() {
    local platform="$1"
    local platform_info=$(get_platform_info "$platform")
    IFS='|' read -r archive_name file_ext binary_name <<< "$platform_info"
    
    local download_url="${DOWNLOAD_BASE}/${archive_name}"
    local temp_file="/tmp/${archive_name}"
    local binaries_dir="src-tauri/binaries"
    
    log_info "下载 $platform..."
    log_info "下载地址: $download_url"
    
    # 下载文件
    if command -v curl >/dev/null 2>&1; then
        if ! curl -L -o "$temp_file" "$download_url" --progress-bar; then
            log_error "下载失败: $platform"
            return 1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if ! wget -O "$temp_file" "$download_url" --progress=bar:force; then
            log_error "下载失败: $platform"
            return 1
        fi
    else
        log_error "需要安装 curl 或 wget"
        return 1
    fi
    
    if [ ! -f "$temp_file" ]; then
        log_error "下载失败: $platform"
        return 1
    fi
    
    # 解压文件
    local extract_dir="/tmp/duck-cli-extract-$platform"
    rm -rf "$extract_dir"
    mkdir -p "$extract_dir"
    
    log_info "解压 $platform..."
    
    if [ "$file_ext" = "tar.gz" ]; then
        if ! tar -xzf "$temp_file" -C "$extract_dir"; then
            log_error "解压失败: $platform"
            rm -f "$temp_file"
            return 1
        fi
    elif [ "$file_ext" = "zip" ]; then
        if command -v unzip >/dev/null 2>&1; then
            if ! unzip -q "$temp_file" -d "$extract_dir"; then
                log_error "解压失败: $platform"
                rm -f "$temp_file"
                return 1
            fi
        else
            log_error "需要安装 unzip"
            return 1
        fi
    fi
    
    # 查找duck-cli可执行文件
    local duck_cli_exe=$(find "$extract_dir" -name "duck-cli*" -type f ! -name "*.tar.gz" ! -name "*.zip" | head -1)
    
    if [ -z "$duck_cli_exe" ]; then
        log_error "在解压文件中找不到 duck-cli 可执行文件: $platform"
        rm -rf "$extract_dir" "$temp_file"
        return 1
    fi
    
    # 复制到目标位置
    local target_path="${binaries_dir}/${platform}/${binary_name}"
    cp "$duck_cli_exe" "$target_path"
    
    # 设置执行权限
    chmod +x "$target_path"
    
    log_success "已安装 $platform 到: $target_path"
    
    # 清理临时文件
    rm -rf "$extract_dir" "$temp_file"
    
    return 0
}

# 下载所有平台
download_all_platforms() {
    local success_count=0
    local total_count=0
    
    # 计算总数
    for platform in $PLATFORMS; do
        total_count=$((total_count + 1))
    done
    
    log_info "开始下载 $total_count 个平台的二进制文件..."
    echo
    
    for platform in $PLATFORMS; do
        echo "==================== $platform ===================="
        if download_platform "$platform"; then
            success_count=$((success_count + 1))
        fi
        echo
    done
    
    log_info "下载完成: $success_count/$total_count 成功"
    
    if [ $success_count -eq $total_count ]; then
        log_success "所有平台下载成功!"
        return 0
    elif [ $success_count -gt 0 ]; then
        log_warn "部分平台下载成功 ($success_count/$total_count)"
        return 1
    else
        log_error "所有平台下载失败"
        return 1
    fi
}

# 验证安装
verify_installation() {
    local binaries_dir="src-tauri/binaries"
    local verified_count=0
    local total_count=0
    
    # 计算总数
    for platform in $PLATFORMS; do
        total_count=$((total_count + 1))
    done
    
    log_info "验证安装..."
    
    for platform in $PLATFORMS; do
        local platform_info=$(get_platform_info "$platform")
        IFS='|' read -r archive_name file_ext binary_name <<< "$platform_info"
        local binary_path="$binaries_dir/$platform/$binary_name"
        
        if [ -f "$binary_path" ] && [ -x "$binary_path" ]; then
            log_success "$platform: 安装成功"
            verified_count=$((verified_count + 1))
            
            # 尝试获取版本信息 (仅对当前平台)
            local current_os=$(uname -s | tr '[:upper:]' '[:lower:]')
            local current_arch=$(uname -m)
            local should_test=false
            
            case "$current_os" in
                darwin*)
                    [ "$platform" = "macos-universal" ] && should_test=true
                    ;;
                linux*)
                    case "$current_arch" in
                        x86_64|amd64)
                            [ "$platform" = "linux-amd64" ] && should_test=true
                            ;;
                        aarch64|arm64)
                            [ "$platform" = "linux-arm64" ] && should_test=true
                            ;;
                    esac
                    ;;
                mingw*|cygwin*|msys*)
                    case "$current_arch" in
                        x86_64|amd64)
                            [ "$platform" = "windows-amd64" ] && should_test=true
                            ;;
                        aarch64|arm64)
                            [ "$platform" = "windows-arm64" ] && should_test=true
                            ;;
                    esac
                    ;;
            esac
            
            if [ "$should_test" = true ]; then
                if "$binary_path" --version >/dev/null 2>&1; then
                    local version_info=$("$binary_path" --version 2>/dev/null || echo "版本信息不可用")
                    log_info "$platform 版本信息: $version_info"
                fi
            fi
        else
            log_error "$platform: 安装失败"
        fi
    done
    
    if [ $verified_count -eq $total_count ]; then
        log_success "所有平台验证成功!"
        return 0
    else
        log_warn "验证完成: $verified_count/$total_count 成功"
        return 1
    fi
}

# 显示目录结构
show_directory_structure() {
    local binaries_dir="src-tauri/binaries"
    
    log_info "目录结构:"
    if command -v tree >/dev/null 2>&1; then
        tree "$binaries_dir" 2>/dev/null || true
    else
        find "$binaries_dir" -type f -exec ls -la {} \; 2>/dev/null || true
    fi
}

# 主函数
main() {
    echo "🦆 Duck CLI 多平台自动下载脚本"
    echo "=============================="
    echo "支持平台: $PLATFORMS"
    echo "=============================="
    echo
    
    create_directories
    check_existing_binaries
    get_latest_version
    
    if download_all_platforms; then
        echo "=============================="
        verify_installation
        echo "=============================="
        show_directory_structure
        echo "=============================="
        log_success "Duck CLI 多平台下载完成! 现在可以编译多平台 Tauri 应用了。"
    else
        log_error "下载过程中出现错误，请检查网络连接并重试。"
        exit 1
    fi
}

# 运行主函数
main "$@" 