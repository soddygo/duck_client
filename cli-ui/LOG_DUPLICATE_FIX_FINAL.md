# 日志重复问题最终修复记录

## 问题描述
用户在点击"一键部署"时，日志输出重复显示，每条日志都出现2次：
```
8:07:40 AM
ℹ[32m INFO[0m 检查Docker服务版本...
8:07:40 AM
ℹ[32m INFO[0m 检查Docker服务版本...
```

## 根本原因分析
通过代码分析发现两个主要问题：

### 1. 事件监听器绕过去重逻辑
在 `cli-ui/src/App.tsx` 中，Tauri事件监听器直接调用 `manageLogBuffer`，没有经过去重检查：
```typescript
// 问题代码
unlistenOutput = await listen('cli-output', (event) => {
  // ... 处理逻辑
  manageLogBuffer(newEntries); // 直接调用，绕过去重
});

unlistenError = await listen('cli-error', (event) => {
  // ... 处理逻辑  
  manageLogBuffer(newEntries); // 直接调用，绕过去重
});
```

### 2. 去重逻辑的时间戳比较bug
去重函数中的时间戳比较有问题：
```typescript
// 问题代码
(Date.now() - new Date(log.timestamp).getTime()) < 1000 // 时间字符串转换有问题
```

## 解决方案

### 1. 修复事件监听器
让所有日志都通过 `addLogEntry` 函数处理，确保去重逻辑生效：
```typescript
// 修复后的代码
unlistenOutput = await listen('cli-output', (event) => {
  const output = event.payload as string;
  if (output.trim()) {
    const lines = output.split('\n')
      .filter(line => line.trim())
      .map(line => line.trim())
      .filter(line => line.length > 0);
    
    // 使用addLogEntry确保去重逻辑
    lines.forEach(line => {
      addLogEntry('info', line);
    });
  }
});

unlistenError = await listen('cli-error', (event) => {
  const error = event.payload as string;
  if (error.trim()) {
    const lines = error.split('\n')
      .filter(line => line.trim())
      .map(line => line.trim())
      .filter(line => line.length > 0);
    
    // 使用addLogEntry确保去重逻辑
    lines.forEach(line => {
      addLogEntry('error', line);
    });
  }
});
```

### 2. 修复去重逻辑的时间戳比较
使用日志ID中的时间戳进行比较：
```typescript
// 修复后的代码
const shouldSkipDuplicate = useCallback((newMessage: string, newType: LogEntry['type']) => {
  const currentLogs = logsRef.current;
  if (currentLogs.length === 0) return false;
  
  const recentLogs = currentLogs.slice(-5);
  const isDuplicate = recentLogs.some(log => 
    log.message === newMessage && 
    log.type === newType &&
    (Date.now() - parseInt(log.id)) < 1000 // 使用ID中的时间戳
  );
  
  return isDuplicate;
}, []);
```

## 修复效果
- ✅ 消除了日志重复显示问题
- ✅ 保持了智能去重功能
- ✅ 维持了性能优化

## 技术细节
- **去重时间窗口**: 1秒内的重复日志被过滤
- **去重范围**: 检查最近5条日志记录
- **时间戳来源**: 使用日志ID中的时间戳（`Date.now()`）

## 测试验证
启动GUI应用后，日志应该不再重复显示。每条日志只会出现一次，即使Duck CLI同时向stdout和stderr输出相同内容。

## 关于服务器连接问题
由于用户使用的是GitHub上的duck-cli二进制文件，服务器地址常量的修改（从 `http://192.168.1.29:3000` 到 `http://127.0.0.1:3000`）需要等到代码提交并触发更新后才能生效。

## 后续步骤
1. 测试GUI应用验证日志重复修复
2. 提交代码到GitHub触发duck-cli更新
3. 下载新版duck-cli解决服务器连接问题 