import React, { useState, useCallback } from 'react';
import WorkingDirectoryBar from './components/WorkingDirectoryBar';
import OperationPanel from './components/OperationPanel';
import TerminalWindow from './components/TerminalWindow';
import { LogEntry } from './types';
import './App.css';

function App() {
  // 工作目录状态
  const [workingDirectory, setWorkingDirectory] = useState<string | null>(null);
  const [isDirectoryValid, setIsDirectoryValid] = useState(false);
  
  // 日志状态
  const [logs, setLogs] = useState<LogEntry[]>([]);

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
    </div>
  );
}

export default App;
