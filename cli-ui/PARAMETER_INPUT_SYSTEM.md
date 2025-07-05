# 参数输入系统

## 功能概述
为 Duck CLI GUI 添加了通用的参数输入系统，支持所有 duck-cli 命令的可选参数输入。

## 核心特性

### 1. 智能参数检测
- 自动检测命令是否有可选参数
- 有参数的命令会弹出参数输入对话框
- 无参数的命令直接执行

### 2. 多种参数类型支持
- **文本类型** (text): 普通文本输入
- **数字类型** (number): 数字输入，支持最小/最大值验证
- **布尔类型** (boolean): 复选框
- **选择类型** (select): 下拉选择
- **多选类型** (multiselect): 多项选择

### 3. 参数验证
- 必填验证
- 数字范围验证
- 正则表达式验证
- 实时错误提示

## 支持的命令

### 一键部署 (auto-upgrade-deploy)
- **端口参数**: 指定前端服务端口号
- 默认值: 80
- 范围: 1-65535

### 升级服务 (upgrade)
- **全量下载**: 下载完整服务包
- **强制重新下载**: 强制覆盖现有文件
- **仅检查版本**: 只检查不下载

### 初始化 (init)
- **强制覆盖**: 覆盖现有配置文件

### 检查更新 (check-update)
- **操作类型**: 检查更新 / 安装版本
- **版本号**: 指定安装版本
- **强制重新安装**: 强制重装当前版本

### 回滚服务 (rollback)
- **备份ID**: 要恢复的备份ID
- **强制覆盖**: 强制覆盖现有文件

### 重启容器 (restart-container)
- **容器名称**: 要重启的容器名称

### 解压服务包 (extract)
- **服务包文件**: 指定zip文件路径
- **目标版本**: 指定目标版本

### 清理下载缓存 (clean-downloads)
- **保留版本数**: 保留的版本数量

### Ducker (ducker)
- **命令参数**: 传递给ducker的参数

## 使用流程

### 1. 点击按钮
用户点击任何操作按钮

### 2. 参数检测
系统检查该命令是否有可选参数：
- **有参数**: 弹出参数输入对话框
- **无参数**: 直接执行命令

### 3. 参数输入
用户在对话框中设置参数：
- 填写必填参数
- 选择可选参数
- 查看使用示例

### 4. 验证执行
系统验证参数后执行命令：
- 参数验证
- 构建命令行
- 执行命令

## 技术实现

### 组件架构
```
ParameterInputModal (参数输入对话框)
├── commandConfigs.ts (命令配置)
├── types/index.ts (类型定义)
└── OperationPanel.tsx (集成逻辑)
```

### 参数构建
将用户输入的参数转换为命令行格式：
```typescript
// 用户输入: { port: 8080, force: true }
// 转换为: ['--port', '8080', '--force']
```

### 命令执行
```typescript
// 基础命令: ['auto-upgrade-deploy', 'run']
// 添加参数: ['auto-upgrade-deploy', 'run', '--port', '8080']
```

## 扩展方式

### 添加新命令参数
在 `commandConfigs.ts` 中添加配置：

```typescript
'new-command': {
  id: 'new-command',
  name: '新命令',
  description: '命令描述',
  parameters: [
    {
      name: 'param1',
      label: '参数1',
      type: 'text',
      required: true,
      placeholder: '请输入...'
    }
  ]
}
```

### 为按钮添加参数支持
在 `OperationPanel.tsx` 中：

```typescript
{
  id: 'new-button',
  commandId: 'new-command', // 对应配置ID
  action: async (parameters?: ParameterInputResult) => {
    const args = parameters ? buildCommandArgs(baseArgs, parameters) : baseArgs;
    // 执行逻辑
  }
}
```

## 用户体验

### 直观的界面
- 清晰的参数标签和说明
- 实时验证反馈
- 使用示例展示

### 智能默认值
- 合理的默认参数值
- 常用配置的快速选择

### 错误处理
- 友好的错误提示
- 参数验证信息
- 取消和重新输入支持

## 开发测试

### 测试用例
1. **一键部署**: 测试端口参数输入
2. **升级服务**: 测试多个布尔参数
3. **检查更新**: 测试选择和文本参数
4. **无参数命令**: 确认直接执行

### 验证要点
- 参数正确传递到命令行
- 用户体验流畅
- 错误处理完善
- 所有参数类型正常工作

## 日期
2024-07-05 - 参数输入系统实现完成 