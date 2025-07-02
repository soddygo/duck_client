# Duck Client 桌面应用 UI 开发任务

## 项目概述

基于现有的 `duck-cli` 命令行工具，开发对应的 Tauri 2.0 桌面应用，为用户提供图形化的 Docker 服务管理界面。

## 技术栈

- **框架**: Tauri 2.0
- **前端**: React + TypeScript
- **UI组件**: 使用现代化的组件库（建议使用 Ant Design 或 Arco Design）
- **状态管理**: React Hooks + Context API
- **HTTP客户端**: 复用 client-core 的 API 能力
- **包管理器**: Deno

## 功能模块映射

基于 `cli.rs` 的命令结构，将功能映射到以下UI模块：

### 1. 仪表盘 (Dashboard) 
**对应CLI命令**: `duck-cli status`
- **功能**:
  - 显示服务运行状态（运行中/已停止）
  - 显示客户端版本和服务版本
  - 显示API连接状态
  - 快速操作按钮（启动/停止服务）
  - 最近活动日志

### 2. 服务管理 (Service Management)
**对应CLI命令**: `duck-cli docker-service`
- **功能**:
  - Docker服务控制：启动、停止、重启
  - 容器管理：重启指定容器
  - 服务状态监控
  - 架构信息显示
  - Docker镜像管理（加载、设置标签）
  - 集成Ducker功能（可选择在新窗口打开）

### 3. 升级管理 (Upgrade Management)
**对应CLI命令**: `duck-cli upgrade`, `duck-cli check-update`, `duck-cli auto-upgrade-deploy`
- **功能**:
  - 检查服务更新
  - 手动升级（全量/增量）
  - 客户端自更新
  - 自动升级部署配置
  - 延迟升级设置
  - 升级历史查看

### 4. 备份与恢复 (Backup & Recovery)
**对应CLI命令**: `duck-cli backup`, `duck-cli list-backups`, `duck-cli rollback`, `duck-cli auto-backup`
- **功能**:
  - 手动创建备份
  - 备份列表展示
  - 从备份恢复（带确认对话框）
  - 自动备份配置（启用/禁用、Cron表达式设置）
  - 备份状态监控

### 5. 系统设置 (Settings)
**对应CLI命令**: `duck-cli init`, `duck-cli api-info`
- **功能**:
  - 客户端初始化
  - API配置管理
  - 更新服务器设置
  - 检查更新频率配置
  - 备份路径设置
  - 日志级别配置

### 6. 关于 (About)
- **功能**:
  - 应用版本信息
  - 许可证信息
  - 诊断日志上传
  - 帮助文档链接

## UI设计要求

### 布局结构
```
┌─────────────────────────────────────────────────────┐
│                    标题栏                           │
├───────────┬─────────────────────────────────────────┤
│           │                                         │
│  侧边导航  │              主内容区                   │
│           │                                         │
│  [图标]   │                                         │
│  仪表盘   │                                         │
│           │                                         │
│  [图标]   │                                         │
│  服务管理  │                                         │
│           │                                         │
│  [图标]   │                                         │
│  升级管理  │                                         │
│           │                                         │
│  [图标]   │                                         │
│  备份恢复  │                                         │
│           │                                         │
│  [图标]   │                                         │
│  系统设置  │                                         │
│           │                                         │
│  [图标]   │                                         │
│  关于     │                                         │
│           │                                         │
└───────────┴─────────────────────────────────────────┘
```

### 设计原则
1. **简洁至上**: 界面简洁，避免信息过载
2. **状态清晰**: 使用颜色和图标明确表示状态（绿色=运行中，红色=停止，黄色=处理中）
3. **操作安全**: 危险操作（如回滚、重启）需要确认对话框
4. **实时反馈**: 长时间操作显示进度条和状态信息

### 颜色规范
- **成功状态**: 绿色 (#52c41a)
- **错误状态**: 红色 (#ff4d4f)  
- **警告状态**: 橙色 (#fa8c16)
- **处理中状态**: 蓝色 (#1677ff)
- **主色调**: 蓝色系

## 开发阶段

### 阶段1: 基础框架搭建
- [ ] 安装和配置UI组件库
- [ ] 创建基础布局组件（侧边栏、主内容区）
- [ ] 设置路由导航
- [ ] 创建基础页面组件骨架
- [ ] 集成Tauri API调用

### 阶段2: 核心功能开发
- [ ] **仪表盘页面**
  - [ ] 服务状态显示组件
  - [ ] 版本信息组件  
  - [ ] 快速操作按钮
  - [ ] 活动日志组件

- [ ] **服务管理页面**
  - [ ] 服务控制面板
  - [ ] 容器状态列表
  - [ ] Docker镜像管理
  - [ ] 实时日志查看器

### 阶段3: 高级功能开发  
- [ ] **升级管理页面**
  - [ ] 更新检查和通知
  - [ ] 升级进度追踪
  - [ ] 自动升级配置界面

- [ ] **备份恢复页面**
  - [ ] 备份列表和操作
  - [ ] 恢复确认对话框
  - [ ] 自动备份配置

### 阶段4: 完善和优化
- [ ] **设置页面**
  - [ ] 配置表单
  - [ ] 初始化向导

- [ ] **关于页面**
  - [ ] 版本信息
  - [ ] 诊断工具

- [ ] **全局优化**
  - [ ] 错误处理和用户反馈
  - [ ] 快捷键支持
  - [ ] 主题适配（浅色/深色）
  - [ ] 响应式设计

## Tauri命令接口设计

需要在Rust后端创建以下Tauri命令，调用 `client-core` 的功能：

```rust
// 服务状态相关
#[tauri::command]
async fn get_service_status() -> Result<ServiceStatus, String>

#[tauri::command] 
async fn start_service() -> Result<(), String>

#[tauri::command]
async fn stop_service() -> Result<(), String>

// 升级相关
#[tauri::command]
async fn check_updates() -> Result<UpdateInfo, String>

#[tauri::command]
async fn perform_upgrade(full: bool, force: bool) -> Result<(), String>

// 备份相关
#[tauri::command]
async fn create_backup() -> Result<BackupInfo, String>

#[tauri::command]
async fn list_backups() -> Result<Vec<BackupInfo>, String>

#[tauri::command]
async fn restore_backup(backup_id: i64, force: bool) -> Result<(), String>

// 配置相关
#[tauri::command]
async fn get_config() -> Result<ClientConfig, String>

#[tauri::command]
async fn update_config(config: ClientConfig) -> Result<(), String>
```

## 开发注意事项

1. **错误处理**: 所有异步操作都要有适当的错误处理和用户提示
2. **进度反馈**: 长时间操作（升级、备份）要显示进度条
3. **确认对话框**: 危险操作必须有二次确认
4. **实时更新**: 服务状态等信息要能实时刷新
5. **离线支持**: 在API不可用时提供适当的降级体验
6. **日志记录**: 关键操作要有日志记录，便于问题排查

## 验收标准

- [ ] 所有CLI功能都有对应的图形界面
- [ ] 界面响应流畅，操作直观
- [ ] 错误处理完善，用户提示清晰
- [ ] 危险操作有适当的安全措施
- [ ] 支持Windows、macOS、Linux三个平台
- [ ] 通过基本的用户体验测试 