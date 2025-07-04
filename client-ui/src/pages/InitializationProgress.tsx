import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { 
  InitStage, 
  InitProgress,
  InitProgressEvent,
  InitCompletedEvent 
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
  const [totalSteps, setTotalSteps] = useState<number>(2);
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
          
          // 更新进度
          setStageProgress(event.percentage);
          setOverallProgress(50 + (event.percentage / 2)); // 第二步占总进度的50%
          setMessage(event.message);
          
          // 添加日志信息
          addLogMessage(`[${event.stage}] ${event.message}`);
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
    addLogMessage(isBackground ? '🔄 切换到前台模式' : '📱 切换到后台运行');
  };

  // 获取阶段信息
  const getStageInfo = (stage: InitStage) => {
    const stageInfoMap: Record<InitStage, { title: string; description: string; icon: string }> = {
      init: {
        title: '第 1 步 / 共 2 步：本地初始化',
        description: '正在创建配置文件和初始化数据库',
        icon: '⚙️'
      },
      deploy: {
        title: '第 2 步 / 共 2 步：下载和部署服务',
        description: '正在下载 Docker 镜像和部署服务容器',
        icon: '🚀'
      },
      // 保留其他兼容性名称
      download: {
        title: '第 2 步 / 共 2 步：下载和部署服务',
        description: '正在下载 Docker 镜像和部署服务容器',
        icon: '📦'
      },
      downloading: {
        title: '第 1 步 / 共 2 步：本地初始化',
        description: '正在创建配置文件和初始化数据库',
        icon: '⚙️'
      },
      extracting: {
        title: '第 2 步 / 共 2 步：下载和部署服务',
        description: '正在下载 Docker 镜像和部署服务容器',
        icon: '📦'
      },
      loading: {
        title: '第 2 步 / 共 2 步：下载和部署服务',
        description: '正在部署和启动服务容器',
        icon: '🚀'
      },
      starting: {
        title: '正在完成部署...',
        description: '正在完成Docker服务的最终配置',
        icon: '🔧'
      },
      configuring: {
        title: '正在完成初始化...',
        description: '正在完成最终的系统配置和初始化',
        icon: '🔧'
      }
    };
    
    return stageInfoMap[stage] || {
      title: '正在初始化...',
      description: '正在准备初始化系统',
      icon: '⏳'
    };
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
    <div className="initialization-progress">
      <div className="container">
        {/* 标题部分 */}
        <div className="header">
          <h1>🦆 Duck Client - 正在初始化服务</h1>
          
          {error ? (
            <div className="error-state">
              <h2>❌ 初始化失败</h2>
              <p className="error-message">{error}</p>
            </div>
          ) : isCompleted ? (
            <div className="completed-state">
              <h2>🎉 恭喜！Duck Client 初始化完成</h2>
            </div>
          ) : stageInfo ? (
            <h2>{stageInfo.title}</h2>
          ) : (
            <h2>正在准备初始化...</h2>
          )}
        </div>

        {/* 阶段进度指示器 */}
        <div className="stage-indicators">
          {(['init', 'deploy'] as InitStage[]).map((stage, index) => {
            const isActive = stage === currentStage;
            const isCompleted = index < currentStep - 1 || (index === currentStep - 1 && stageProgress === 100);
            const stageInfo = getStageInfo(stage);
            
            return (
              <div 
                key={stage}
                className={`stage-indicator ${isActive ? 'active' : ''} ${isCompleted ? 'completed' : ''}`}
              >
                <div className="stage-icon">{stageInfo ? stageInfo.icon : '⏳'}</div>
                <div className="stage-label">{stage === 'init' ? 'Init' : 'Deploy'}</div>
              </div>
            );
          })}
        </div>

        {/* 当前阶段详情 */}
        {!error && !isCompleted && (
          <div className="current-stage">
            <h3>{stageInfo ? stageInfo.description : '正在准备初始化...'}</h3>
            <p className="stage-message">{message}</p>
            
            {/* 进度条 */}
            <div className="progress-section">
              <div className="progress-bar">
                <div 
                  className="progress-fill" 
                  style={{ width: `${stageProgress}%` }}
                ></div>
              </div>
              <div className="progress-text">
                {stageProgress.toFixed(1)}% | 总进度: {overallProgress.toFixed(1)}%
              </div>
            </div>

            {/* 各阶段特殊信息 */}
            {currentStage === 'init' && (
              <div className="stage-details">
                <p>💡 正在本地创建配置文件和数据库，这个过程很快</p>
              </div>
            )}

            {(currentStage === 'deploy' || currentStage === 'download') && (
              <div className="stage-details">
                <p>💡 正在下载 Docker 镜像和部署服务，首次下载可能需要较长时间</p>
                <p>📱 您可以选择后台运行，完成后会自动通知</p>
              </div>
            )}
          </div>
        )}

        {/* 完成状态显示 */}
        {isCompleted && (
          <div className="completion-details">
            <div className="completion-stats">
              <div className="stat-item">
                <span className="label">📊 服务统计:</span>
                <span className="value">Docker 服务已部署</span>
              </div>
              <div className="stat-item">
                <span className="label">📋 完成步骤:</span>
                <span className="value">{totalSteps} 个步骤</span>
              </div>
              <div className="stat-item">
                <span className="label">🌐 服务状态:</span>
                <span className="value">已准备就绪</span>
              </div>
            </div>
            
            <div className="completion-actions">
              <button onClick={onComplete} className="btn-primary large">
                🚀 进入控制台
              </button>
            </div>
          </div>
        )}

        {/* 操作按钮 */}
        {!error && !isCompleted && (
          <div className="actions">
            <button onClick={onBack} className="btn-secondary" disabled={isInitializing}>
              ← 返回上一步
            </button>
            
            <button onClick={toggleBackgroundMode} className="btn-secondary" disabled={currentStage === 'init'}>
              📱 后台运行
            </button>
            
            <button onClick={cancelInitialization} className="btn-danger" disabled={isInitializing}>
              ❌ 取消初始化
            </button>
          </div>
        )}

        {/* 错误状态的操作按钮 */}
        {error && (
          <div className="actions">
            <button onClick={onBack} className="btn-secondary">
              ← 返回上一步
            </button>
            <button onClick={() => window.location.reload()} className="btn-primary">
              🔄 重试
            </button>
          </div>
        )}

        {/* 详细日志 */}
        <div className="log-section">
          <div className="log-header" onClick={() => setShowDetails(!showDetails)}>
            <span>详细日志 {showDetails ? '🔼' : '🔽'}</span>
            <span className="log-count">({logMessages.length} 条)</span>
          </div>
          
          {showDetails && (
            <div className="log-content">
              {logMessages.map((log, index) => (
                <div key={index} className="log-item">{log}</div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
} 