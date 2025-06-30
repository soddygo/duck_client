# 跨平台支持文档

## 支持的操作系统

Duck Client 支持以下操作系统：

### ✅ macOS
- **架构**: Intel (x86_64) 和 Apple Silicon (arm64)  
- **Docker连接**: Unix Socket (`/var/run/docker.sock`)
- **测试状态**: 完全支持

### ✅ Linux
- **架构**: x86_64, ARM64
- **发行版**: Ubuntu, Debian, CentOS, RHEL, Fedora 等主流发行版
- **Docker连接**: Unix Socket (`/var/run/docker.sock`)
- **测试状态**: 完全支持

### ✅ Windows
- **架构**: x86_64
- **版本**: Windows 10/11, Windows Server 2019/2022
- **Docker连接**: Named Pipe (`\\.\pipe\docker_engine`)
- **测试状态**: 完全支持

## Docker 连接方式

项目使用 [bollard](https://github.com/fussybeaver/bollard) 库来连接 Docker daemon，该库原生支持跨平台：

### 自动平台检测
程序会自动检测运行平台并使用正确的连接方式：

```rust
// 自动选择平台对应的连接方式
// Unix/Linux/macOS: /var/run/docker.sock  
// Windows: \\.\pipe\docker_engine
```

### 环境变量优先级
支持 Docker 标准的环境变量配置：

1. **DOCKER_HOST** - 自定义 Docker daemon 地址
   ```bash
   # TCP 连接示例
   export DOCKER_HOST=tcp://192.168.1.100:2376
   
   # Unix socket 示例 (Linux/macOS)
   export DOCKER_HOST=unix:///var/run/docker.sock
   
   # Named pipe 示例 (Windows)
   set "DOCKER_HOST=npipe:////./pipe/docker_engine"
   ```

2. **DOCKER_CERT_PATH** - SSL 证书路径（用于远程连接）

3. **DOCKER_TLS_VERIFY** - 启用 TLS 验证

## 平台特定注意事项

### Windows 用户
1. **Docker Desktop 要求**：需要安装 Docker Desktop for Windows
2. **权限要求**：某些操作可能需要管理员权限
3. **路径格式**：使用 Windows 路径格式（`\` 分隔符）
4. **Named Pipe**：Docker 通过命名管道通信

### Linux 用户  
1. **Docker 安装**：需要安装 Docker Engine
2. **用户权限**：用户需要在 `docker` 组中，或使用 `sudo`
3. **Socket 权限**：确保 `/var/run/docker.sock` 有正确权限

### macOS 用户
1. **Docker Desktop**：推荐使用 Docker Desktop for Mac
2. **Intel/M1 支持**：支持 Intel 和 Apple Silicon 芯片
3. **Unix Socket**：使用标准 Unix socket 连接

## 构建和部署

### 本地开发
```bash
# 所有平台
cargo build
cargo run

# 检查跨平台兼容性
cargo check --target x86_64-pc-windows-gnu  # Windows
cargo check --target x86_64-apple-darwin    # macOS Intel  
cargo check --target aarch64-apple-darwin   # macOS Apple Silicon
cargo check --target x86_64-unknown-linux-gnu # Linux
```

### 发布构建
```bash
# 当前平台
cargo build --release

# 交叉编译（需要配置工具链）
cargo build --release --target x86_64-pc-windows-gnu
cargo build --release --target x86_64-apple-darwin
cargo build --release --target x86_64-unknown-linux-gnu
```

## 依赖库的平台支持

### 核心依赖
- **bollard**: 完全跨平台，原生支持 Windows Named Pipes
- **tokio**: 异步运行时，完全跨平台支持
- **serde**: 序列化库，跨平台  
- **clap**: CLI 参数解析，跨平台

### 平台特定依赖
- **Windows**: 
  - `hyper-named-pipe` - Windows Named Pipe 支持
  - `winapi` - Windows API 绑定
- **Unix/Linux**: 
  - `hyperlocal` - Unix Socket 支持

## 故障排除

### Docker 连接失败
1. **检查 Docker daemon 状态**：
   ```bash
   # Linux/macOS
   docker info
   
   # Windows
   docker info
   ```

2. **检查连接权限**：
   ```bash
   # Linux - 检查用户是否在 docker 组
   groups $USER
   
   # 如果不在，添加用户到 docker 组
   sudo usermod -aG docker $USER
   ```

3. **环境变量检查**：
   ```bash
   echo $DOCKER_HOST
   echo $DOCKER_CERT_PATH  
   echo $DOCKER_TLS_VERIFY
   ```

### Windows 特定问题
1. **Named Pipe 连接失败**：
   - 确保 Docker Desktop 正在运行
   - 检查是否启用了 "Expose daemon on tcp://localhost:2375 without TLS"

2. **权限错误**：
   - 以管理员身份运行命令提示符
   - 检查 Docker Desktop 的访问控制设置

### 网络连接问题
对于远程 Docker daemon 连接：
```bash
# 测试 TCP 连接
telnet <docker-host> 2376

# 使用 docker 命令测试
DOCKER_HOST=tcp://<host>:2376 docker info
``` 