// 日志条目类型
export interface LogEntry {
  id: string;
  timestamp: string;
  type: 'info' | 'success' | 'error' | 'warning' | 'command';
  message: string;
  command?: string;
  args?: string[];
}

// 日志管理配置
export interface LogConfig {
  maxEntries: number;        // 最大日志条目数
  trimBatchSize: number;     // 一次清理的数量
}

// 默认日志配置
export const DEFAULT_LOG_CONFIG: LogConfig = {
  maxEntries: 100000,        // 最多保留100000条日志
  trimBatchSize: 10000,      // 超出时一次清理10000条（保留90000条）
};

// 工作目录状态
export interface WorkingDirectoryState {
  path: string | null;
  isValid: boolean;
  isChecking: boolean;
  error?: string;
}

// 应用状态
export interface AppState {
  workingDirectory: WorkingDirectoryState;
  logs: LogEntry[];
  isLoading: boolean;
  logConfig: LogConfig;
}

// 命令执行结果
export interface CommandResult {
  success: boolean;
  output: string;
  error?: string;
} 