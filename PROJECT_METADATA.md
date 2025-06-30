# 项目元数据统一管理方案

## 概述

为了避免项目信息分散在多个地方导致的不一致问题，我们实施了一套统一的项目元数据管理方案。

## 设计原则

1. **单一数据源**：所有项目元数据统一定义在 `client-core/src/constants.rs` 中
2. **编译时同步**：使用 `env!("CARGO_PKG_VERSION")` 确保版本号与 Cargo.toml 同步
3. **向后兼容**：保留原有常量，使用 `#[deprecated]` 标记过时项
4. **易于使用**：提供便捷的访问函数和结构体

## 统一管理的信息类型

### 1. 项目基本信息
- 项目名称和完整名称
- 项目描述（简短和详细版本）
- 作者信息
- 许可证信息
- 仓库和文档链接
- 关键词和分类

### 2. 版本信息
- 客户端版本（自动从 Cargo.toml 获取）
- Docker 服务版本
- 最小支持的依赖版本
- API 和数据库架构版本

## 文件结构

```
client-core/src/constants.rs
├── version::metadata        # 项目元数据
│   ├── PROJECT_NAME
│   ├── PROJECT_DESCRIPTION
│   ├── PROJECT_AUTHORS
│   └── ...
└── version::version_info    # 版本信息
    ├── CLIENT_VERSION
    ├── MIN_DOCKER_VERSION
    └── ...
```

## 使用方法

### 1. 在 CLI 中使用

```rust
use client_core::constants::version::{metadata, version_info};

#[derive(Parser)]
#[command(about = metadata::PROJECT_DESCRIPTION)]
#[command(version = version_info::CLIENT_VERSION)]
#[command(author = metadata::PROJECT_AUTHORS)]
pub struct Cli {
    // ...
}
```

### 2. 使用便捷访问函数

```rust
use client_core::project_info;

// 获取完整项目信息
let info = project_info::get_project_info();
println!("项目: {} v{}", info.name, info.version);

// 获取格式化的版本字符串
let version_str = project_info::get_version_string();
println!("{}", version_str); // "Duck Client v0.1.0"

// 获取系统要求
let requirements = project_info::get_system_requirements();
println!("最小 Docker 版本: {}", requirements.min_docker_version);
```

### 3. 在 Cargo.toml 中使用

虽然 Cargo.toml 不能直接使用 Rust 常量，但我们确保了信息的一致性：

```toml
[package]
name = "duck-cli"
description = "Docker 服务管理和升级工具"    # 与 PROJECT_DESCRIPTION 保持一致
authors = ["Duck Team"]                    # 与 PROJECT_AUTHORS 保持一致
# ...
```

## 信息同步策略

### 自动同步
- **版本号**：通过 `env!("CARGO_PKG_VERSION")` 自动从 Cargo.toml 获取
- **包名称**：通过 `env!("CARGO_PKG_NAME")` 自动获取

### 手动同步
- **描述信息**：需要手动确保 Cargo.toml 与常量定义一致
- **作者信息**：需要手动同步
- **许可证信息**：需要手动同步

## 更新流程

### 版本更新
1. 更新相关 `Cargo.toml` 文件中的 `version` 字段
2. `CLIENT_VERSION` 会自动同步（通过 `env!` 宏）
3. 无需手动更新代码中的版本号

### 项目信息更新
1. 修改 `client-core/src/constants.rs` 中的相应常量
2. 如需要，同步更新 `Cargo.toml` 文件
3. 运行测试确保所有引用都正确更新

## 好处

### 1. 一致性保证
- 避免不同地方信息不一致
- 统一的更新入口
- 编译时检查确保引用正确

### 2. 维护便利性
- 集中管理，易于查找和修改
- 明确的更新流程
- 向后兼容，渐进式迁移

### 3. 功能丰富性
- 便捷的访问函数
- 结构化的信息组织
- 支持多种使用场景

## 最佳实践

### 1. 新增信息时
- 优先考虑添加到统一常量中
- 提供便捷的访问函数
- 更新相关文档

### 2. 使用时
- 优先使用新的统一常量
- 避免硬编码项目信息
- 使用 `project_info` 模块的便捷函数

### 3. 废弃旧常量时
- 使用 `#[deprecated]` 标记
- 提供迁移指导
- 保持足够的过渡期

## 迁移指南

### 使用统一常量的方式
```rust
// 直接使用新的统一常量 ✅
use client_core::constants::version::version_info::CLIENT_VERSION;
use client_core::constants::version::metadata::PROJECT_DESCRIPTION;

// 或者使用便捷访问函数 ✅
use client_core::project_info;
let version = project_info::get_project_info().version;
let description = project_info::get_project_info().description;
```

### 添加新的项目信息
```rust
// 1. 在 constants.rs 中添加常量
pub const NEW_INFO: &str = "新信息";

// 2. 在 project_info 模块中添加访问函数
pub fn get_new_info() -> &'static str {
    metadata::NEW_INFO
}

// 3. 更新相关结构体（如需要）
pub struct ProjectInfo {
    // ... 现有字段 ...
    pub new_info: &'static str,
}
```

## 检查清单

### 发布前检查
- [ ] 所有 Cargo.toml 文件的版本号已更新
- [ ] 项目描述在各处保持一致
- [ ] 新增的常量有对应的便捷访问函数
- [ ] 废弃的常量已正确标记
- [ ] 文档已更新

### 代码审查重点
- [ ] 是否使用了统一的项目信息常量
- [ ] 是否避免了硬编码的项目信息
- [ ] 新增信息是否遵循了统一管理原则 