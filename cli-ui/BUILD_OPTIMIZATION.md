# CLI-UI 构建优化记录

## 问题描述

在构建 CLI-UI 应用时，原本使用下载预构建的 duck-cli 二进制文件的方式，但在 GitHub Actions 环境中遇到了依赖问题：

1. **依赖冲突**：整个 workspace 的其他模块（如 client-core）引入了 GTK 相关依赖，导致 Linux 构建失败
2. **系统库缺失**：在 Ubuntu 环境中缺少 `glib-2.0` 等系统库
3. **构建复杂性**：需要为每个平台下载和管理预构建的二进制文件

## 解决方案

### 1. 直接构建 duck-cli 模块

**原来的方式**：
```bash
# 下载预构建的二进制文件
cd cli-ui
chmod +x download-duck-cli.sh
echo "y" | bash ./download-duck-cli.sh
```

**优化后的方式**：
```bash
# 只构建 duck-cli 模块，避免其他模块的依赖问题
cargo build --release --target ${{ matrix.platform.rust_target }} -p duck-cli
```

### 2. 跨平台二进制文件管理

构建完成后，根据 Tauri 自动命名约定复制二进制文件：

```bash
# 使用 Tauri 自动命名约定：binaries/duck-cli-$TARGET_TRIPLE
if [[ "${{ matrix.platform.os }}" == "windows-latest" ]]; then
  # Windows 平台 (.exe 扩展名)
  cp target/${{ matrix.platform.rust_target }}/release/duck-cli.exe \
     cli-ui/src-tauri/binaries/duck-cli-${{ matrix.platform.rust_target }}.exe
else
  # macOS 和 Linux 平台 (无扩展名)
  cp target/${{ matrix.platform.rust_target }}/release/duck-cli \
     cli-ui/src-tauri/binaries/duck-cli-${{ matrix.platform.rust_target }}
fi
```

### 3. 交叉编译支持

为 Linux ARM64 添加交叉编译支持：

```bash
# 安装交叉编译工具链
sudo apt-get install -y gcc-aarch64-linux-gnu
echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc" >> $GITHUB_ENV
```

### 4. Tauri 配置优化

使用 Tauri 的自动命名约定配置外部二进制文件：

```json
{
  "bundle": {
    "externalBin": [
      "binaries/duck-cli"
    ]
  }
}
```

**Tauri 自动命名约定**：
- Linux x86_64: `binaries/duck-cli-x86_64-unknown-linux-gnu`
- Linux aarch64: `binaries/duck-cli-aarch64-unknown-linux-gnu`
- Windows x86_64: `binaries/duck-cli-x86_64-pc-windows-msvc.exe`
- Windows aarch64: `binaries/duck-cli-aarch64-pc-windows-msvc.exe`
- macOS x86_64: `binaries/duck-cli-x86_64-apple-darwin`
- macOS aarch64: `binaries/duck-cli-aarch64-apple-darwin`
- macOS Universal: `binaries/duck-cli-universal-apple-darwin`

## 优化效果

### 1. 解决依赖问题
- ✅ 避免了 workspace 其他模块的依赖冲突
- ✅ 不再需要安装大量的 GTK 系统库
- ✅ 构建过程更加纯净和可预测

### 2. 提高构建效率
- ✅ 减少了外部依赖的下载和管理
- ✅ 构建时间更短（只构建必要的模块）
- ✅ 减少了构建失败的可能性

### 3. 更好的平台支持
- ✅ 支持所有主流平台（Windows、macOS、Linux）
- ✅ 支持多架构（x86_64、ARM64）
- ✅ 自动化的交叉编译配置

## 构建流程

1. **环境准备**：安装 Rust 工具链和目标平台支持
2. **构建 duck-cli**：使用 `cargo build -p duck-cli` 单独构建特定包，避免依赖问题
3. **复制二进制文件**：根据平台将二进制文件复制到 Tauri 期望的位置
4. **构建 Tauri 应用**：使用 tauri-action 构建最终的桌面应用

## 技术细节

- **模块隔离**：使用 `cargo build -p duck-cli` 指定构建特定包
- **目标平台**：使用 `--target` 参数指定构建目标
- **交叉编译**：为 ARM64 平台配置正确的链接器
- **文件组织**：使用 Tauri 自动命名约定 `duck-cli-$TARGET_TRIPLE[.exe]`

这种优化方式彻底解决了构建依赖问题，使 CLI-UI 应用能够在各种环境中稳定构建。 