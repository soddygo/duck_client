import { useState, useCallback, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import WorkingDirectoryBar from './components/WorkingDirectoryBar';
import OperationPanel from './components/OperationPanel';
import TerminalWindow from './components/TerminalWindow';
import WelcomeSetupModal from './components/WelcomeSetupModal';
import ErrorBoundary from './components/ErrorBoundary';
import { LogEntry, DEFAULT_LOG_CONFIG, LogConfig } from './types';
import { ConfigManager, DialogManager, DuckCliManager, FileSystemManager, ProcessManager } from './utils/tauri';
import './App.css';

function App() {
  // 工作目录状态
  const [workingDirectory, setWorkingDirectory] = useState<string | null>(null);
  const [isDirectoryValid, setIsDirectoryValid] = useState(false);
  const [showWelcomeModal, setShowWelcomeModal] = useState(false);
  
  // 日志状态 - 使用循环缓冲区
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [logConfig] = useState<LogConfig>(DEFAULT_LOG_CONFIG);
  const [totalLogCount, setTotalLogCount] = useState(0); // 总日志数量统计
  const [isInitialized, setIsInitialized] = useState(false); // 初始化状态标记
  
  // 当前执行状态
  const [isExecuting, setIsExecuting] = useState(false);
  
  // 使用 useRef 避免循环依赖
  const logsRef = useRef<LogEntry[]>([]);

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

  // 轻量级去重逻辑 - 只检查连续重复
  const shouldSkipDuplicate = useCallback((newMessage: string, newType: LogEntry['type']) => {
    const currentLogs = logsRef.current;
    if (currentLogs.length === 0) return false;
    
    // 只检查最后一条日志，避免连续重复（极端情况的保护）
    const lastLog = currentLogs[currentLogs.length - 1];
    return lastLog && 
      lastLog.message === newMessage && 
      lastLog.type === newType;
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
    
    // 只对相同类型的连续消息做去重，移除时间限制
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

  // 设置Tauri事件监听器 - 使用ref避免重复注册
  const addLogEntryRef = useRef(addLogEntry);
  
  // 同步最新的addLogEntry函数到ref
  useEffect(() => {
    addLogEntryRef.current = addLogEntry;
  }, [addLogEntry]);

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
            // 使用ref访问最新的addLogEntry，避免重复注册
            addLogEntryRef.current('info', output.trim());
          }
        });

        // 监听CLI错误事件
        unlistenError = await listen('cli-error', (event) => {
          const error = event.payload as string;
          if (error.trim()) {
            // 使用ref访问最新的addLogEntry，避免重复注册
            addLogEntryRef.current('error', error.trim());
          }
        });

        // 监听CLI完成事件
        unlistenComplete = await listen('cli-complete', (event) => {
          const exitCode = event.payload as number;
          setIsExecuting(false);
          
          if (exitCode === 0) {
            addLogEntryRef.current('success', `命令执行完成 (退出码: ${exitCode})`);
          } else {
            addLogEntryRef.current('error', `命令执行失败 (退出码: ${exitCode})`);
          }
          
          // 添加分隔线
          addLogEntryRef.current('info', '─'.repeat(50));
        });

        console.log('Tauri事件监听器已设置（仅一次）');
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
      console.log('Tauri事件监听器已清理');
    };
  }, []); // ✅ 空依赖数组，确保只注册一次

  // 处理工作目录变化
  const handleDirectoryChange = useCallback(async (directory: string | null, isValid: boolean) => {
    console.log('工作目录变更:', directory, '有效性:', isValid);
    
    const previousDirectory = workingDirectory;
    setWorkingDirectory(directory);
    setIsDirectoryValid(isValid);

    if (directory && isValid && directory !== previousDirectory) {
      // 当工作目录变更且有效时，执行进程检查
      addLogEntry('info', `📁 工作目录已设置: ${directory}`);
      
      try {
        addLogEntry('info', '🔍 检查并清理冲突进程...');
        const checkResult = await ProcessManager.initializeProcessCheck(directory);
        
        if (checkResult.processCleanup.processes_found.length > 0) {
          addLogEntry('warning', `🧹 发现 ${checkResult.processCleanup.processes_found.length} 个冲突进程`);
          addLogEntry('success', `✅ 已清理 ${checkResult.processCleanup.processes_killed.length} 个进程`);
        }
        
        if (checkResult.databaseLocked) {
          addLogEntry('error', '⚠️ 数据库文件仍被锁定，请稍后重试');
          setIsDirectoryValid(false); // 临时禁用功能直到锁定解除
        } else {
          addLogEntry('success', checkResult.message);
        }
      } catch (error) {
        console.error('进程检查失败:', error);
        addLogEntry('error', `❌ 进程检查失败: ${error}`);
      }
    }

    // 根据是否需要显示欢迎界面
    if (!directory || !isValid) {
      setShowWelcomeModal(true);
    } else {
      setShowWelcomeModal(false);
    }
  }, [workingDirectory, addLogEntry]);

  // 处理命令执行
  const handleCommandExecute = useCallback(async (command: string, args: string[]) => {
    addLogEntry('command', '', command, args);
    setIsExecuting(true);
    
    // 添加执行开始标记
    addLogEntry('info', `🚀 开始执行: ${command} ${args.join(' ')}`);
    
    try {
      // 真正执行Tauri命令，会触发事件监听器接收实时输出
      if (command === 'duck-cli' && workingDirectory) {
        await DuckCliManager.executeSmart(args, workingDirectory);
      }
    } catch (error) {
      addLogEntry('error', `❌ 命令执行失败: ${error}`);
    }
    // 注意：setIsExecuting(false) 会在事件监听器的 cli-complete 事件中处理
  }, [addLogEntry, workingDirectory]);

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
          // 验证保存的目录
          const validation = await FileSystemManager.validateDirectory(savedDirectory);
          await handleDirectoryChange(savedDirectory, validation.valid);
        } else {
          setShowWelcomeModal(true);
        }
      } catch (error) {
        console.error('初始化失败:', error);
        setShowWelcomeModal(true);
      }
      
      setIsInitialized(true);
      console.log('应用初始化完成');
    };

    initializeApp();
  }, [isInitialized, logConfig.maxEntries, handleDirectoryChange]);

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

      {/* 欢迎设置弹窗 */}
      {showWelcomeModal && (
        <WelcomeSetupModal
          isOpen={showWelcomeModal}
          onComplete={async (directory: string) => {
            // 验证目录
            const validation = await FileSystemManager.validateDirectory(directory);
            await handleDirectoryChange(directory, validation.valid);
            setShowWelcomeModal(false);
          }}
          onSkip={() => setShowWelcomeModal(false)}
        />
      )}
    </div>
  );
}

export default function AppWithErrorBoundary() {
  return (
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  );
}
