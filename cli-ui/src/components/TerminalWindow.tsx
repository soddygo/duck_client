import React, { useState, useEffect, useRef } from 'react';
import { 
  CommandLineIcon, 
  TrashIcon,
  DocumentTextIcon,
  ArrowDownTrayIcon
} from '@heroicons/react/24/outline';
import { FileSystemManager, DialogManager } from '../utils/tauri';

interface LogEntry {
  id: string;
  timestamp: string;
  type: 'info' | 'success' | 'error' | 'warning' | 'command';
  message: string;
  command?: string;
  args?: string[];
}

interface TerminalWindowProps {
  logs: LogEntry[];
  onClearLogs: () => void;
  isEnabled: boolean;
}

const TerminalWindow: React.FC<TerminalWindowProps> = ({ 
  logs, 
  onClearLogs, 
  isEnabled 
}) => {
  const [autoScroll, setAutoScroll] = useState(true);
  const logsEndRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // 自动滚动到底部
  useEffect(() => {
    if (autoScroll && logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [logs, autoScroll]);

  // 检测用户是否手动滚动
  const handleScroll = () => {
    if (containerRef.current) {
      const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
      const isAtBottom = scrollTop + clientHeight >= scrollHeight - 10;
      setAutoScroll(isAtBottom);
    }
  };

  // 导出日志
  const exportLogs = async () => {
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
        } else {
          await DialogManager.showMessage('错误', '日志导出失败', 'error');
        }
      }
    } catch (error) {
      console.error('Export logs failed:', error);
      await DialogManager.showMessage('错误', '导出失败', 'error');
    }
  };

  // 获取日志条目样式
  const getLogEntryStyle = (type: string) => {
    const baseClasses = "flex items-start space-x-2 py-1 px-2 rounded text-xs font-mono";
    
    switch (type) {
      case 'command':
        return `${baseClasses} bg-gray-100 border-l-4 border-blue-400`;
      case 'success':
        return `${baseClasses} text-green-700`;
      case 'error':
        return `${baseClasses} text-red-700 bg-red-50`;
      case 'warning':
        return `${baseClasses} text-yellow-700 bg-yellow-50`;
      default:
        return `${baseClasses} text-gray-700`;
    }
  };

  // 获取类型图标
  const getTypeIcon = (type: string) => {
    switch (type) {
      case 'command':
        return <CommandLineIcon className="h-3 w-3 text-blue-500 mt-0.5 flex-shrink-0" />;
      case 'success':
        return <div className="h-3 w-3 bg-green-500 rounded-full mt-0.5 flex-shrink-0"></div>;
      case 'error':
        return <div className="h-3 w-3 bg-red-500 rounded-full mt-0.5 flex-shrink-0"></div>;
      case 'warning':
        return <div className="h-3 w-3 bg-yellow-500 rounded-full mt-0.5 flex-shrink-0"></div>;
      default:
        return <div className="h-3 w-3 bg-gray-400 rounded-full mt-0.5 flex-shrink-0"></div>;
    }
  };

  return (
    <div className="bg-white border-t border-gray-200 flex flex-col h-full">
      {/* 终端标题栏 */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-gray-200 bg-gray-50">
        <div className="flex items-center space-x-2">
          <CommandLineIcon className="h-4 w-4 text-gray-500" />
          <span className="text-sm font-medium text-gray-700">终端输出</span>
          {!isEnabled && (
            <span className="text-xs text-yellow-600 bg-yellow-100 px-2 py-1 rounded">
              工作目录无效
            </span>
          )}
          <span className="text-xs text-gray-500">
            ({logs.length} 条记录)
          </span>
        </div>

        <div className="flex items-center space-x-2">
          {/* 自动滚动开关 */}
          <label className="flex items-center space-x-1 text-xs text-gray-600">
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => setAutoScroll(e.target.checked)}
              className="h-3 w-3 text-blue-600 rounded"
            />
            <span>自动滚动</span>
          </label>

          {/* 导出日志 */}
          <button
            onClick={exportLogs}
            disabled={logs.length === 0}
            className="p-1 text-gray-500 hover:text-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
            title="导出日志"
          >
            <ArrowDownTrayIcon className="h-4 w-4" />
          </button>

          {/* 清除日志 */}
          <button
            onClick={onClearLogs}
            disabled={logs.length === 0}
            className="p-1 text-gray-500 hover:text-red-600 disabled:opacity-50 disabled:cursor-not-allowed"
            title="清除日志"
          >
            <TrashIcon className="h-4 w-4" />
          </button>
        </div>
      </div>

      {/* 终端内容区域 */}
      <div 
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto p-4 bg-gray-900 text-green-400 font-mono text-sm"
        style={{ minHeight: '300px' }}
      >
        {logs.length === 0 ? (
          <div className="flex items-center justify-center h-full text-gray-500">
            <div className="text-center">
              <DocumentTextIcon className="h-12 w-12 mx-auto mb-2 opacity-50" />
              <p className="text-sm">暂无日志信息</p>
              <p className="text-xs mt-1">执行操作后会在此显示输出</p>
            </div>
          </div>
        ) : (
          <div className="space-y-1">
            {logs.map((log) => (
              <div key={log.id} className="flex items-start space-x-2">
                <span className="text-gray-500 text-xs flex-shrink-0 mt-0.5">
                  {log.timestamp}
                </span>
                <div className="flex-1 min-w-0">
                  {log.type === 'command' ? (
                    <div className="text-blue-400">
                      <span className="text-gray-500">$</span> {log.command} {log.args?.join(' ')}
                    </div>
                  ) : (
                    <div className={
                      log.type === 'error' ? 'text-red-400' :
                      log.type === 'success' ? 'text-green-400' :
                      log.type === 'warning' ? 'text-yellow-400' :
                      'text-gray-300'
                    }>
                      <span className="mr-2">
                        {log.type === 'error' ? '✗' :
                         log.type === 'success' ? '✓' :
                         log.type === 'warning' ? '⚠' : 'ℹ'}
                      </span>
                      {log.message}
                    </div>
                  )}
                </div>
              </div>
            ))}
            <div ref={logsEndRef} />
          </div>
        )}
      </div>

      {/* 状态栏 */}
      <div className="px-4 py-2 bg-gray-50 border-t border-gray-200">
        <div className="flex items-center justify-between text-xs text-gray-500">
          <div className="flex items-center space-x-4">
            <span>就绪</span>
            {!autoScroll && (
              <span className="text-orange-500">● 手动滚动模式</span>
            )}
          </div>
          <div className="flex items-center space-x-2">
            <span>终端</span>
            <div className={`h-2 w-2 rounded-full ${isEnabled ? 'bg-green-500' : 'bg-red-500'}`}></div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default TerminalWindow; 