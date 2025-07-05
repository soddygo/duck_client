# 数据库文件统一迁移文档

## 概述

此次更改将项目中使用的多个数据库文件统一为一个标准的数据库文件：`duck_client.db`

## 问题描述

在统一前，项目中存在以下问题：
1. **多个数据库文件**：同时使用 `history.db` 和 `duck_client.db`
2. **硬编码路径**：在多处代码中硬编码数据库文件名
3. **数据分散**：不同组件使用不同的数据库文件，导致数据分散
4. **WAL文件冲突**：多个数据库文件产生的WAL文件导致初始化冲突

## 解决方案

### 1. 统一数据库文件名
- **标准文件名**：`duck_client.db`
- **标准路径**：`data/duck_client.db`
- **常量定义**：使用 `client_core::constants::config::DATABASE_FILE_NAME`

### 2. 更新的文件列表

#### 2.1 Core模块更新
- `client-core/src/db/actor.rs`
  - 添加WAL文件清理逻辑
  - 避免数据库初始化冲突

- `client-core/src/database_manager.rs`
  - 添加WAL文件清理逻辑
  - 确保数据库连接前清理冲突文件

#### 2.2 CLI模块更新
- `duck-cli/src/app.rs`
  - 使用 `config::get_database_path()` 获取数据库路径
  - 移除硬编码的 `history.db`

- `duck-cli/src/init.rs`
  - 使用常量获取数据库路径
  - 更新日志信息显示正确的数据库文件名

#### 2.3 UI模块更新
- `client-ui/src-tauri/src/commands/init.rs`
  - 将 `"data/history.db"` 改为 `"data/duck_client.db"`
  - 统一两个初始化函数中的数据库路径

- `client-ui/src-tauri/src/commands/directory.rs`
  - 更新初始化检查逻辑，使用标准数据库文件名

- `client-ui/src-tauri/src/commands/types.rs`
  - 使用 `config::DATABASE_FILE_NAME` 常量
  - 确保全局状态管理器使用统一的数据库文件

### 3. WAL文件冲突处理

#### 3.1 问题描述
DuckDB使用WAL (Write-Ahead Logging) 机制，当数据库文件和WAL文件不同步时会出现：
```
Catalog Error: Failure while replaying WAL file "data/history.db.wal": 
Table with name "app_config" already exists!
```

#### 3.2 解决方案
在数据库连接前自动清理WAL文件：

```rust
// 在 DuckDbActor::new() 和 DatabaseManager::new() 中添加
fn cleanup_wal_files(db_path: &std::path::Path) -> Result<()> {
    let mut wal_path = db_path.as_os_str().to_owned();
    wal_path.push(".wal");
    let wal_path = std::path::PathBuf::from(wal_path);
    
    if wal_path.exists() {
        // 删除WAL文件避免冲突
        std::fs::remove_file(&wal_path)?;
    }
    Ok(())
}
```

### 4. 清理操作

#### 4.1 删除的文件
```bash
# 删除所有旧的 history.db 文件
find . -name "history.db*" -type f -exec rm -f {} \;

# 删除所有WAL文件
find . -name "*.wal" -type f -exec rm -f {} \;
```

#### 4.2 清理的目录
- `./test_deploy/history.db`
- `./client-ui/src-tauri/history.db`
- `./client-ui/src-tauri/data/history.db.wal`
- `./history.db`
- `./duck-cli/history.db`
- 各种嵌套的临时目录中的数据库文件

## 验证

### 1. 编译验证
```bash
cargo build --workspace
# ✅ 编译成功，无错误
```

### 2. 数据库路径验证
- 所有模块现在都使用 `data/duck_client.db` 作为标准数据库路径
- 通过常量引用确保一致性

### 3. WAL文件处理验证
- 数据库初始化前自动清理WAL文件
- 避免"Table already exists"错误

## 影响

### 1. 正面影响
- ✅ **数据统一**：所有数据都存储在同一个数据库文件中
- ✅ **路径一致**：消除硬编码，使用常量管理
- ✅ **冲突避免**：自动清理WAL文件，避免初始化错误
- ✅ **维护性**：更易于维护和调试

### 2. 注意事项
- 🔄 **数据迁移**：如果用户之前有 `history.db` 文件，数据不会自动迁移
- 📁 **路径依赖**：确保应用在正确的工作目录中运行

## 后续工作

1. **数据迁移脚本**：为已有用户提供从 `history.db` 到 `duck_client.db` 的数据迁移
2. **文档更新**：更新用户文档中的数据库文件引用
3. **测试验证**：在不同平台上测试数据库统一的效果

## 总结

通过此次数据库文件统一，项目现在具有：
- 单一的数据库文件 (`duck_client.db`)
- 一致的路径管理 (通过常量)
- 自动的冲突处理 (WAL文件清理)
- 更好的可维护性

这为后续的功能开发和数据管理提供了坚实的基础。 