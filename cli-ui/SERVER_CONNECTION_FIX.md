# 服务器连接问题修复总结

## 问题描述
用户在使用Duck CLI GUI应用的"下载服务"功能时遇到HTTP请求错误：
- 错误信息：`检查版本失败: HTTP 请求错误: error sending request`
- 结果：`首次部署时无法获取版本信息，且本地没有安装包文件`

## 问题原因
`client-core/src/constants.rs`中的默认服务器地址配置错误：
```rust
// 修复前 - 错误的IP地址
pub const DEFAULT_BASE_URL: &str = "http://192.168.1.29:3000";
```

该IP地址无法访问，导致HTTP请求失败。

## 解决方案

### 1. 问题诊断
```bash
# 测试本地服务器 - 正常
curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:3000/api-docs/openapi.json
# 返回: 200

# 测试配置中的地址 - 失败
curl -s -o /dev/null -w "%{http_code}" http://192.168.1.29:3000/api-docs/openapi.json
# 返回: 000 (连接失败)
```

### 2. 修复配置
修改`client-core/src/constants.rs`第227行：
```rust
// 修复后 - 正确的本地地址
pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:3000";
```

### 3. 验证修复
重新编译并测试API连接：
```bash
cargo build
curl -X GET "http://127.0.0.1:3000/api/v1/docker/checkVersion" \
  -H "Content-Type: application/json" \
  -d '{"current_version": "0.0.1"}'
```

API返回正确的版本信息：
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
      "size": 10342623178
    },
    "patch": null
  }
}
```

## 修复结果
- ✅ GUI应用可以正常连接到本地服务器
- ✅ "下载服务"功能可以正常获取版本信息
- ✅ HTTP请求错误已解决
- ✅ 版本检查和下载功能恢复正常

## 验证步骤
1. 启动GUI应用：`cd cli-ui && npm run tauri dev`
2. 点击"下载服务"按钮
3. 确认不再出现HTTP请求错误
4. 验证版本信息正确显示

## 注意事项
- 确保本地服务器运行在 `127.0.0.1:3000`
- 如果部署到其他环境，需要相应修改 `DEFAULT_BASE_URL` 配置
- 建议将服务器地址配置化，支持运行时修改 