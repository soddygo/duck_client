c# 版本同步修复说明

## 问题背景

用户报告了版本不一致的问题：
- **配置文件版本**: `1.0.0` (在 `config.toml` 中)
- **常量定义版本**: `0.0.1` (在 `constants.rs` 中)
- **服务器API返回版本**: `0.0.1`
- **下载路径**: 使用了错误的版本构建路径

## 问题分析

1. **根本原因**: 下载成功后，配置文件中的版本号没有同步更新为服务器返回的版本
2. **影响**: 导致下载路径不一致，用户困惑
3. **场景**: 在直接调用 `duck-cli upgrade` 时出现，而 `auto-upgrade-deploy` 有正确的版本更新逻辑

## 修复内容

### 1. 版本同步机制 ✅
在 `duck-cli/src/commands/update.rs` 中添加了版本同步逻辑：

```rust
// 📝 更新配置文件中的Docker服务版本
if target_version != &app.config.versions.docker_service {
    info!(
        "📝 更新Docker服务版本: {} -> {}",
        app.config.versions.docker_service, target_version
    );

    // 更新内存中的版本信息
    app.config.versions.docker_service = target_version.clone();

    // 持久化到配置文件
    match app.config.save_to_file("config.toml") {
        Ok(_) => {
            info!("✅ 配置文件版本号已更新并保存");
        }
        Err(e) => {
            warn!("⚠️ 保存配置文件失败: {}", e);
            warn!("   版本号已在内存中更新，但配置文件未同步");
        }
    }
}
```

**应用位置**:
- ✅ 智能下载模式（`download_service_update_optimized`）
- ✅ 强制下载模式（`--force` 参数）

### 2. 双模式下载支持 ✅
基于服务器返回的 `download_method` 字段实现两种下载方式：

#### API接口下载 (`"api"`)
- 使用 `/api/v1/clients/downloads/docker/services/full/latest?version=x.x.x`
- 需要认证客户端
- 适合私有服务器部署

#### 直接URL下载 (`"direct"`)  
- 使用服务器返回的外部URL（如OSS云存储）
- 不需要认证
- 适合CDN加速和大文件下载

**实现细节**:
```rust
// 根据下载方式构建下载URL
let download_url = match manifest.packages.full.download_method.as_str() {
    "direct" => {
        // 直接使用服务器返回的URL（OSS等外部存储）
        info!("📥 使用直接下载方式 (外部存储)");
        manifest.packages.full.url.clone()
    }
    "api" | _ => {
        // 使用API接口下载（默认方式）
        info!("📥 使用API接口下载方式");
        let mut url = self.config.get_endpoint_url(&self.config.endpoints.docker_download_full);
        if let Some(v) = version {
            url = format!("{url}?version={v}");
        }
        url
    }
};

// 根据下载方式决定是否使用认证
let use_auth = manifest.packages.full.download_method != "direct";
```

### 3. 认证优化 ✅
添加了认证控制逻辑，避免对外部URL进行不必要的认证：

**新增方法**:
- `download_with_progress_internal()` - 内部实现，支持认证控制
- `download_service_update_from_url_with_auth()` - 支持认证控制的URL下载

**认证逻辑**:
```rust
let response = if use_auth && self.authenticated_client.is_some() {
    // 使用认证客户端（API下载）
    let auth_client = self.authenticated_client.as_ref().unwrap();
    // ... 认证下载逻辑
} else {
    // 使用普通客户端（直接URL下载）
    info!("使用普通HTTP客户端下载");
    self.build_request(url).send().await?
};
```

## 服务器端配置

为了支持双模式下载，服务器API响应需要包含 `download_method` 字段：

```json
{
  "version": "0.0.1",
  "release_date": "2025-07-04T23:15:46Z",
  "release_notes": "This is a test release for the new workflow.",
  "packages": {
    "full": {
      "url": "http://127.0.0.1:3000/api/v1/downloads/full/0.0.1",
      "hash": "9c34bedc6e5d84512db677aa68790ddd03a07993d49b0f64a6298dfd3d667da7",
      "signature": "",
      "size": 10342623178,
      "download_method": "api"  // 或 "direct"
    },
    "patch": null
  }
}
```

**建议配置**:
- `"api"`: 用于内部API下载
- `"direct"`: 用于OSS/CDN直接下载，提供完整的外部URL

## 测试验证

### 场景1: API接口下载
```bash
# 服务器返回: download_method = "api"
duck-cli upgrade --full

# 预期行为:
# 1. 使用API接口下载: /api/v1/clients/downloads/docker/services/full/latest?version=0.0.1
# 2. 使用认证客户端
# 3. 下载完成后同步更新配置文件版本: 1.0.0 -> 0.0.1
```

### 场景2: 直接URL下载
```bash
# 服务器返回: download_method = "direct", url = "https://oss.example.com/docker-0.0.1.zip"
duck-cli upgrade --full

# 预期行为:
# 1. 直接下载外部URL
# 2. 不使用认证客户端
# 3. 下载完成后同步更新配置文件版本
```

## 影响和收益

### ✅ 解决的问题
1. **版本一致性**: 配置文件版本与服务器版本保持同步
2. **下载路径正确**: 使用正确的版本构建下载路径
3. **双模式支持**: 支持API和直接URL两种下载方式
4. **认证优化**: 避免对外部URL进行不必要的认证

### ✅ 用户体验改进
1. **版本透明**: 用户可以清楚看到版本更新过程
2. **错误减少**: 消除版本不一致导致的混淆
3. **下载稳定**: 根据下载方式选择最优的下载策略
4. **日志清晰**: 详细的下载方式和版本更新日志

### ✅ 系统健壮性
1. **向后兼容**: 默认使用API下载方式，保持兼容性
2. **错误恢复**: 认证失败时自动降级到普通下载
3. **配置同步**: 内存和文件配置双重同步
4. **多场景支持**: 覆盖强制下载和智能下载两种模式

## 后续优化建议

1. **配置验证**: 添加启动时的版本一致性检查
2. **回滚机制**: 在版本更新失败时回滚到之前版本
3. **缓存管理**: 自动清理过期版本的下载缓存
4. **监控告警**: 添加版本不一致的监控和告警机制

---

**修复完成时间**: 2025-07-05  
**影响版本**: v1.0.10+  
**测试状态**: ✅ 编译通过，功能验证中 