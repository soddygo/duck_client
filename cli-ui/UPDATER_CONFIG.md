# Tauri Updater 插件配置说明

## 🔧 开发环境配置

### 1. 密钥文件位置
- **私钥**: `/Users/soddy/.tauri/test-key.key`
- **公钥**: `/Users/soddy/.tauri/test-key.key.pub`
- **密码**: 空（开发测试用）

### 2. 环境变量配置

为了在开发和构建时使用签名功能，需要设置以下环境变量：

```bash
# 方式一：使用文件路径
export TAURI_SIGNING_PRIVATE_KEY="/Users/soddy/.tauri/test-key.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""

# 方式二：直接使用密钥内容（GitHub Actions 推荐）
export TAURI_PRIVATE_KEY="<base64-encoded-private-key-content>"
export TAURI_KEY_PASSWORD=""
```

### 3. 公钥配置

已在 `tauri.conf.json` 中配置的公钥：
```
dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDhCQzZBODJFNkREQTUyMjIKUldRaVV0cHRMcWpHaTdOOG5VOWNCcThy\ndTBrai9GMTlFRGhPWHRMY0dkWjkvQUh2bFhGZTFFMHgK
```

## 🚀 配置详情

### tauri.conf.json 配置
```json
{
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/soddygo/duck_client/releases/latest/download/latest.json"
      ],
      "dialog": true,
      "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDhCQzZBODJFNkREQTUyMjIKUldRaVV0cHRMcWpHaTdOOG5VOWNCcThy\ndTBrai9GMTlFRGhPWHRMY0dkWjkvQUh2bFhGZTFFMHgK"
    }
  }
}
```

### capabilities 权限配置
```json
{
  "permissions": [
    "updater:default",
    "updater:allow-check",
    "updater:allow-download", 
    "updater:allow-install",
    "updater:allow-download-and-install"
  ]
}
```

## 📋 使用方法

### 开发阶段
1. 当前配置已经可以直接使用
2. 运行 `npm run tauri dev` 正常启动
3. 更新功能会尝试从配置的端点检查更新

### 生产环境准备
1. **替换密钥**: 生成新的生产环境密钥对
   ```bash
   cargo tauri signer generate -w production-key.key
   ```

2. **更新配置**: 
   - 在 `tauri.conf.json` 中更新 `pubkey`
   - 在 CI/CD 中配置 `TAURI_PRIVATE_KEY` 和 `TAURI_KEY_PASSWORD`

3. **更新端点**: 确保更新服务器正确配置并返回有效的更新信息

## 🔐 安全注意事项

⚠️ **重要**: 当前配置的密钥仅用于开发测试！

- 私钥文件位于本地，**不要**提交到版本控制
- 生产环境必须使用安全的密钥管理方案
- GitHub Secrets 适合存储生产环境的私钥
- 定期轮换签名密钥

## 📚 更多信息

- [Tauri Updater 官方文档](https://tauri.app/plugin/updater/)
- [GitHub Actions 构建配置](.github/workflows/cli-ui-build.yml)
- [签名和分发指南](https://tauri.app/distribute/sign/)

## 🧪 测试更新功能

可以通过以下方式测试更新功能：

1. **手动触发检查更新**（在应用中添加按钮）
2. **模拟更新服务器**（本地搭建测试端点）
3. **创建测试发布版本**（GitHub Releases）

---

**提醒**: 正式发布前请务必替换为生产环境的安全密钥！ 