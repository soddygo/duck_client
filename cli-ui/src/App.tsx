import React, { useState, useCallback, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import WorkingDirectoryBar from './components/WorkingDirectoryBar';
import OperationPanel from './components/OperationPanel';
import TerminalWindow from './components/TerminalWindow';
import WelcomeSetupModal from './components/WelcomeSetupModal';
import { LogEntry } from './types';
import { ConfigManager } from './utils/tauri';
import './App.css';

function App() {
  // 工作目录状态
  const [workingDirectory, setWorkingDirectory] = useState<string | null>(null);
  const [isDirectoryValid, setIsDirectoryValid] = useState(false);
  
  // 日志状态
  const [logs, setLogs] = useState<LogEntry[]>([]);
  
  // 当前执行状态
  const [isExecuting, setIsExecuting] = useState(false);

  // 首次使用引导状态
  const [showWelcomeModal, setShowWelcomeModal] = useState(false);
  const [isInitialLoad, setIsInitialLoad] = useState(true);

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
            const lines = output.split('\n').filter(line => line.trim());
            lines.forEach(line => {
              addLogEntry('info', line.trim());
            });
          }
        });

        // 监听CLI错误事件
        unlistenError = await listen('cli-error', (event) => {
          const error = event.payload as string;
          if (error.trim()) {
            // 将错误按行分割并添加到日志
            const lines = error.split('\n').filter(line => line.trim());
            lines.forEach(line => {
              addLogEntry('error', line.trim());
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
        addLogEntry('error', `事件监听器设置失败: ${error}`);
      }
    };

    setupEventListeners();

    // 清理函数
    return () => {
      if (unlistenOutput) unlistenOutput();
      if (unlistenError) unlistenError();
      if (unlistenComplete) unlistenComplete();
    };
  }, []);

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
  }, []);

  // 添加日志条目
  const addLogEntry = useCallback((
    type: LogEntry['type'], 
    message: string, 
    command?: string, 
    args?: string[]
  ) => {
    const entry: LogEntry = {
      id: Date.now().toString() + Math.random().toString(36).substr(2, 9),
      timestamp: new Date().toLocaleTimeString(),
      type,
      message,
      command,
      args
    };
    
    setLogs(prev => [...prev, entry]);
  }, []);

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
    addLogEntry('info', '日志已清除');
  }, [addLogEntry]);

  // 处理欢迎弹窗完成
  const handleWelcomeComplete = useCallback((directory: string) => {
    setWorkingDirectory(directory);
    setShowWelcomeModal(false);
    addLogEntry('success', `工作目录设置完成: ${directory}`);
    addLogEntry('info', '现在可以开始使用 Duck CLI 功能了');
  }, [addLogEntry]);

  // 处理欢迎弹窗跳过
  const handleWelcomeSkip = useCallback(() => {
    setShowWelcomeModal(false);
    addLogEntry('warning', '已跳过工作目录设置');
    addLogEntry('info', '您可以随时点击顶部的"选择目录"按钮进行设置');
  }, [addLogEntry]);

  // 应用初始化
  useEffect(() => {
    const initializeApp = async () => {
      addLogEntry('info', 'Duck CLI GUI 已启动');
      
      try {
        // 检查是否已有保存的工作目录
        const savedDirectory = await ConfigManager.getWorkingDirectory();
        
        if (savedDirectory) {
          addLogEntry('info', `加载保存的工作目录: ${savedDirectory}`);
          setWorkingDirectory(savedDirectory);
          // 工作目录验证会在 WorkingDirectoryBar 组件中进行
        } else {
          addLogEntry('info', '未发现保存的工作目录设置');
          setShowWelcomeModal(true);
        }
      } catch (error) {
        console.error('初始化失败:', error);
        addLogEntry('error', '应用初始化失败，请重新设置工作目录');
        setShowWelcomeModal(true);
      } finally {
        setIsInitialLoad(false);
      }
    };

    initializeApp();
  }, [addLogEntry]);

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

      {/* 首次使用引导弹窗 */}
      <WelcomeSetupModal
        isOpen={showWelcomeModal}
        onComplete={handleWelcomeComplete}
        onSkip={handleWelcomeSkip}
      />
    </div>
  );
}

export default App;
