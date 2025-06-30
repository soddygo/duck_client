# Ducker 更新指南

本文档说明如何更新集成在 duck-cli 中的第三方 ducker 项目。

## 当前状态

- **ducker 源码位置**: `third-party/ducker/`
- **上游仓库**: https://github.com/robertpsoane/ducker
- **集成方式**: Git 子目录，完整源码集成

## 更新方式

### 方式1：使用自动化脚本（推荐）

#### Linux/macOS
```bash
# 更新到最新版本
./scripts/update-ducker.sh

# 更新到指定版本
./scripts/update-ducker.sh v0.3.2
```

#### Windows
```cmd
# 更新到最新版本
scripts\update-ducker.bat

# 更新到指定版本
scripts\update-ducker.bat v0.3.2
```

### 方式2：手动更新

```bash
# 1. 进入ducker目录
cd third-party/ducker

# 2. 获取最新信息
git fetch origin

# 3. 查看可用版本
git tag --sort=-version:refname | head -10

# 4. 更新（选择一种方式）

# 更新到最新开发版本
git checkout master
git pull origin master

# 或者更新到指定稳定版本
git checkout v0.3.1

# 5. 回到项目根目录
cd ../../

# 6. 验证编译
cargo check -p duck-cli
```

### 方式3：重新克隆（彻底更新）

如果遇到冲突或问题，可以完全重新下载：

```bash
# 1. 删除现有目录
rm -rf third-party/ducker

# 2. 重新克隆
cd third-party
git clone https://github.com/robertpsoane/ducker.git

# 3. 可选：切换到特定版本
cd ducker
git checkout v0.3.1

# 4. 回到根目录验证
cd ../../
cargo check -p duck-cli
```

## 版本选择建议

### 稳定版本（推荐）
使用带版本号的 git 标签，如 `v0.3.1`：
- ✅ 经过测试，稳定可靠
- ✅ 版本明确，便于追踪
- ✅ 适合生产环境

### 开发版本
使用 `master` 分支：
- ⚠️ 可能包含未发布的功能
- ⚠️ 可能存在不稳定因素
- ✅ 可以获得最新功能和修复

## 更新后的验证

更新完成后，建议进行以下验证：

```bash
# 1. 检查编译
cargo check -p duck-cli

# 2. 测试基本功能
cargo run -p duck-cli -- ducker --help

# 3. 测试TUI界面（需要Docker环境）
cargo run -p duck-cli -- ducker
```

## 兼容性说明

### ducker 版本兼容性
- **推荐版本**: v0.3.x 系列
- **最低要求**: v0.3.0+
- **测试版本**: v0.3.1

### 依赖冲突处理
如果更新后出现依赖冲突：

1. **检查 Cargo.toml**: 确保版本兼容
2. **更新依赖锁定**: `cargo update`
3. **清理重建**: `cargo clean && cargo build`

### 已知问题和解决方案

#### 问题1: tracing subscriber 冲突
**症状**: `SetGlobalDefaultError("a global default trace dispatcher has already been set")`
**解决**: 已在代码中处理，跳过重复初始化

#### 问题2: 编译错误
**症状**: 新版本ducker引入不兼容的依赖
**解决**: 
1. 检查 `duck-cli/Cargo.toml` 中的 ducker 依赖版本
2. 必要时调整或锁定相关依赖版本

## 更新记录

| 日期 | ducker 版本 | 操作 | 备注 |
|------|-------------|------|------|
| 2025-06-29 | v0.3.1 | 初始集成 | 完整功能集成成功 |

## 获取帮助

如果更新过程中遇到问题：

1. **查看错误日志**: 仔细阅读编译错误信息
2. **检查上游变更**: 访问 [ducker 仓库](https://github.com/robertpsoane/ducker) 查看变更日志
3. **回滚版本**: 如果新版本有问题，回滚到之前的稳定版本
4. **提交Issue**: 在项目中记录兼容性问题

## 自动化建议

对于团队开发，建议：

1. **固定版本**: 在 CI/CD 中使用特定的 ducker 版本
2. **定期更新**: 建立定期检查和更新流程
3. **测试覆盖**: 更新后进行完整的功能测试 