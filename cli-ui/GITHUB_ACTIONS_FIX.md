# GitHub Actions 构建修复

## 问题描述
GitHub Actions 在构建 CLI-UI Tauri 应用时出现 GObject 系统依赖缺失错误：

### 第一轮错误：glib-2.0 缺失
```
The system library `glib-2.0` required by crate `glib-sys` was not found.
The file `glib-2.0.pc` needs to be installed and the PKG_CONFIG_PATH environment variable must contain its parent directory.
```

### 第二轮错误：gobject-2.0 缺失
```
The system library `gobject-2.0` required by crate `gobject-sys` was not found.
The file `gobject-2.0.pc` needs to be installed and the PKG_CONFIG_PATH environment variable must contain its parent directory.
```

## 问题原因
1. **Workspace 依赖传播**: 在构建 `duck-cli` 时，Cargo 解析了整个 workspace 的依赖，包括 Tauri 相关的 GTK 依赖
2. **Linux 依赖不完整**: Ubuntu 环境缺少 `libglib2.0-dev` 和相关的 GTK 开发库
3. **PKG_CONFIG 配置**: 缺少 `pkg-config` 工具来正确定位系统库

## 解决方案

### 1. 补充 Linux 系统依赖
在 `.github/workflows/cli-ui-build.yml` 中添加完整的依赖：

```yaml
- name: Install Linux dependencies for Tauri
  if: startsWith(matrix.platform.os, 'ubuntu')
  run: |
    sudo apt-get update
    sudo apt-get install -y \
      libwebkit2gtk-4.0-dev \
      libwebkit2gtk-4.1-dev \
      libappindicator3-dev \
      librsvg2-dev \
      patchelf \
      libssl-dev \
      libgtk-3-dev \
      libayatana-appindicator3-dev \
      libglib2.0-dev \              # 🔧 新增：解决 glib-2.0 缺失
      libgobject-2.0-dev \          # 🔧 新增：解决 gobject-2.0 缺失
      libgio-2.0-dev \              # 🔧 新增：GIO 系统库
      libcairo2-dev \               # 🔧 新增：Cairo 图形库
      libpango1.0-dev \             # 🔧 新增：文本渲染库
      libatk1.0-dev \               # 🔧 新增：可访问性工具包
      libgdk-pixbuf-2.0-dev \       # 🔧 新增：图像加载库
      libsoup2.4-dev \              # 🔧 新增：HTTP 客户端库
      libjavascriptcoregtk-4.0-dev \ # 🔧 新增：JavaScript 引擎
      pkg-config \                  # 🔧 新增：库配置工具
      build-essential               # 🔧 新增：基础构建工具
```

### 2. 依赖说明

#### 核心修复依赖
- **libglib2.0-dev**: 解决 glib-2.0 缺失，提供 `glib-2.0.pc` 文件
- **libgobject-2.0-dev**: 解决 gobject-2.0 缺失，提供 `gobject-2.0.pc` 文件
- **libgio-2.0-dev**: 提供 GIO 系统库，完整的 GObject 生态系统
- **pkg-config**: 允许构建系统正确找到和链接系统库
- **build-essential**: 提供 GCC 编译器和基础构建工具

#### 支持依赖
- **libcairo2-dev**: 2D 图形库，Tauri 图形渲染需要
- **libpango1.0-dev**: 文本布局和渲染库
- **libatk1.0-dev**: 可访问性支持库
- **libgdk-pixbuf-2.0-dev**: 图像加载和操作库
- **libsoup2.4-dev**: HTTP 网络库
- **libjavascriptcoregtk-4.0-dev**: WebKit JavaScript 引擎

### 3. 构建流程
构建流程保持简单有效：

1. **安装依赖**: 完整的 Linux 系统依赖
2. **构建 duck-cli**: 使用 `cargo build --release --target $TARGET -p duck-cli`
3. **复制二进制**: 使用 Tauri 命名约定 `duck-cli-$TARGET_TRIPLE`
4. **构建 Tauri**: 使用 `tauri-action` 构建桌面应用

### 4. 跨平台支持
- ✅ **Linux** (x86_64, ARM64): 完整依赖安装
- ✅ **Windows** (x86_64, ARM64): 无需额外依赖
- ✅ **macOS** (x86_64, ARM64, Universal): 系统自带依赖

## 验证方法

### 本地验证
```bash
# 在 Ubuntu 环境中测试
sudo apt-get install libglib2.0-dev libgobject-2.0-dev libgio-2.0-dev pkg-config

# 验证 GLib 库
pkg-config --exists glib-2.0
echo $?  # 应该输出 0

# 验证 GObject 库
pkg-config --exists gobject-2.0
echo $?  # 应该输出 0

# 验证 GIO 库
pkg-config --exists gio-2.0
echo $?  # 应该输出 0

# 检查完整的 GObject 系统
pkg-config --modversion glib-2.0 gobject-2.0 gio-2.0
```

### CI 验证
检查 GitHub Actions 日志：
1. 依赖安装成功
2. `cargo build` 成功完成
3. Tauri 应用构建成功
4. 生成跨平台构建产物

## 技术背景

### Workspace 依赖传播
在 Rust workspace 中，即使使用 `-p duck-cli` 只构建特定包，Cargo 仍会：
1. 解析整个 workspace 的 `Cargo.lock`
2. 检查所有依赖的可用性
3. 链接时需要满足所有传递依赖

### Tauri 系统依赖
Tauri 应用需要 WebKit 和 GTK 生态系统：
- **WebKit**: 渲染 Web 前端
- **GTK**: 原生窗口和控件
- **GObject 系统**: GTK 的基础对象系统
  - **GLib**: 核心库系统和工具
  - **GObject**: 面向对象的类型系统
  - **GIO**: 现代 I/O 和应用程序框架
- **Cairo/Pango**: 图形和文本渲染

## 日期
2024-07-05 - GitHub Actions 构建问题修复完成 