// 日志条目类型
export interface LogEntry {
  id: string;
  timestamp: string;
  type: 'info' | 'success' | 'error' | 'warning' | 'command';
  message: string;
  command?: string;
  args?: string[];
}

// 工作目录状态
export interface WorkingDirectoryState {
  path: string | null;
  isValid: boolean;
  validationError?: string;
}

// 应用状态
export interface AppState {
  workingDirectory: WorkingDirectoryState;
  logs: LogEntry[];
  isLoading: boolean;
}

// 命令执行结果
export interface CommandResult {
  success: boolean;
  output: string;
  error?: string;
} 