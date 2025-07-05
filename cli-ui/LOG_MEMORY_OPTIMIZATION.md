# Duck CLI GUI 日志内存优化

## 问题描述

原始的日志系统使用普通数组无限增长，长时间运行会导致内存占用过高，可能引起：
- 内存泄漏
- 应用卡顿
- 浏览器崩溃

## 解决方案

### 1. 循环缓冲区设计

实现了智能的循环缓冲区系统：
- **最大容量**: 100000 条日志记录
- **清理策略**: 超出容量时自动清理最早的 10000 条记录
- **保留策略**: 保留最新的 90000 条记录

### 2. 配置参数

```typescript
// 日志管理配置
export interface LogConfig {
  maxEntries: number;        // 最大日志条目数
  trimBatchSize: number;     // 一次清理的数量
}

// 默认配置
export const DEFAULT_LOG_CONFIG: LogConfig = {
  maxEntries: 100000,        // 最多保留100000条日志
  trimBatchSize: 10000,      // 超出时一次清理10000条
};
```

### 3. 核心实现

#### 循环缓冲区管理
```typescript
const manageLogBuffer = useCallback((newLogs: LogEntry[]) => {
  setLogs(currentLogs => {
    const allLogs = [...currentLogs, ...newLogs];
    
    // 检查是否需要清理
    if (allLogs.length > logConfig.maxEntries) {
      const excessCount = allLogs.length - logConfig.maxEntries;
      const trimCount = Math.max(excessCount, logConfig.trimBatchSize);
      
      // 保留最新的日志条目
      const trimmedLogs = allLogs.slice(trimCount);
      
      console.log(`日志缓冲区清理: 删除 ${trimCount} 条旧记录, 保留 ${trimmedLogs.length} 条`);
      
      return trimmedLogs;
    }
    
    return allLogs;
  });
}, [logConfig.maxEntries, logConfig.trimBatchSize]);
```

#### 批量处理优化
```typescript
// 批量添加日志以提高性能
const newEntries = lines.map(line => ({
  id: Date.now().toString() + Math.random().toString(36).substr(2, 9),
  timestamp: new Date().toLocaleTimeString(),
  type: 'info' as const,
  message: line
}));

if (newEntries.length > 0) {
  setTotalLogCount(prev => prev + newEntries.length);
  manageLogBuffer(newEntries);
}
```

### 4. 性能监控

#### 内存使用状态
- 实时显示当前日志条目数
- 显示总累计日志数量
- 缓冲区使用率百分比显示：
  - 绿色：< 70%
  - 黄色：70-90%
  - 红色：> 90%

#### 界面显示
```typescript
// 日志统计信息
<div className="flex items-center space-x-2 text-xs text-gray-500">
  <span className="flex items-center space-x-1">
    <ChartBarIcon className="h-3 w-3" />
    <span>显示: {currentLogs}</span>
  </span>
  <span>总计: {totalLogCount}</span>
  <span className={`px-2 py-1 rounded ${
    percentage > 90 ? 'bg-red-100 text-red-700' :
    percentage > 70 ? 'bg-yellow-100 text-yellow-700' :
    'bg-green-100 text-green-700'
  }`}>
    缓冲区: {percentage}%
  </span>
</div>
```

### 5. 优化特性

#### 智能去重
- 检查最近5条日志避免重复显示
- 1秒内的相同消息自动过滤
- 空消息自动过滤

#### 静默清理
- 自动清理过程不显示用户提示
- 后台静默完成，不影响用户体验
- 控制台日志记录清理统计

#### 导出功能保留
- 手动导出功能仍然可用
- 支持导出当前缓冲区中的所有日志
- 自动生成时间戳文件名

### 6. 内存效益

#### 预期效果
- **内存占用**: 控制在合理范围内（约 10-20MB）
- **性能稳定**: 长时间运行不会导致内存泄漏
- **用户体验**: 界面响应保持流畅

#### 技术指标
- 单条日志约 200-500 字节
- 100000 条日志约 20-50MB 内存占用
- 自动清理确保内存不会无限增长

### 7. 使用场景

#### 适用场景
- 长时间运行的开发环境
- 大量日志输出的操作
- 连续多次命令执行
- 服务器长期监控

#### 清理触发条件
- 日志数量超过 100000 条
- 内存使用率达到设定阈值
- 用户手动清空日志

### 8. 配置建议

#### 不同使用场景的配置
```typescript
// 开发环境（频繁操作）
const DEV_CONFIG: LogConfig = {
  maxEntries: 50000,
  trimBatchSize: 5000,
};

// 生产环境（长期运行）
const PROD_CONFIG: LogConfig = {
  maxEntries: 100000,
  trimBatchSize: 10000,
};

// 轻量级环境（资源受限）
const LIGHT_CONFIG: LogConfig = {
  maxEntries: 20000,
  trimBatchSize: 2000,
};
```

## 总结

通过实施循环缓冲区策略，Duck CLI GUI 现在具备了：
- 稳定的内存管理
- 自动化的清理机制
- 透明的用户体验
- 高效的性能表现

这种设计确保了应用在长时间运行或处理大量日志时仍能保持稳定和响应性。 