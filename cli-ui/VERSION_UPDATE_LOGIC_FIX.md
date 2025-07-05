# 版本更新逻辑修复总结

## 问题描述
用户发现版本更新逻辑错误：
- `duck-cli upgrade` 仅是下载服务包，还未部署，不应该更新配置文件中的版本号
- 只有 `duck-cli auto-upgrade-deploy run` 实际部署服务成功后，才应该更新配置版本号

## 修复内容

### 1. 修复 `duck-cli upgrade` (update.rs)
**移除了错误的版本更新逻辑**：
- 强制下载模式：移除版本更新，只显示下载信息
- 智能下载模式：移除版本更新，只显示下载信息
- 增加了版本信息显示，区分"下载版本"和"当前部署版本"

### 2. 保持 `duck-cli auto-upgrade-deploy run` (auto_upgrade_deploy.rs)
**确认版本更新逻辑正确**：
- 只有在Docker服务包成功解压后才更新版本号
- 更新内存中的版本信息
- 持久化保存到config.toml文件

## 修复前后对比

### 修复前（错误）
```
duck-cli upgrade --full    # ❌ 下载后立即更新版本号
duck-cli auto-upgrade-deploy run  # ✅ 部署后也更新版本号
```

### 修复后（正确）
```
duck-cli upgrade --full    # ✅ 仅下载，不更新版本号
duck-cli auto-upgrade-deploy run  # ✅ 部署成功后更新版本号
```

## 版本号含义
- **配置文件中的版本号**: 表示当前已部署的服务版本
- **下载的版本号**: 可能与部署版本不同，仅在部署成功后才同步

## 用户体验改进
- 明确显示"下载版本"和"当前部署版本"
- 提示用户部署成功后将自动更新配置版本号
- 避免了版本不一致导致的下载路径错误

## 验证方法
1. 运行 `duck-cli upgrade --full` - 检查版本号不变
2. 运行 `duck-cli auto-upgrade-deploy run` - 检查版本号更新
3. 验证下载路径使用正确的版本号 