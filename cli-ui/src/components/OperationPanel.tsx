import React, { useState } from 'react';
import {
  PlayIcon,
  StopIcon,
  ArrowPathIcon,
  RocketLaunchIcon,
  CloudArrowDownIcon,
  ArrowUpTrayIcon,
  WrenchScrewdriverIcon,
  DocumentDuplicateIcon,
  BackwardIcon,
  CheckBadgeIcon,
  Cog6ToothIcon
} from '@heroicons/react/24/outline';
import { DuckCliManager, UpdateManager, DialogManager } from '../utils/tauri';

interface OperationPanelProps {
  workingDirectory: string | null;
  isDirectoryValid: boolean;
  onCommandExecute: (command: string, args: string[]) => void;
  onLogMessage: (message: string, type: 'info' | 'success' | 'error' | 'warning') => void;
}

interface ActionButton {
  id: string;
  title: string;
  description: string;
  icon: React.ReactNode;
  action: () => Promise<void>;
  variant: 'primary' | 'secondary' | 'success' | 'warning' | 'danger';
  disabled?: boolean;
}

const OperationPanel: React.FC<OperationPanelProps> = ({
  workingDirectory,
  isDirectoryValid,
  onCommandExecute,
  onLogMessage
}) => {
  const [executingActions, setExecutingActions] = useState<Set<string>>(new Set());

  // 检查是否禁用（工作目录无效）
  const isDisabled = !workingDirectory || !isDirectoryValid;

  // 执行操作的包装函数
  const executeAction = async (actionId: string, actionFn: () => Promise<void>) => {
    if (isDisabled) {
      await DialogManager.showMessage('警告', '请先设置有效的工作目录', 'warning');
      return;
    }

    setExecutingActions(prev => new Set(prev).add(actionId));
    
    try {
      await actionFn();
    } catch (error) {
      onLogMessage(`操作失败: ${error}`, 'error');
    } finally {
      setExecutingActions(prev => {
        const newSet = new Set(prev);
        newSet.delete(actionId);
        return newSet;
      });
    }
  };

  // 定义所有操作按钮
  const actionButtons: ActionButton[] = [
    {
      id: 'init',
      title: '初始化',
      description: '初始化 Duck CLI 项目',
      icon: <RocketLaunchIcon className="h-5 w-5" />,
      variant: 'primary',
      action: async () => {
        onLogMessage('开始初始化项目...', 'info');
        onCommandExecute('duck-cli', ['init']);
        
        const result = await DuckCliManager.initialize(workingDirectory!);
        if (result.success) {
          onLogMessage('项目初始化成功', 'success');
        } else {
          onLogMessage(`初始化失败: ${result.error}`, 'error');
        }
      }
    },
    {
      id: 'download',
      title: '下载服务',
      description: '下载 Docker 服务镜像',
      icon: <CloudArrowDownIcon className="h-5 w-5" />,
      variant: 'secondary',
      action: async () => {
        onLogMessage('开始下载服务...', 'info');
        onCommandExecute('duck-cli', ['upgrade', '--full']);
        
        const result = await DuckCliManager.upgradeService(workingDirectory!, true);
        if (result.success) {
          onLogMessage('服务下载完成', 'success');
        } else {
          onLogMessage(`下载失败: ${result.error}`, 'error');
        }
      }
    },
    {
      id: 'deploy',
      title: '一键部署',
      description: '自动升级并部署服务',
      icon: <ArrowUpTrayIcon className="h-5 w-5" />,
      variant: 'primary',
      action: async () => {
        onLogMessage('开始一键部署...', 'info');
        onCommandExecute('duck-cli', ['auto-upgrade-deploy', 'run']);
        
        const result = await DuckCliManager.autoUpgradeDeploy(workingDirectory!);
        if (result.success) {
          onLogMessage('部署完成', 'success');
        } else {
          onLogMessage(`部署失败: ${result.error}`, 'error');
        }
      }
    },
    {
      id: 'start',
      title: '启动服务',
      description: '启动 Docker 服务',
      icon: <PlayIcon className="h-5 w-5" />,
      variant: 'success',
      action: async () => {
        onLogMessage('启动服务...', 'info');
        onCommandExecute('duck-cli', ['docker-service', 'start']);
        
        const result = await DuckCliManager.startService(workingDirectory!);
        if (result.success) {
          onLogMessage('服务启动成功', 'success');
        } else {
          onLogMessage(`启动失败: ${result.error}`, 'error');
        }
      }
    },
    {
      id: 'stop',
      title: '停止服务',
      description: '停止 Docker 服务',
      icon: <StopIcon className="h-5 w-5" />,
      variant: 'warning',
      action: async () => {
        onLogMessage('停止服务...', 'info');
        onCommandExecute('duck-cli', ['docker-service', 'stop']);
        
        const result = await DuckCliManager.stopService(workingDirectory!);
        if (result.success) {
          onLogMessage('服务已停止', 'success');
        } else {
          onLogMessage(`停止失败: ${result.error}`, 'error');
        }
      }
    },
    {
      id: 'restart',
      title: '重启服务',
      description: '重启 Docker 服务',
      icon: <ArrowPathIcon className="h-5 w-5" />,
      variant: 'secondary',
      action: async () => {
        onLogMessage('重启服务...', 'info');
        onCommandExecute('duck-cli', ['docker-service', 'restart']);
        
        const result = await DuckCliManager.restartService(workingDirectory!);
        if (result.success) {
          onLogMessage('服务重启成功', 'success');
        } else {
          onLogMessage(`重启失败: ${result.error}`, 'error');
        }
      }
    },
    {
      id: 'check-update',
      title: '检查更新',
      description: '检查服务更新',
      icon: <CheckBadgeIcon className="h-5 w-5" />,
      variant: 'secondary',
      action: async () => {
        onLogMessage('检查更新...', 'info');
        onCommandExecute('duck-cli', ['check-update', 'check']);
        
        const result = await DuckCliManager.checkCliUpdate(workingDirectory!);
        if (result.success) {
          onLogMessage('更新检查完成', 'success');
        } else {
          onLogMessage(`检查失败: ${result.error}`, 'error');
        }
      }
    },
    {
      id: 'upgrade',
      title: '升级服务',
      description: '升级 Docker 服务',
      icon: <WrenchScrewdriverIcon className="h-5 w-5" />,
      variant: 'primary',
      action: async () => {
        onLogMessage('升级服务...', 'info');
        onCommandExecute('duck-cli', ['upgrade']);
        
        const result = await DuckCliManager.upgradeService(workingDirectory!);
        if (result.success) {
          onLogMessage('服务升级完成', 'success');
        } else {
          onLogMessage(`升级失败: ${result.error}`, 'error');
        }
      }
    },
    {
      id: 'backup',
      title: '创建备份',
      description: '创建服务备份',
      icon: <DocumentDuplicateIcon className="h-5 w-5" />,
      variant: 'secondary',
      action: async () => {
        onLogMessage('创建备份...', 'info');
        onCommandExecute('duck-cli', ['backup', 'create']);
        
        const result = await DuckCliManager.createBackup(workingDirectory!);
        if (result.success) {
          onLogMessage('备份创建成功', 'success');
        } else {
          onLogMessage(`备份失败: ${result.error}`, 'error');
        }
      }
    },
    {
      id: 'rollback',
      title: '回滚服务',
      description: '回滚到上一个版本',
      icon: <BackwardIcon className="h-5 w-5" />,
      variant: 'warning',
      action: async () => {
        const confirmed = await DialogManager.confirmAction(
          '确认回滚',
          '确定要回滚到上一个版本吗？此操作不可逆。'
        );
        
        if (confirmed) {
          onLogMessage('回滚服务...', 'info');
          onCommandExecute('duck-cli', ['backup', 'restore', '--latest']);
          
          const result = await DuckCliManager.rollbackService(workingDirectory!);
          if (result.success) {
            onLogMessage('服务回滚成功', 'success');
          } else {
            onLogMessage(`回滚失败: ${result.error}`, 'error');
          }
        }
      }
    },
    {
      id: 'app-update',
      title: '应用更新',
      description: '检查并更新 GUI 应用',
      icon: <Cog6ToothIcon className="h-5 w-5" />,
      variant: 'primary',
      action: async () => {
        onLogMessage('检查应用更新...', 'info');
        
        try {
          const update = await UpdateManager.checkForUpdates();
          if (update) {
            const confirmed = await DialogManager.confirmAction(
              '发现新版本',
              `发现新版本 ${update.version}，是否立即更新？`
            );
            
            if (confirmed) {
              onLogMessage('下载并安装更新...', 'info');
              await UpdateManager.downloadAndInstallUpdate((downloaded, total) => {
                const progress = ((downloaded / total) * 100).toFixed(1);
                onLogMessage(`下载进度: ${progress}%`, 'info');
              });
              onLogMessage('更新完成，应用即将重启', 'success');
            }
          } else {
            onLogMessage('已是最新版本', 'info');
          }
        } catch (error) {
          onLogMessage(`更新检查失败: ${error}`, 'error');
        }
      }
    }
  ];

  // 获取按钮样式
  const getButtonStyle = (variant: string, disabled: boolean, executing: boolean) => {
    const baseClasses = "relative inline-flex items-center px-4 py-3 border text-sm font-medium rounded-lg focus:outline-none focus:ring-2 focus:ring-offset-2 transition-all duration-200 min-h-[3rem]";
    
    if (disabled) {
      return `${baseClasses} border-gray-200 text-gray-400 bg-gray-50 cursor-not-allowed`;
    }
    
    if (executing) {
      return `${baseClasses} border-blue-300 text-blue-700 bg-blue-50 cursor-wait`;
    }

    switch (variant) {
      case 'primary':
        return `${baseClasses} border-blue-300 text-blue-700 bg-blue-50 hover:bg-blue-100 focus:ring-blue-500`;
      case 'success':
        return `${baseClasses} border-green-300 text-green-700 bg-green-50 hover:bg-green-100 focus:ring-green-500`;
      case 'warning':
        return `${baseClasses} border-yellow-300 text-yellow-700 bg-yellow-50 hover:bg-yellow-100 focus:ring-yellow-500`;
      case 'danger':
        return `${baseClasses} border-red-300 text-red-700 bg-red-50 hover:bg-red-100 focus:ring-red-500`;
      default:
        return `${baseClasses} border-gray-300 text-gray-700 bg-gray-50 hover:bg-gray-100 focus:ring-gray-500`;
    }
  };

  return (
    <div className="bg-white p-6">
      <div className="mb-4">
        <h2 className="text-lg font-semibold text-gray-900">操作面板</h2>
        <p className="text-sm text-gray-600 mt-1">
          {isDisabled ? '请先设置有效的工作目录' : '选择要执行的操作'}
        </p>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
        {actionButtons.map((button) => {
          const isExecuting = executingActions.has(button.id);
          const isButtonDisabled = isDisabled || isExecuting;

          return (
            <button
              key={button.id}
              onClick={() => executeAction(button.id, button.action)}
              disabled={isButtonDisabled}
              className={getButtonStyle(button.variant, isButtonDisabled, isExecuting)}
              title={button.description}
            >
              <div className="flex flex-col items-center text-center w-full">
                <div className="mb-2 relative">
                  {isExecuting ? (
                    <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-current"></div>
                  ) : (
                    button.icon
                  )}
                </div>
                <span className="text-xs font-medium">{button.title}</span>
              </div>
            </button>
          );
        })}
      </div>

      {/* 状态提示 */}
      {isDisabled && (
        <div className="mt-4 p-3 bg-yellow-50 border border-yellow-200 rounded-md">
          <div className="flex">
            <div className="flex-shrink-0">
              <svg className="h-5 w-5 text-yellow-400" viewBox="0 0 20 20" fill="currentColor">
                <path fillRule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
              </svg>
            </div>
            <div className="ml-3">
              <p className="text-sm text-yellow-800">
                工作目录未设置或无效，所有操作已禁用。请在顶部选择有效的工作目录。
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default OperationPanel; 