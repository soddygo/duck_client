# Duck CLI GUI 界面技术设计文档

## 项目概述

本文档描述如何创建一个 Tauri GUI 应用，通过图形界面调用 duck-cli 命令行工具，实现 Docker 服务的可视化管理。

## 核心需求

### 功能需求
1. **CLI 工具集成**：自动下载并集成最新的 duck-cli 工具
2. **分割式界面**：上半部分为操作面板，下半部分为终端窗口
3. **核心功能按钮**：提供主要 Docker 服务管理功能的 GUI 操作
4. **实时日志**：在终端窗口中显示 duck-cli 命令的输出
5. **应用更新**：支持 Tauri updater 插件进行应用自动更新

### 非功能需求
- 跨平台支持（Windows、macOS、Linux）
- 响应式界面设计
- 实时命令执行状态反馈
- 错误处理和用户友好的错误提示

## 架构设计

### 整体架构

```
┌─────────────────────────────────────────┐
│           Tauri Frontend (React)        │
├─────────────────────────────────────────┤
│  ┌───────────── GUI Panel ─────────────┐ │
│  │ • 初始化     • 服务启停            │ │
│  │ • 下载更新   • 版本检测            │ │  
│  │ • 一键部署   • 备份回滚            │ │
│  │ • 应用更新                         │ │
│  └───────────────────────────────────────┘ │
│  ┌──────────── Terminal Panel ────────────┐ │
│  │ • 命令输出日志                     │ │
│  │ • 交互式命令输入                   │ │
│  │ • 实时状态显示                     │ │
│  └───────────────────────────────────────┘ │
├─────────────────────────────────────────┤
│           Tauri Backend (Rust)          │
│  • Command 执行                        │
│  • 文件系统操作                       │
│  • CLI 工具管理                       │
│  • 进程管理                           │
└─────────────────────────────────────────┘
```

## 技术实现方案

### 1. CLI 工具集成策略

#### 1.1 自动下载和集成

```rust
// src-tauri/src/commands/cli_manager.rs
pub async fn download_latest_cli() -> Result<PathBuf, String> {
    // 1. 从 GitHub Releases API 获取最新版本信息
    let latest_release = fetch_latest_release().await?;
    
    // 2. 根据当前平台选择合适的下载包
    let platform_asset = select_platform_asset(&latest_release)?;
    
    // 3. 下载到应用数据目录
    let cli_path = download_and_extract(&platform_asset).await?;
    
    // 4. 设置可执行权限（Unix系统）
    set_executable_permissions(&cli_path)?;
    
    Ok(cli_path)
}
```

#### 1.2 CLI 工具路径管理

```toml
# tauri.conf.json - 资源配置
{
  "bundle": {
    "resources": [
      "bin/duck-cli*"
    ]
  }
}
```

### 2. 用户界面设计

#### 2.1 布局结构

```tsx
// src/components/CliInterface.tsx
export const CliInterface: React.FC = () => {
  return (
    <div className="h-screen flex flex-col">
      {/* 上半部分：操作面板 */}
      <div className="flex-1 p-4 bg-gray-50">
        <ControlPanel />
      </div>
      
      {/* 分割线 */}
      <div className="h-1 bg-gray-300 cursor-row-resize" />
      
      {/* 下半部分：终端窗口 */}
      <div className="flex-1 bg-black text-green-400 p-4">
        <TerminalWindow />
      </div>
    </div>
  );
};
```

#### 2.2 控制面板组件

```tsx
// src/components/ControlPanel.tsx
const CORE_FUNCTIONS = [
  { id: 'init', label: '初始化', command: 'init', icon: '🚀' },
  { id: 'upgrade', label: '下载服务', command: 'upgrade --full', icon: '⬇️' },
  { id: 'deploy', label: '一键部署', command: 'auto-upgrade-deploy run', icon: '🚀' },
  { id: 'start', label: '启动服务', command: 'docker-service start', icon: '▶️' },
  { id: 'stop', label: '停止服务', command: 'docker-service stop', icon: '⏹️' },
  { id: 'restart', label: '重启服务', command: 'docker-service restart', icon: '🔄' },
  { id: 'check-update', label: '检查更新', command: 'upgrade --check', icon: '🔍' },
  { id: 'upgrade-service', label: '升级服务', command: 'upgrade --full', icon: '⬆️' },
  { id: 'backup', label: '创建备份', command: 'backup', icon: '💾' },
  { id: 'rollback', label: '回滚服务', command: 'list-backups', icon: '↩️' },
];

export const ControlPanel: React.FC = () => {
  return (
    <div className="grid grid-cols-5 gap-4">
      {CORE_FUNCTIONS.map(func => (
        <FunctionButton key={func.id} {...func} />
      ))}
      
      {/* 应用更新按钮 */}
      <UpdateButton />
    </div>
  );
};
```

#### 2.3 终端窗口组件

```tsx
// src/components/TerminalWindow.tsx
export const TerminalWindow: React.FC = () => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [inputValue, setInputValue] = useState('');
  
  return (
    <div className="h-full flex flex-col font-mono">
      {/* 日志输出区域 */}
      <div className="flex-1 overflow-y-auto p-2">
        {logs.map((log, index) => (
          <LogLine key={index} entry={log} />
        ))}
      </div>
      
      {/* 命令输入区域 */}
      <div className="flex items-center p-2 border-t border-gray-600">
        <span className="text-green-400 mr-2">duck-cli$</span>
        <input
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyPress={handleKeyPress}
          className="flex-1 bg-transparent text-green-400 outline-none"
          placeholder="输入命令..."
        />
      </div>
    </div>
  );
};
```

### 3. 后端命令执行

#### 3.1 命令执行服务

```rust
// src-tauri/src/commands/cli_executor.rs
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};

#[tauri::command]
pub async fn execute_cli_command(
    command: String,
    args: Vec<String>,
    window: tauri::Window,
) -> Result<(), String> {
    let cli_path = get_cli_executable_path()?;
    
    let mut cmd = Command::new(cli_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to execute command: {}", e))?;
    
    // 实时输出处理
    if let Some(stdout) = cmd.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        
        while let Some(line) = lines.next_line().await.unwrap_or(None) {
            // 发送日志到前端
            window.emit("cli-output", &line).unwrap();
        }
    }
    
    let status = cmd.wait().await.map_err(|e| e.to_string())?;
    
    if !status.success() {
        return Err(format!("Command failed with exit code: {:?}", status.code()));
    }
    
    Ok(())
}
```

#### 3.2 CLI 工具管理

```rust
// src-tauri/src/services/cli_manager.rs
pub struct CliManager {
    cli_path: PathBuf,
    version: Option<String>,
}

impl CliManager {
    pub async fn new() -> Result<Self, String> {
        let cli_path = Self::ensure_cli_available().await?;
        let version = Self::get_cli_version(&cli_path).await?;
        
        Ok(Self { cli_path, version })
    }
    
    async fn ensure_cli_available() -> Result<PathBuf, String> {
        // 1. 检查本地是否已有 CLI 工具
        if let Some(path) = Self::find_local_cli() {
            return Ok(path);
        }
        
        // 2. 从 GitHub 下载最新版本
        Self::download_latest_cli().await
    }
    
    pub async fn check_and_update(&mut self) -> Result<bool, String> {
        let latest_version = self.fetch_latest_version().await?;
        
        if self.version.as_ref() != Some(&latest_version) {
            self.cli_path = Self::download_latest_cli().await?;
            self.version = Some(latest_version);
            return Ok(true);
        }
        
        Ok(false)
    }
}
```

### 4. 应用更新集成

#### 4.1 Tauri Updater 配置

```toml
# Cargo.toml
[dependencies]
tauri-plugin-updater = "2.0"
```

```json
// tauri.conf.json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/soddygo/duck_client/releases/latest/download/latest.json"
      ],
      "dialog": true,
      "pubkey": "YOUR_PUBLIC_KEY"
    }
  }
}
```

#### 4.2 更新检查组件

```tsx
// src/components/UpdateButton.tsx
import { check } from '@tauri-apps/plugin-updater';

export const UpdateButton: React.FC = () => {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  
  const checkForUpdates = async () => {
    try {
      const update = await check();
      if (update?.available) {
        setUpdateAvailable(true);
        // 显示更新对话框
        await update.downloadAndInstall();
      }
    } catch (error) {
      console.error('Update check failed:', error);
    }
  };
  
  return (
    <button 
      onClick={checkForUpdates}
      className={`p-3 rounded-lg ${updateAvailable ? 'bg-orange-500' : 'bg-blue-500'}`}
    >
      {updateAvailable ? '🆙 有可用更新' : '🔄 检查应用更新'}
    </button>
  );
};
```

## 实施计划

### Phase 1: 基础框架搭建
1. ✅ 创建基本的分割界面布局
2. ✅ 实现 CLI 工具下载和集成逻辑
3. ✅ 建立前后端通信机制

### Phase 2: 核心功能实现
1. 🔄 实现所有核心功能按钮
2. 🔄 开发终端窗口组件
3. 🔄 集成实时日志输出

### Phase 3: 高级功能和优化
1. ⏳ 集成 Tauri updater 插件
2. ⏳ 添加错误处理和用户反馈
3. ⏳ 性能优化和界面美化

### Phase 4: 测试和发布
1. ⏳ 跨平台测试
2. ⏳ 用户体验测试
3. ⏳ 构建自动化流程

## 技术依赖

### 前端依赖
```json
{
  "dependencies": {
    "react": "^18.0.0",
    "typescript": "^5.0.0",
    "tailwindcss": "^3.0.0",
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-updater": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0"
  }
}
```

### 后端依赖
```toml
[dependencies]
tauri = { version = "2.0", features = ["protocol-asset"] }
tauri-plugin-updater = "2.0"
tauri-plugin-shell = "2.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
reqwest = "0.11"
```

## 安全考虑

1. **命令执行安全**：限制可执行的命令范围，避免任意命令执行
2. **文件系统访问**：使用 Tauri 的安全文件系统 API
3. **网络请求**：验证下载文件的完整性
4. **更新安全**：使用数字签名验证更新包

## 部署策略

1. **自动构建**：通过 GitHub Actions 自动构建多平台版本
2. **版本管理**：与 CLI 工具版本保持同步
3. **分发渠道**：通过 GitHub Releases 分发
4. **更新机制**：支持自动更新和手动更新

---

## 总结

本设计文档提供了一个完整的技术方案，将 duck-cli 命令行工具集成到用户友好的 Tauri GUI 应用中。通过分割式界面设计，用户可以通过图形界面轻松管理 Docker 服务，同时保留命令行的强大功能和灵活性。 