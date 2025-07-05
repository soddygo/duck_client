import { useState, useCallback, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import WorkingDirectoryBar from './components/WorkingDirectoryBar';
import OperationPanel from './components/OperationPanel';
import TerminalWindow from './components/TerminalWindow';
import { LogEntry, DEFAULT_LOG_CONFIG, LogConfig } from './types';
import { ConfigManager, DialogManager, FileSystemManager } from './utils/tauri';
import './App.css';

function App() {
  // 工作目录状态
  const [workingDirectory, setWorkingDirectory] = useState<string | null>(null);
  const [isDirectoryValid, setIsDirectoryValid] = useState(false);
  
  // 日志状态 - 使用循环缓冲区
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [logConfig] = useState<LogConfig>(DEFAULT_LOG_CONFIG);
  const [totalLogCount, setTotalLogCount] = useState(0); // 总日志数量统计
  const [isInitialized, setIsInitialized] = useState(false); // 初始化状态标记
  
  // 当前执行状态
  const [isExecuting, setIsExecuting] = useState(false);
  
  // 使用 useRef 避免循环依赖
  const logsRef = useRef<LogEntry[]>([]);
  const lastLogTimeRef = useRef<number>(0);

  // 同步 logs 状态到 ref
  useEffect(() => {
    logsRef.current = logs;
  }, [logs]);

  // 智能日志管理 - 循环缓冲区实现
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

  // 智能去重逻辑 - 使用 ref 避免依赖 logs 状态
  const shouldSkipDuplicate = useCallback((newMessage: string, newType: LogEntry['type']) => {
    const currentLogs = logsRef.current;
    if (currentLogs.length === 0) return false;
    
    // 检查最近5条日志
    const recentLogs = currentLogs.slice(-5);
    const isDuplicate = recentLogs.some(log => 
      log.message === newMessage && 
      log.type === newType &&
      (Date.now() - parseInt(log.id)) < 1000 // 1秒内的重复（使用ID中的时间戳）
    );
    
    return isDuplicate;
  }, []);

  // 添加日志条目 - 使用循环缓冲区
  const addLogEntry = useCallback((
    type: LogEntry['type'], 
    message: string, 
    command?: string, 
    args?: string[]
  ) => {
    // 过滤空消息
    if (!message.trim() && type !== 'command') return;
    
    // 简单的时间限制去重（避免过于频繁的日志）
    const now = Date.now();
    if (now - lastLogTimeRef.current < 10) { // 10ms 内的重复调用
      return;
    }
    lastLogTimeRef.current = now;
    
    // 智能去重
    if (shouldSkipDuplicate(message, type)) return;
    
    const entry: LogEntry = {
      id: Date.now().toString() + Math.random().toString(36).substr(2, 9),
      timestamp: new Date().toLocaleTimeString(),
      type,
      message,
      command,
      args
    };
    
    // 更新统计
    setTotalLogCount(prev => prev + 1);
    
    // 使用循环缓冲区管理
    manageLogBuffer([entry]);
  }, [shouldSkipDuplicate, manageLogBuffer]);

  // 导出所有日志
  const exportAllLogs = useCallback(async () => {
    try {
      const timestamp = new Date().toISOString().slice(0, 19).replace(/:/g, '-');
      const filename = `duck-cli-logs-${timestamp}.txt`;
      
      const logContent = logs.map(log => {
        const prefix = `[${log.timestamp}] [${log.type.toUpperCase()}]`;
        if (log.type === 'command') {
          return `${prefix} $ ${log.command} ${log.args?.join(' ') || ''}`;
        }
        return `${prefix} ${log.message}`;
      }).join('\n');

      const savedPath = await DialogManager.saveFile('导出日志', filename);
      if (savedPath) {
        const success = await FileSystemManager.writeTextFile(savedPath, logContent);
        if (success) {
          await DialogManager.showMessage('成功', '日志已导出', 'info');
          return true;
        }
      }
      return false;
    } catch (error) {
      console.error('Export logs failed:', error);
      return false;
    }
  }, [logs]);

  // 设置Tauri事件监听器
  useEffect(() => {
    let unlistenOutput: any;
    let unlistenError: any;
    let unlistenComplete: any;

    const setupEventListeners = async () => {
      try {
        // 监听CLI输出事件
        unlistenOutput = await listen('cli-output', (event) => {
          const output = event.payload as string;
          if (output.trim()) {
            // 将输出按行分割并添加到日志
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

        // 监听CLI错误事件
        unlistenError = await listen('cli-error', (event) => {
          const error = event.payload as string;
          if (error.trim()) {
            // 将错误按行分割并添加到日志
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

        // 监听CLI完成事件
        unlistenComplete = await listen('cli-complete', (event) => {
          const exitCode = event.payload as number;
          setIsExecuting(false);
          
          if (exitCode === 0) {
            addLogEntry('success', `命令执行完成 (退出码: ${exitCode})`);
          } else {
            addLogEntry('error', `命令执行失败 (退出码: ${exitCode})`);
          }
          
          // 添加分隔线
          addLogEntry('info', '─'.repeat(50));
        });

        console.log('Tauri事件监听器已设置');
      } catch (error) {
        console.error('设置事件监听器失败:', error);
      }
    };

    setupEventListeners();

    // 清理函数
    return () => {
      if (unlistenOutput) unlistenOutput();
      if (unlistenError) unlistenError();
      if (unlistenComplete) unlistenComplete();
    };
  }, [addLogEntry, manageLogBuffer]);

  // 处理工作目录变化
  const handleDirectoryChange = useCallback((directory: string | null, isValid: boolean) => {
    setWorkingDirectory(directory);
    setIsDirectoryValid(isValid);
    
    // 添加目录变化日志
    if (directory && isValid) {
      addLogEntry('success', `工作目录已设置: ${directory}`);
    } else if (directory && !isValid) {
      addLogEntry('error', `工作目录无效: ${directory}`);
    }
  }, [addLogEntry]);

  // 处理命令执行
  const handleCommandExecute = useCallback((command: string, args: string[]) => {
    addLogEntry('command', '', command, args);
    setIsExecuting(true);
    
    // 添加执行开始标记
    addLogEntry('info', `开始执行: ${command} ${args.join(' ')}`);
  }, [addLogEntry]);

  // 处理日志消息
  const handleLogMessage = useCallback((message: string, type: LogEntry['type']) => {
    addLogEntry(type, message);
  }, [addLogEntry]);

  // 清除日志
  const handleClearLogs = useCallback(() => {
    setLogs([]);
    setTotalLogCount(0);
    addLogEntry('info', '日志已清除');
  }, [addLogEntry]);

  // 应用初始化 - 只执行一次
  useEffect(() => {
    if (isInitialized) return;

    const initializeApp = async () => {
      console.log('开始初始化应用...');
      
      // 使用直接的状态更新避免循环
      const initEntry: LogEntry = {
        id: Date.now().toString() + Math.random().toString(36).substr(2, 9),
        timestamp: new Date().toLocaleTimeString(),
        type: 'info',
        message: '🚀 Duck CLI GUI 已启动'
      };
      
      const configEntry: LogEntry = {
        id: (Date.now() + 1).toString() + Math.random().toString(36).substr(2, 9),
        timestamp: new Date().toLocaleTimeString(),
        type: 'info',
        message: `📊 日志管理: 最大 ${logConfig.maxEntries} 条，自动循环覆盖旧记录`
      };
      
      setLogs([initEntry, configEntry]);
      setTotalLogCount(2);
      
      try {
        // 检查是否已有保存的工作目录
        const savedDirectory = await ConfigManager.getWorkingDirectory();
        
        if (savedDirectory) {
          const dirEntry: LogEntry = {
            id: (Date.now() + 2).toString() + Math.random().toString(36).substr(2, 9),
            timestamp: new Date().toLocaleTimeString(),
            type: 'info',
            message: `📁 加载保存的工作目录: ${savedDirectory}`
          };
          
          setLogs(prev => [...prev, dirEntry]);
          setTotalLogCount(prev => prev + 1);
          setWorkingDirectory(savedDirectory);
        } else {
          const noDirEntry: LogEntry = {
            id: (Date.now() + 3).toString() + Math.random().toString(36).substr(2, 9),
            timestamp: new Date().toLocaleTimeString(),
            type: 'info',
            message: '❓ 未发现保存的工作目录设置'
          };
          
          setLogs(prev => [...prev, noDirEntry]);
          setTotalLogCount(prev => prev + 1);
        }
      } catch (error) {
        console.error('初始化失败:', error);
        
        const errorEntry: LogEntry = {
          id: (Date.now() + 4).toString() + Math.random().toString(36).substr(2, 9),
          timestamp: new Date().toLocaleTimeString(),
          type: 'error',
          message: '❌ 应用初始化失败，请重新设置工作目录'
        };
        
        setLogs(prev => [...prev, errorEntry]);
        setTotalLogCount(prev => prev + 1);
      }
      
      setIsInitialized(true);
      console.log('应用初始化完成');
    };

    initializeApp();
  }, [isInitialized, logConfig.maxEntries]);

  return (
    <div className="h-screen flex flex-col bg-gray-100">
      {/* 顶部工作目录栏 */}
      <WorkingDirectoryBar onDirectoryChange={handleDirectoryChange} />

      {/* 主内容区域 */}
      <div className="flex-1 flex flex-col min-h-0">
        {/* 上半部分：操作面板 */}
        <div className="flex-1 overflow-auto">
          <OperationPanel
            workingDirectory={workingDirectory}
            isDirectoryValid={isDirectoryValid}
            onCommandExecute={handleCommandExecute}
            onLogMessage={handleLogMessage}
          />
        </div>
        
        {/* 下半部分：终端窗口 */}
        <div className="h-80 border-t border-gray-200">
          <TerminalWindow
            logs={logs}
            onClearLogs={handleClearLogs}
            isEnabled={isDirectoryValid}
            totalLogCount={totalLogCount}
            maxLogEntries={logConfig.maxEntries}
            onExportLogs={exportAllLogs}
          />
        </div>
      </div>

      {/* 执行状态指示器 */}
      {isExecuting && (
        <div className="fixed bottom-4 right-4 bg-blue-600 text-white px-4 py-2 rounded-lg shadow-lg flex items-center space-x-2">
          <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
          <span className="text-sm font-medium">正在执行命令...</span>
        </div>
      )}
    </div>
  );
}

export default App;
