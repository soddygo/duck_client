# 日志性能优化

## 问题描述
用户报告点击一键部署后，虽然日志输出很快，但感觉执行特别慢。分析发现是日志处理逻辑导致的性能问题。

## 问题原因
1. **过度严格的时间限制**：10ms 内的重复调用被跳过，导致快速输出的日志丢失
2. **过度复杂的去重逻辑**：检查最近5条日志，每次都要遍历比较
3. **过度精细的行处理**：每行输出都单独处理，导致大量的函数调用
4. **时间戳解析开销**：每次去重都要解析 ID 中的时间戳

## 优化方案
1. **移除时间限制**：删除 10ms 的时间限制，允许快速连续的日志输出
2. **简化去重逻辑**：只检查最后一条日志，避免连续重复即可
3. **批量处理**：整块输出而不是逐行处理，减少函数调用次数
4. **移除时间戳比较**：简化为纯文本和类型比较

## 修改内容

### 1. 简化 addLogEntry 函数
```typescript
// 之前：复杂的时间限制和去重
const now = Date.now();
if (now - lastLogTimeRef.current < 10) {
  return;
}
lastLogTimeRef.current = now;

// 之后：简化为仅去重检查
if (shouldSkipDuplicate(message, type)) return;
```

### 2. 优化去重逻辑
```typescript
// 之前：检查最近5条日志 + 时间戳比较
const recentLogs = currentLogs.slice(-5);
const isDuplicate = recentLogs.some(log => 
  log.message === newMessage && 
  log.type === newType &&
  (Date.now() - parseInt(log.id)) < 1000
);

// 之后：仅检查最后一条日志
const lastLog = currentLogs[currentLogs.length - 1];
const isDuplicate = lastLog && 
  lastLog.message === newMessage && 
  lastLog.type === newType;
```

### 3. 批量处理输出
```typescript
// 之前：逐行处理
const lines = output.split('\n').filter(line => line.trim());
lines.forEach(line => {
  addLogEntry('info', line);
});

// 之后：整块处理
addLogEntry('info', output.trim());
```

## 性能改进
1. **减少函数调用**：从 N 行 × 多次检查 → 1 次调用 × 1 次检查
2. **降低 CPU 使用**：移除时间戳解析和数组遍历
3. **减少内存操作**：避免多次数组切片和过滤
4. **提升响应速度**：去除不必要的延迟限制

## 测试验证
- 点击一键部署，日志输出应该更流畅
- 连续相同的日志仍会被过滤（避免重复）
- 不同类型的日志正常显示
- 性能明显改善，无卡顿感

## 日期
2024-07-05 - 日志性能优化完成 