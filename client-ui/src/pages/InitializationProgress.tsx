import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { 
  InitStage, 
  InitProgress,
  InitProgressEvent,
  InitCompletedEvent,
  DownloadProgressEvent,
  DownloadCompletedEvent
} from '../types/index.ts';
import { globalEventManager } from '../utils/tauri.ts';

interface InitializationProgressProps {
  onComplete: () => void;
  onBack: () => void;
}

export function InitializationProgress({ onComplete, onBack }: InitializationProgressProps) {
  const [currentStage, setCurrentStage] = useState<InitStage>('init');
  const [stageProgress, setStageProgress] = useState<number>(0);
  const [overallProgress, setOverallProgress] = useState<number>(0);
  const [currentStep, setCurrentStep] = useState<number>(1);
  const [totalSteps, setTotalSteps] = useState<number>(4);
  const [message, setMessage] = useState<string>('正在准备初始化...');
  const [taskId, setTaskId] = useState<string>('');
  
  // 控制状态
  const [isBackground, setIsBackground] = useState<boolean>(false);
  const [isCompleted, setIsCompleted] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [isInitializing, setIsInitializing] = useState<boolean>(false);
  
  // 详细信息状态
  const [showDetails, setShowDetails] = useState<boolean>(false);
  const [logMessages, setLogMessages] = useState<string[]>([]);
  const [showLogs, setShowLogs] = useState<boolean>(false);

  // 启动初始化流程
  useEffect(() => {
    startInitializationFlow();
    
    // 清理函数
    return () => {
      globalEventManager.cleanup();
    };
  }, []);

  // 启动初始化流程
  const startInitializationFlow = async () => {
    try {
      // 获取当前工作目录
      const appState = await invoke<any>('get_app_state');
      const workingDir = appState.working_directory;
      
      if (!workingDir) {
        setError('工作目录未设置');
        return;
      }

      addLogMessage('🚀 开始初始化 Duck Client...');
      addLogMessage(`📁 工作目录: ${workingDir}`);
      
      // 第一步：快速本地初始化
      await performLocalInitialization();
      
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      setError(`启动初始化失败: ${errorMessage}`);
      addLogMessage(`❌ 启动初始化失败: ${errorMessage}`);
    }
  };

  // 第一步：快速本地初始化
  const performLocalInitialization = async () => {
    setIsInitializing(true);
    setCurrentStage('init');
    setCurrentStep(1);
    setMessage('正在创建配置文件和数据库...');
    addLogMessage('⚙️ 开始本地初始化...');
    
    try {
      // 模拟进度更新
      for (let i = 0; i <= 100; i += 20) {
        setStageProgress(i);
        setOverallProgress(i / 2); // 第一步占总进度的50%
        await new Promise(resolve => setTimeout(resolve, 100));
      }
      
      // 调用快速本地初始化
      const result = await invoke<string>('init_client_with_progress');
      
      setStageProgress(100);
      setOverallProgress(50);
      addLogMessage('✅ 本地初始化完成');
      addLogMessage('📦 准备下载和部署服务...');
      
      // 等待一下让用户看到第一步完成
      await new Promise(resolve => setTimeout(resolve, 500));
      
      // 继续第二步
      await performServiceDeployment();
      
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      setError(`本地初始化失败: ${errorMessage}`);
      addLogMessage(`❌ 本地初始化失败: ${errorMessage}`);
      setIsInitializing(false);
    }
  };

  // 第二步：下载和部署服务
  const performServiceDeployment = async () => {
    setCurrentStage('deploy');
    setCurrentStep(2);
    setMessage('正在下载和部署 Docker 服务...');
    setStageProgress(0);
    addLogMessage('🚀 开始下载和部署服务...');
    
    try {
      // 设置事件监听器
      const setupEventListeners = async () => {
        // 监听初始化进度
        await globalEventManager.onInitProgress((event: InitProgressEvent) => {
          console.log('收到初始化进度事件:', event);
          
          // 更新当前阶段和步骤
          setCurrentStage(event.stage as InitStage);
          setCurrentStep(event.current_step);
          setTotalSteps(event.total_steps);
          
          // 更新进度
          setStageProgress(event.percentage);
          setOverallProgress(event.percentage);
          setMessage(event.message);
          
          // 添加日志信息
          addLogMessage(`[${event.stage}] ${event.message}`);
        });

        // 监听下载进度
        await globalEventManager.onDownloadProgress((event: DownloadProgressEvent) => {
          console.log('收到下载进度事件:', event);
          
          // 更新下载进度
          setStageProgress(event.percentage);
          setOverallProgress(50 + (event.percentage / 2)); // 第二步占总进度的50%
          
          // 更新消息显示更详细的下载信息
          const downloadSpeed = (event.download_speed / 1024 / 1024).toFixed(1); // MB/s
          const downloadedMB = (event.downloaded_bytes / 1024 / 1024).toFixed(1);
          const totalMB = (event.total_bytes / 1024 / 1024).toFixed(1);
          const etaMinutes = Math.floor(event.eta_seconds / 60);
          const etaSeconds = event.eta_seconds % 60;
          
          setMessage(`正在下载 ${event.file_name}... ${downloadedMB}/${totalMB} MB (${downloadSpeed} MB/s, 剩余 ${etaMinutes}:${etaSeconds.toString().padStart(2, '0')})`);
          
          // 添加日志信息
          addLogMessage(`📦 下载进度: ${event.percentage.toFixed(1)}% - ${event.file_name}`);
        });

        // 监听下载完成
        await globalEventManager.onDownloadCompleted((event: DownloadCompletedEvent) => {
          console.log('收到下载完成事件:', event);
          
          if (event.success) {
            addLogMessage('✅ 下载完成，开始部署服务...');
            setMessage('下载完成，正在部署服务...');
          } else {
            setError(event.error || '下载失败');
            addLogMessage(`❌ 下载失败: ${event.error || '未知错误'}`);
            setIsInitializing(false);
          }
        });

        // 监听初始化完成
        await globalEventManager.onInitCompleted((event: InitCompletedEvent) => {
          console.log('收到初始化完成事件:', event);
          
          if (event.success) {
            setStageProgress(100);
            setOverallProgress(100);
            setIsCompleted(true);
            setMessage('初始化完成！');
            addLogMessage('🎉 服务部署完成');
            addLogMessage('✅ Duck Client 初始化成功');
          } else {
            setError(event.error || '服务部署失败');
            addLogMessage(`❌ 服务部署失败: ${event.error || '未知错误'}`);
          }
          setIsInitializing(false);
        });
      };
      
      // 先设置事件监听器
      await setupEventListeners();
      
      // 调用真实的服务部署函数
      await invoke<string>('download_and_deploy_services');
      
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      setError(`服务部署失败: ${errorMessage}`);
      addLogMessage(`❌ 服务部署失败: ${errorMessage}`);
      setIsInitializing(false);
    }
  };

  // 添加日志消息
  const addLogMessage = (message: string) => {
    const timestamp = new Date().toLocaleTimeString();
    setLogMessages(prev => [...prev.slice(-50), `[${timestamp}] ${message}`]);
  };

  // 取消初始化
  const cancelInitialization = async () => {
    try {
      if (taskId) {
        await invoke('cancel_task', { taskId: taskId });
        addLogMessage('❌ 初始化已取消');
      }
    } catch (error) {
      console.error('取消初始化失败:', error);
      addLogMessage('⚠️ 取消初始化失败');
    }
  };

  // 后台下载模式
  const toggleBackgroundMode = () => {
    setIsBackground(!isBackground);
    addLogMessage(isBackground ? '🔄 切换到前台模式' : '�� 切换到后台运行');
  };

  // 获取阶段信息
  const getStageInfo = (stage: InitStage) => {
    const stageInfoMap: Record<InitStage, { title: string; description: string; icon: string }> = {
      init: {
        title: '第 1 步 / 共 4 步：本地初始化',
        description: '正在创建配置文件和初始化数据库',
        icon: '⚙️'
      },
      initializing: {
        title: '第 1 步 / 共 4 步：版本检查',
        description: '正在检查最新服务版本',
        icon: '🔍'
      },
      download: {
        title: '第 2 步 / 共 4 步：下载服务包',
        description: '正在下载 Docker 服务包',
        icon: '📥'
      },
      downloading: {
        title: '第 2 步 / 共 4 步：下载服务包',
        description: '正在下载 Docker 服务包',
        icon: '📥'
      },
      extracting: {
        title: '第 3 步 / 共 4 步：解压服务包',
        description: '正在解压 Docker 服务包',
        icon: '📦'
      },
      deploy: {
        title: '第 4 步 / 共 4 步：部署服务',
        description: '正在部署和启动服务容器',
        icon: '🚀'
      },
      deploying: {
        title: '第 4 步 / 共 4 步：部署服务',
        description: '正在部署和启动服务容器',
        icon: '🚀'
      },
      loading: {
        title: '第 4 步 / 共 4 步：启动服务',
        description: '正在启动服务容器',
        icon: '🔄'
      },
      starting: {
        title: '第 4 步 / 共 4 步：启动服务',
        description: '正在启动服务容器',
        icon: '▶️'
      },
      configuring: {
        title: '第 4 步 / 共 4 步：配置服务',
        description: '正在配置服务参数',
        icon: '⚙️'
      }
    };
    
    return stageInfoMap[stage] || {
      title: '正在初始化...',
      description: '正在准备初始化系统',
      icon: '⏳'
    };
  };

  // 重试初始化
  const retryInitialization = async () => {
    // 重置所有状态
    setError(null);
    setIsCompleted(false);
    setIsInitializing(false);
    setCurrentStage('init');
    setStageProgress(0);
    setOverallProgress(0);
    setCurrentStep(1);
    setMessage('正在准备重新初始化...');
    setLogMessages([]);
    
    // 添加重试日志
    addLogMessage('🔄 开始重试初始化...');
    
    // 等待一下确保状态更新
    await new Promise(resolve => setTimeout(resolve, 500));
    
    // 重新开始初始化流程
    await startInitializationFlow();
  };

  // 后台模式最小化显示
  if (isBackground) {
    return (
      <div className="background-progress">
        <div className="mini-progress-bar">
          <div className="progress-header">
            <span className="title">🦆 Duck Client - 后台初始化中</span>
            <div className="progress-info">
              <div className="progress-line">
                <div 
                  className="progress-fill" 
                  style={{ width: `${overallProgress}%` }}
                ></div>
              </div>
              <span className="progress-text">
                {overallProgress.toFixed(0)}% | 步骤 {currentStep}/{totalSteps}
              </span>
            </div>
          </div>
          <div className="mini-actions">
            <button onClick={toggleBackgroundMode} className="btn-mini">
              📋 查看详情
            </button>
            <button onClick={cancelInitialization} className="btn-mini danger">
              ❌ 取消
            </button>
          </div>
        </div>
      </div>
    );
  }

  const stageInfo = getStageInfo(currentStage);

  return (
    <div className="w-full h-screen bg-gradient-to-br from-blue-400 via-purple-500 to-purple-600 flex justify-center items-start p-5 overflow-auto">
      <div className="max-w-4xl w-full bg-white/95 backdrop-blur-md rounded-3xl p-10 shadow-2xl mt-5">
        {/* 标题部分 */}
        <div className="text-center mb-8">
          <h1 className="text-4xl font-bold mb-4 bg-gradient-to-r from-blue-500 to-purple-600 bg-clip-text text-transparent">
            🦆 Duck Client - 正在初始化服务
          </h1>
          
          {error ? (
            <div className="text-center mb-8">
              <h2 className="text-2xl font-semibold text-red-600 mb-2">❌ 初始化失败</h2>
              <div className="bg-red-50 border border-red-200 rounded-lg p-4 mt-4">
                <p className="text-red-700 font-medium">{error}</p>
              </div>
            </div>
          ) : isCompleted ? (
            <div className="text-center mb-8">
              <h2 className="text-2xl font-semibold text-green-600 mb-2">🎉 初始化完成</h2>
            </div>
          ) : (
            <h2 className="text-xl font-medium text-gray-700">{stageInfo.title}</h2>
          )}
        </div>

        {/* 阶段指示器 */}
        {!error && !isCompleted && (
          <div className="flex justify-between mb-8 bg-white/10 p-4 rounded-2xl backdrop-blur-sm">
            <div className={`flex flex-col items-center p-4 rounded-xl transition-all duration-300 ${
              currentStage === 'init' || currentStage === 'initializing'
                ? 'bg-blue-100/80 border-2 border-blue-400 scale-105 shadow-lg' 
                : currentStep > 1 
                  ? 'bg-green-100/80 border-2 border-green-400' 
                  : 'bg-white/60 border-2 border-gray-300'
            }`}>
              <div className="text-2xl mb-2">⚙️</div>
              <div className="text-xs font-semibold uppercase tracking-wide text-gray-600">初始化</div>
            </div>
            
            <div className={`flex flex-col items-center p-4 rounded-xl transition-all duration-300 ${
              currentStage === 'download' || currentStage === 'downloading'
                ? 'bg-blue-100/80 border-2 border-blue-400 scale-105 shadow-lg' 
                : currentStep > 2 
                  ? 'bg-green-100/80 border-2 border-green-400' 
                  : 'bg-white/60 border-2 border-gray-300'
            }`}>
              <div className="text-2xl mb-2">📥</div>
              <div className="text-xs font-semibold uppercase tracking-wide text-gray-600">下载</div>
            </div>
            
            <div className={`flex flex-col items-center p-4 rounded-xl transition-all duration-300 ${
              currentStage === 'extracting'
                ? 'bg-blue-100/80 border-2 border-blue-400 scale-105 shadow-lg' 
                : currentStep > 3 
                  ? 'bg-green-100/80 border-2 border-green-400' 
                  : 'bg-white/60 border-2 border-gray-300'
            }`}>
              <div className="text-2xl mb-2">📦</div>
              <div className="text-xs font-semibold uppercase tracking-wide text-gray-600">解压</div>
            </div>
            
            <div className={`flex flex-col items-center p-4 rounded-xl transition-all duration-300 ${
              currentStage === 'deploy' || currentStage === 'deploying'
                ? 'bg-blue-100/80 border-2 border-blue-400 scale-105 shadow-lg' 
                : currentStep > 4 
                  ? 'bg-green-100/80 border-2 border-green-400' 
                  : 'bg-white/60 border-2 border-gray-300'
            }`}>
              <div className="text-2xl mb-2">🚀</div>
              <div className="text-xs font-semibold uppercase tracking-wide text-gray-600">部署</div>
            </div>
          </div>
        )}

        {/* 当前阶段详情 */}
        {!error && !isCompleted && (
          <div className="bg-white/80 rounded-2xl p-6 mb-8 backdrop-blur-sm border border-black/10">
            <h3 className="text-xl font-semibold mb-2 text-gray-800">{stageInfo.icon} {stageInfo.title}</h3>
            <p className="text-base mb-4 text-gray-600 font-medium">{message}</p>
            
            {/* 进度条 */}
            <div className="mb-6">
              <div className="w-full h-3 bg-white/20 rounded-full overflow-hidden mb-2">
                <div 
                  className="h-full bg-gradient-to-r from-green-400 to-blue-500 rounded-full transition-all duration-300 shadow-sm"
                  style={{ width: `${stageProgress}%` }}
                ></div>
              </div>
              <p className="text-center text-sm text-gray-600 font-medium">
                阶段进度: {stageProgress.toFixed(1)}% | 总进度: {overallProgress.toFixed(1)}%
              </p>
            </div>

            {/* 各阶段特殊信息 */}
            {(currentStage === 'init' || currentStage === 'initializing') && (
              <div className="stage-details">
                <p>💡 正在检查服务版本和本地配置，这个过程很快</p>
              </div>
            )}

            {(currentStage === 'download' || currentStage === 'downloading') && (
              <div className="stage-details">
                <p>💡 正在下载 Docker 服务包，首次下载可能需要较长时间</p>
                <p>📱 您可以选择后台运行，完成后会自动通知</p>
              </div>
            )}
            
            {currentStage === 'extracting' && (
              <div className="stage-details">
                <p>💡 正在解压 Docker 服务包，请耐心等待</p>
                <p>📦 解压过程可能需要1-3分钟</p>
              </div>
            )}

            {(currentStage === 'deploy' || currentStage === 'deploying') && (
              <div className="stage-details">
                <p>💡 正在部署和启动服务容器</p>
                <p>🚀 服务部署可能需要5-10分钟，请耐心等待</p>
              </div>
            )}
          </div>
        )}

        {/* 完成状态的操作按钮 */}
        {isCompleted && (
          <div className="flex justify-center space-x-4 mb-8">
            <button 
              onClick={onComplete} 
              className="px-8 py-3 bg-gradient-to-r from-green-500 to-emerald-600 text-white font-semibold rounded-xl shadow-lg hover:from-green-600 hover:to-emerald-700 transform hover:scale-105 transition-all duration-200 flex items-center space-x-2"
            >
              <span>🎉</span>
              <span>进入管理界面</span>
            </button>
          </div>
        )}

        {/* 错误状态的操作按钮 */}
        {error && (
          <div className="flex justify-center space-x-4">
            <button 
              onClick={onBack} 
              className="px-6 py-3 bg-gray-500 text-white font-medium rounded-lg hover:bg-gray-600 transform hover:scale-105 transition-all duration-200 flex items-center space-x-2"
            >
              <span>←</span>
              <span>返回上一步</span>
            </button>
            <button 
              onClick={retryInitialization} 
              className="px-6 py-3 bg-gradient-to-r from-blue-500 to-blue-600 text-white font-medium rounded-lg hover:from-blue-600 hover:to-blue-700 transform hover:scale-105 transition-all duration-200 flex items-center space-x-2"
            >
              <span>🔄</span>
              <span>重试初始化</span>
            </button>
          </div>
        )}

        {/* 进行中的操作按钮 */}
        {!error && !isCompleted && (
          <div className="flex justify-center space-x-4">
            <button 
              onClick={toggleBackgroundMode}
              className="px-4 py-2 bg-gray-200/80 text-gray-700 font-medium rounded-lg hover:bg-gray-300/80 transition-all duration-200 flex items-center space-x-2"
            >
              <span>📱</span>
              <span>后台运行</span>
            </button>
            <button 
              onClick={cancelInitialization}
              className="px-4 py-2 bg-red-200/80 text-red-700 font-medium rounded-lg hover:bg-red-300/80 transition-all duration-200 flex items-center space-x-2"
            >
              <span>❌</span>
              <span>取消</span>
            </button>
          </div>
        )}

        {/* 日志显示区域 */}
        {logMessages.length > 0 && (
          <div className="bg-black/5 rounded-2xl p-4 mt-8">
            <div 
              className="flex justify-between items-center py-3 cursor-pointer border-b border-black/10 hover:text-gray-800 transition-colors"
              onClick={() => setShowLogs(!showLogs)}
            >
              <span className="font-semibold text-gray-700">📋 详细日志</span>
              <div className="flex items-center space-x-2">
                <span className="text-sm text-gray-500 bg-gray-100 px-2 py-1 rounded-full">
                  {logMessages.length} 条记录
                </span>
                <span className="text-gray-400">
                  {showLogs ? '▼' : '▶'}
                </span>
              </div>
            </div>
            
            {showLogs && (
              <div className="max-h-48 overflow-y-auto pt-4 space-y-1">
                {logMessages.map((log, index) => (
                  <div key={index} className="font-mono text-sm text-gray-600 py-1 hover:bg-gray-50 px-2 rounded">
                    {log}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
} 