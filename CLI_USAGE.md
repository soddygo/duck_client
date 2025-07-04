# Duck Client CLI 使用指南

Duck Client CLI 是一个用于管理和升级Docker服务的命令行工具。

## 🚀 首次使用（完整初始化流程）

第一次使用Duck Client时，请按照以下步骤进行初始化：

### 1. 初始化客户端

```bash
duck-cli init
```

这个命令会：
- ✅ 创建配置文件 `config.toml`
- ✅ 创建DuckDB数据库 `history.db`
- ✅ 创建必要的目录结构（`docker/`, `backups/`, `cacheDuckData/`）
- ✅ 生成唯一的客户端UUID
- ✅ 向服务器注册客户端（如果网络可用）
- 📝 显示后续步骤的引导信息

### 2. 下载服务包

```bash
duck-cli upgrade
```

这个命令会：
- 🔍 自动检测到这是首次部署
- 📥 自动下载完整的Docker服务包 `docker.zip`
- 💾 保存到 `cacheDuckData/download/` 目录
- 📝 提示用户执行部署操作

**支持的参数：**
```bash
duck-cli upgrade --full    # 强制下载完整服务包
duck-cli upgrade --force   # 强制重新下载（用于文件损坏时）
```

### 3. 部署Docker服务

```bash
duck-cli docker-service deploy
```

这个命令会：
- 📦 自动检测并解压 `docker.zip` 到 `docker/` 目录
- 📦 加载Docker镜像文件
- 🏷️ 设置镜像标签
- ✅ 验证 `docker-compose.yml` 文件存在
- 🚀 启动所有Docker服务容器
- 🎉 显示部署成功信息

**💡 提示**：如果只需要启动已部署的服务：
```bash
duck-cli docker-service start
```

## 📊 常用命令

### 查看状态
```bash
duck-cli status
```
显示：
- 客户端和服务版本信息
- 文件状态（配置、服务包、Docker Compose文件）
- Docker服务运行状态
- 智能建议下一步操作

### 服务控制
```bash
# 启动服务
duck-cli docker-service start

# 停止服务  
duck-cli docker-service stop

# 重启服务
duck-cli docker-service restart

# 检查服务状态
duck-cli docker-service status

# 重启指定容器
duck-cli docker-service restart-container <容器名称>

# 解压Docker服务包
duck-cli docker-service extract

# 解压指定的docker.zip文件
duck-cli docker-service extract --file /path/to/docker.zip

# 解压指定版本的服务包
duck-cli docker-service extract --version 1.2.0

# 加载Docker镜像
duck-cli docker-service load-images

# 设置镜像标签
duck-cli docker-service setup-tags

# 显示架构信息
duck-cli docker-service arch-info

# 列出Docker镜像（使用ducker）
duck-cli docker-service list-images
```

### 更新和升级
```bash
# 检查客户端自身更新
duck-cli check-update check

# 安装最新版本客户端
duck-cli check-update install

# 安装指定版本客户端
duck-cli check-update install --version 1.2.0

# 强制重新安装客户端（即使当前已是最新版本）
duck-cli check-update install --force

# 检查是否有可用的服务升级版本（不执行下载）
duck-cli upgrade --check

# 下载Docker服务包（首次部署也使用此命令）
duck-cli upgrade

# 下载完整服务包
duck-cli upgrade --full

# 强制重新下载（用于文件损坏时）
duck-cli upgrade --force
```

### 备份和恢复

**冷备份机制**：为确保数据一致性，备份功能采用冷备份模式，要求所有Docker服务处于停止状态。

```bash
# 创建手动备份（冷备份）
duck-cli backup

# 列出所有备份
duck-cli list-backups

# 从指定备份恢复
duck-cli rollback <backup_id>

# 强制恢复（跳过确认）
duck-cli rollback <backup_id> --force
```

**备份内容说明**：
- 🗄️ **数据目录**: `./docker/data/` - 数据库和容器持久化数据
- 📱 **应用目录**: `./docker/app/` - Java应用jar包和前端资源
- 🎯 **精确备份**: 只备份关键数据目录，不包含配置文件和临时文件
- 📦 **合并压缩**: 将多个目录合并到单个 .tar.gz 文件中

**备份流程**：
1. 检查Docker服务状态（必须全部停止）
2. 扫描关键目录（data/, app/）
3. 精确备份指定目录：只打包 `data/` 和 `app/` 目录为 .tar.gz 文件
4. 生成人类易读的备份文件名：`backup_{类型}_v{版本}_{时间}.tar.gz`
5. 记录备份信息到本地数据库

**备份文件命名格式**：
- **手动备份**: `backup_manual_v1.0.0_2025-06-29_14-30-15.tar.gz`
- **升级前备份**: `backup_pre-upgrade_v1.0.0_2025-06-29_14-30-15.tar.gz`

**备份列表功能**：
- 显示所有备份记录及其状态（可用/文件缺失）
- 检查备份文件实际存在性
- 显示文件大小和存储统计
- 提供清晰的操作建议

### 缓存管理

Duck CLI 提供了完整的缓存管理功能，可以帮助您清理磁盘空间并优化性能。

```bash
# 显示缓存使用情况
duck-cli cache status

# 清理所有缓存文件
duck-cli cache clear

# 清理下载缓存，保留最新3个版本
duck-cli cache clean-downloads

# 清理下载缓存，保留最新5个版本
duck-cli cache clean-downloads --keep 5
```

**缓存类型说明**：
- **下载缓存**: `cacheDuckData/download/` - 存储不同版本的Docker服务包
- **临时缓存**: 构建和解压过程中产生的临时文件
- **历史缓存**: 旧版本的备份和日志文件

**缓存清理策略**：
- `cache status`: 显示各类缓存的大小和占用情况
- `cache clear`: 清理所有可安全删除的缓存文件
- `cache clean-downloads --keep N`: 保留最新N个版本的下载文件，删除其余版本

### 🐋 Ducker 集成

Duck CLI 集成了强大的 Docker 容器管理工具 Ducker，提供终端界面的容器管理功能。

```bash
# 启动ducker终端界面
duck-cli ducker

# 传递参数给ducker
duck-cli ducker --help

# 使用ducker的特定功能
duck-cli ducker [ducker参数...]
```

### 其他命令
```bash
# 显示API配置信息
duck-cli api-info

# 显示帮助信息
duck-cli --help

# 查看特定命令的帮助
duck-cli [command] --help

# 详细输出模式
duck-cli --verbose [command]

# 使用自定义配置文件
duck-cli --config /path/to/config.toml [command]

# 组合参数使用示例
duck-cli --verbose --config ./my-config.toml status
```

## 📁 文件结构

初始化完成后，您的工作目录将包含以下文件和目录：

```
.
├── config.toml                    # 配置文件（可手动编辑）
├── history.db                     # DuckDB数据库（存储历史记录）
├── docker/                        # Docker服务文件目录
│   ├── docker-compose.yml         # Docker Compose配置
│   ├── data/                       # 服务数据目录（升级时会保留）
│   └── app/                        # 应用目录（Java工程和前端资源）
├── backups/                       # 备份存储目录
└── cacheDuckData/                 # 缓存目录
    └── download/                   # 下载缓存（按版本组织）
        ├── 1.0.0/                  # 版本1.0.0
        │   └── full/               # 全量下载
        │       └── docker.zip      # 服务包文件
        ├── 1.1.0/                  # 版本1.1.0
        │   └── full/               # 全量下载
        │       └── docker.zip      # 服务包文件
        └── 1.2.0/                  # 版本1.2.0（最新）
            └── full/               # 全量下载
                └── docker.zip      # 服务包文件
```

## ⚠️ 重要提示

1. **首次初始化**：请严格按照 `init` → `check-update` → `start` 的顺序执行
2. **配置文件**：可以手动编辑 `config.toml` 来自定义备份目录等设置
3. **数据安全**：升级时会自动备份，`docker/data/` 目录中的用户数据会被保留
4. **网络要求**：`check-update` 和 `upgrade` 命令需要网络连接
5. **Docker要求**：确保系统已安装Docker和Docker Compose

## 🔧 故障排除

### 常见问题

**Q: init命令提示文件已存在？**
A: 使用 `duck-cli init --force` 强制重新初始化

**Q: check-update下载失败？**
A: 检查网络连接和服务器可用性，稍后重试

**Q: start命令提示Docker未安装？**
A: 请先安装Docker和Docker Compose

**Q: 服务启动失败？**
A: 检查端口占用情况，使用 `docker-compose logs` 查看详细错误

### 状态诊断

使用 `duck-cli status` 命令可以快速诊断当前状态并获得相应的操作建议。

## 🤖 自动化命令

Duck CLI 提供了两个强大的自动化命令，可以简化复杂的运维流程，减少人工操作错误。

### 自动备份 (auto-backup)

自动备份功能提供了一个完整的自动化备份流程：**停止服务 → 备份数据 → 重启服务**

```bash
# 立即执行一次自动备份
duck-cli auto-backup run

# 配置定时备份（显示当前配置）
duck-cli auto-backup cron

# 设置自定义cron表达式（如每天凌晨3点）
duck-cli auto-backup cron "0 3 * * *"

# 启用/禁用自动备份
duck-cli auto-backup enabled
duck-cli auto-backup enabled true    # 启用
duck-cli auto-backup enabled false   # 禁用

# 查看自动备份状态
duck-cli auto-backup status
```

**工作流程**：
1. 🔍 检查Docker服务运行状态
2. ⏹️  如果服务运行中，自动停止所有Docker服务
3. ⏳ 等待服务完全停止（5秒）
4. 💾 执行冷备份，确保数据一致性
5. ▶️  重新启动Docker服务
6. ⏳ 等待服务启动完成（10秒）
7. ✅ 验证服务状态并报告结果

**智能处理**：
- 如果Docker服务未运行，直接进行备份，跳过停止/启动步骤
- 提供详细的进度反馈和状态信息
- 失败时给出明确的错误信息和建议

### 自动升级部署 (auto-upgrade-deploy)

自动升级部署功能提供了完整的升级和部署自动化流程：**下载最新版本 → 智能备份 → 部署服务 → 启动服务**

```bash
# 立即执行自动升级部署
duck-cli auto-upgrade-deploy run

# 指定前端服务端口执行升级部署
duck-cli auto-upgrade-deploy run --port 8080

# 延迟2小时后执行升级部署
duck-cli auto-upgrade-deploy delay-time-deploy 2

# 延迟30分钟后执行升级部署
duck-cli auto-upgrade-deploy delay-time-deploy 30 --unit minutes

# 延迟1天后执行升级部署
duck-cli auto-upgrade-deploy delay-time-deploy 1 --unit days

# 查看自动升级部署状态
duck-cli auto-upgrade-deploy status
```

**工作流程**：
1. 📥 下载最新的Docker服务版本（全量下载）
2. 🔍 检查当前Docker服务状态
3. 🧠 **智能备份决策**：
   - 如果服务运行中：停止服务 → 执行备份
   - 如果服务未运行：检查是否有重要文件需要备份
   - 如果没有重要文件：跳过备份步骤
4. 🚀 执行Docker服务部署（解压、加载镜像、设置标签）
5. ⚙️  应用端口配置（如果指定了 --port 参数）
6. ▶️  启动Docker服务
7. ⏳ 等待服务启动完成（15秒）
8. ✅ 验证部署结果并报告状态

**智能备份逻辑**：
- **服务运行中**：必须先停止服务，然后执行备份
- **服务未运行**：检查 `docker/` 目录中是否存在重要文件：
  - `docker-compose.yml`、`docker-compose.yaml`
  - `.env` 环境配置文件
  - `data/`、`config/`、`logs/` 目录
- **首次部署**：如果没有发现重要文件，自动跳过备份步骤

**延迟部署功能**：
- 支持三种时间单位：`hours`（小时）、`minutes`（分钟）、`days`（天）
- 默认时间单位为小时
- 提供友好的时间格式化显示
- 延迟期间显示等待状态，延迟结束后自动执行升级部署

**使用场景**：
- 🌙 **深夜升级**：避免影响业务，可在凌晨进行升级部署
- 🚀 **一键升级**：复杂的升级流程简化为单条命令
- 🛡️  **风险降低**：自动备份确保数据安全，失败时可快速回滚
- ⏰ **灵活调度**：支持延迟执行，适应不同的运维时间窗口

## 🎯 使用建议

### 日常运维
```bash
# 检查系统状态（推荐每日执行）
duck-cli status

# 查看缓存使用情况
duck-cli cache status

# 每日自动备份（建议配置系统cron）
0 2 * * * /path/to/duck-cli auto-backup run

# 版本升级（建议先在测试环境验证）
duck-cli auto-upgrade-deploy run

# 计划内的升级部署（如晚上11点后2小时执行）
duck-cli auto-upgrade-deploy delay-time-deploy 2

# 定期清理缓存（保留最新3个版本）
duck-cli cache clean-downloads --keep 3
```

### 应急场景
```bash
# 紧急备份（如果即将进行高风险操作）
duck-cli auto-backup run

# 快速恢复到稳定版本
duck-cli list-backups
duck-cli rollback <backup_id>

# 重启单个有问题的容器
duck-cli docker-service restart-container <容器名称>

# 检查架构信息（用于故障诊断）
duck-cli docker-service arch-info

# 查看所有Docker镜像
duck-cli docker-service list-images
```

### 维护操作
```bash
# 手动解压服务包（用于调试）
duck-cli docker-service extract

# 重新加载Docker镜像
duck-cli docker-service load-images

# 重新设置镜像标签
duck-cli docker-service setup-tags

# 清理所有缓存（释放磁盘空间）
duck-cli cache clear
```

## 📞 获取帮助

- 使用 `duck-cli --help` 查看所有可用命令
- 使用 `duck-cli [command] --help` 查看特定命令的详细帮助
- 使用 `duck-cli --verbose [command]` 获得详细的执行日志 