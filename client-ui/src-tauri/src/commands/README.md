# Commands 模块化结构

原来的单一 `commands.rs` 文件已经被拆分为以下模块化结构：

## 模块组织

```
commands/
├── mod.rs              # 模块导出入口
├── types.rs            # 共享类型定义
├── version.rs          # 版本管理
├── upgrade.rs          # 升级管理  
├── services.rs         # 服务管理
├── system.rs           # 系统检查
├── init.rs             # 初始化
├── directory.rs        # 目录管理
├── ui.rs              # UI配置
├── logs.rs            # 日志管理
└── tasks.rs           # 任务管理
```

## 各模块功能

### types.rs - 共享类型定义
- `VersionInfo` - 版本信息
- `UpgradeInfo` - 升级信息
- `ServiceInfo` - 服务信息  
- `SystemRequirements` - 系统要求
- `AppGlobalState` - 全局状态管理
- 各种事件数据结构

### version.rs - 版本管理
- `get_version_info()` - 获取客户端和服务版本信息

### upgrade.rs - 升级管理
- `check_upgrade_available()` - 检查可用升级
- `start_upgrade_download()` - 开始升级下载
- `simulate_upgrade_progress()` - 模拟升级进度（测试用）

### services.rs - 服务管理
- `get_services_status()` - 获取服务状态
- `start_services_monitoring()` - 启动服务监控
- `start_services()` - 启动服务
- `stop_services()` - 停止服务  
- `restart_services()` - 重启服务
- `monitor_services()` - 监控服务

### system.rs - 系统检查
- `check_system_requirements()` - 检查系统要求
- `get_platform()` - 获取平台信息
- `check_system_storage()` - 检查系统存储空间
- `check_storage_space()` - 检查指定路径存储空间
- `open_file_manager()` - 打开文件管理器

### init.rs - 初始化
- `check_initialization_status()` - 检查初始化状态
- `init_client_with_progress()` - 初始化客户端并显示进度
- `download_package_with_progress()` - 下载包并显示进度

### directory.rs - 目录管理
- `get_app_state()` - 获取应用状态
- `set_working_directory()` - 设置工作目录
- `get_working_directory()` - 获取工作目录
- `reset_working_directory()` - 重设工作目录
- `open_directory()` - 打开目录

### ui.rs - UI配置
- `get_ui_config()` - 获取UI配置
- `update_ui_config()` - 更新UI配置

### logs.rs - 日志管理
- `get_activity_logs()` - 获取活动日志

### tasks.rs - 任务管理
- `get_current_tasks()` - 获取当前任务
- `cancel_task()` - 取消任务

## 使用方式

在 `lib.rs` 中，所有命令通过以下方式注册：

```rust
tauri::generate_handler![
    // 版本管理命令
    commands::version::get_version_info,
    
    // 升级管理命令
    commands::upgrade::check_upgrade_available,
    commands::upgrade::start_upgrade_download,
    // ... 其他命令
]
```

## 优势

1. **模块化**: 相关功能组织在一起，便于维护
2. **可读性**: 每个文件专注于特定功能域
3. **可扩展性**: 容易添加新功能到相应模块
4. **可测试性**: 每个模块可以独立测试
5. **团队协作**: 不同开发者可以独立工作在不同模块上 