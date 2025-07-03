import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { 
  InitStage, 
  DownloadProgress, 
  InitProgress,
  InitProgressEvent,
  InitCompletedEvent 
} from '../types/index.ts';
import { formatFileSize, formatDownloadSpeed, formatETA, globalEventManager } from '../utils/tauri.ts';

interface InitializationProgressProps {
  onComplete: () => void;
  onBack: () => void;
}

export function InitializationProgress({ onComplete, onBack }: InitializationProgressProps) {
  const [currentStage, setCurrentStage] = useState<InitStage>('downloading');
  const [stageProgress, setStageProgress] = useState<number>(0);
  const [overallProgress, setOverallProgress] = useState<number>(0);
  const [currentStep, setCurrentStep] = useState<number>(1);
  const [totalSteps, setTotalSteps] = useState<number>(5);
  const [message, setMessage] = useState<string>('正在准备初始化...');
  const [taskId, setTaskId] = useState<string>('');
  
  // 下载相关状态
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [downloadSpeed, setDownloadSpeed] = useState<number>(0);
  const [eta, setEta] = useState<number>(0);
  const [downloadedBytes, setDownloadedBytes] = useState<number>(0);
  const [totalBytes, setTotalBytes] = useState<number>(0);
  
  // 控制状态
  const [canPause, setCanPause] = useState<boolean>(true);
  const [isPaused, setIsPaused] = useState<boolean>(false);
  const [isBackground, setIsBackground] = useState<boolean>(false);
  const [isCompleted, setIsCompleted] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  
  // 详细信息状态
  const [showDetails, setShowDetails] = useState<boolean>(false);
  const [logMessages, setLogMessages] = useState<string[]>([]);

  // 启动初始化和监听进度事件
  useEffect(() => {
    let unsubscribe: (() => void) | null = null;

    const startInitialization = async () => {
      try {
        // 获取当前工作目录
        const appState = await invoke<any>('get_app_state');
        const workingDir = appState.working_directory;
        
        if (!workingDir) {
          setError('工作目录未设置');
          return;
        }

        addLogMessage('🚀 开始初始化服务...');
        addLogMessage(`📁 工作目录: ${workingDir}`);
        
        // 启动初始化任务
        const initTaskId = await invoke<string>('init_client_with_progress', { 
          workingDir: workingDir 
        });
        setTaskId(initTaskId);
        
        addLogMessage(`✅ 初始化任务已启动 (ID: ${initTaskId})`);
        
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        setError(`启动初始化失败: ${errorMessage}`);
        addLogMessage(`❌ 启动初始化失败: ${errorMessage}`);
      }
    };

    const setupEventListeners = async () => {
      // 监听初始化进度
      await globalEventManager.onInitProgress((event: InitProgressEvent) => {
        // 安全的类型转换，确保stage是有效的InitStage
        const validStages: InitStage[] = ['downloading', 'extracting', 'loading', 'starting', 'configuring'];
        const stage = validStages.includes(event.stage as InitStage) 
          ? (event.stage as InitStage) 
          : 'downloading';
          
        setCurrentStage(stage);
        setStageProgress(event.percentage);
        setMessage(event.message);
        setCurrentStep(event.current_step);
        setTotalSteps(event.total_steps);
        
        // 计算总体进度
        const stageWeight = 100 / event.total_steps;
        const totalProgress = ((event.current_step - 1) * stageWeight) + (event.percentage * stageWeight / 100);
        setOverallProgress(Math.min(100, Math.max(0, totalProgress)));
        
        // 添加日志信息
        addLogMessage(`[${event.stage}] ${event.message}`);
      });

      // 监听初始化完成
      await globalEventManager.onInitCompleted((event: InitCompletedEvent) => {
        if (event.success) {
          setIsCompleted(true);
          setOverallProgress(100);
          setMessage('初始化完成！');
          addLogMessage('✅ 初始化完成');
          // 可以在这里调用 onComplete() 来跳转到下一页
        } else {
          setError(event.error || '初始化失败');
          addLogMessage(`❌ 初始化失败: ${event.error || '未知错误'}`);
        }
      });

      // 监听下载进度
      await globalEventManager.onDownloadProgress((event) => {
        setDownloadedBytes(event.downloaded_bytes);
        setTotalBytes(event.total_bytes);
        setDownloadSpeed(event.download_speed);
        setEta(event.eta_seconds);
        setStageProgress(event.percentage);
        
        addLogMessage(`下载进度: ${event.percentage.toFixed(1)}% (${formatFileSize(event.downloaded_bytes)}/${formatFileSize(event.total_bytes)})`);
      });
    };

    // 先设置事件监听器，然后启动初始化
    setupEventListeners().then(() => {
      startInitialization();
    });

    return () => {
      globalEventManager.cleanup();
    };
  }, []);

  // 添加日志消息
  const addLogMessage = (message: string) => {
    const timestamp = new Date().toLocaleTimeString();
    setLogMessages(prev => [...prev.slice(-50), `[${timestamp}] ${message}`]); // 保留最近50条
  };

  // 暂停下载
  const pauseDownload = async () => {
    try {
      // await invoke('pause_download');
      setIsPaused(true);
      addLogMessage('⏸️ 下载已暂停');
    } catch (error) {
      console.error('暂停下载失败:', error);
    }
  };

  // 恢复下载
  const resumeDownload = async () => {
    try {
      // await invoke('resume_download');
      setIsPaused(false);
      addLogMessage('▶️ 下载已恢复');
    } catch (error) {
      console.error('恢复下载失败:', error);
    }
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
    addLogMessage(isBackground ? '🔄 切换到前台模式' : '📱 切换到后台模式');
  };

  // 获取阶段信息
  const getStageInfo = (stage: InitStage) => {
    const stageInfoMap: Record<InitStage, { title: string; description: string; icon: string }> = {
      downloading: {
        title: '第 1 步 / 共 5 步：下载 Docker 服务包',
        description: '正在下载 Docker 服务包，包含所需的镜像和配置文件',
        icon: '📦'
      },
      extracting: {
        title: '第 2 步 / 共 5 步：解压服务文件',
        description: '正在解压下载的服务包，准备镜像文件',
        icon: '📁'
      },
      loading: {
        title: '第 3 步 / 共 5 步：加载 Docker 镜像',
        description: '正在将镜像文件加载到本地 Docker 环境',
        icon: '🐳'
      },
      starting: {
        title: '第 4 步 / 共 5 步：启动 Docker 服务',
        description: '正在启动和配置 Docker 服务容器',
        icon: '🚀'
      },
      configuring: {
        title: '第 5 步 / 共 5 步：完成系统配置',
        description: '正在进行最终的系统配置和初始化',
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
                {overallProgress.toFixed(0)}% | {formatDownloadSpeed(downloadSpeed)} | {formatETA(eta)}
              </span>
            </div>
          </div>
          <div className="mini-actions">
            <button onClick={toggleBackgroundMode} className="btn-mini">
              📋 查看详情
            </button>
            {canPause && !isPaused && (
              <button onClick={pauseDownload} className="btn-mini">
                ⏸️ 暂停
              </button>
            )}
            {isPaused && (
              <button onClick={resumeDownload} className="btn-mini">
                ▶️ 恢复
              </button>
            )}
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
          {(['downloading', 'extracting', 'loading', 'starting', 'configuring'] as InitStage[]).map((stage, index) => {
            const isActive = stage === currentStage;
            const isCompleted = index < currentStep - 1;
            const stageInfo = getStageInfo(stage);
            
            return (
              <div 
                key={stage}
                className={`stage-indicator ${isActive ? 'active' : ''} ${isCompleted ? 'completed' : ''}`}
              >
                <div className="stage-icon">{stageInfo ? stageInfo.icon : '⏳'}</div>
                <div className="stage-label">{stage}</div>
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
                {stageProgress.toFixed(1)}%
              </div>
            </div>

            {/* 下载阶段特殊信息 */}
            {currentStage === 'downloading' && (
              <div className="download-details">
                <div className="download-stats">
                  <div className="stat-item">
                    <span className="label">📊 已下载:</span>
                    <span className="value">{formatFileSize(downloadedBytes)} / {formatFileSize(totalBytes)}</span>
                  </div>
                  <div className="stat-item">
                    <span className="label">⏱️ 下载速度:</span>
                    <span className="value">{formatDownloadSpeed(downloadSpeed)}</span>
                  </div>
                  <div className="stat-item">
                    <span className="label">⏰ 预计剩余:</span>
                    <span className="value">{formatETA(eta)}</span>
                  </div>
                </div>
                
                <div className="download-info">
                  <p>ℹ️ 支持断点续传，网络中断后可自动恢复。您可以最小化窗口或暂停下载</p>
                </div>
              </div>
            )}

            {/* 其他阶段的特殊信息 */}
            {currentStage === 'extracting' && (
              <div className="extract-details">
                <p>💡 解压过程中系统可能会比较繁忙，这是正常现象</p>
              </div>
            )}

            {currentStage === 'loading' && (
              <div className="loading-details">
                <p>💡 首次加载镜像需要较长时间，后续启动会很快</p>
              </div>
            )}

            {currentStage === 'starting' && (
              <div className="starting-details">
                <p>💡 首次启动需要初始化数据库，请耐心等待</p>
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
                <span className="value">5 个容器</span>
              </div>
              <div className="stat-item">
                <span className="label">📦 下载大小:</span>
                <span className="value">{formatFileSize(totalBytes)}</span>
              </div>
              <div className="stat-item">
                <span className="label">🌐 服务地址:</span>
                <span className="value">http://localhost</span>
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
            <button onClick={onBack} className="btn-secondary">
              ← 返回上一步
            </button>
            
            <button onClick={toggleBackgroundMode} className="btn-secondary">
              💾 后台下载
            </button>
            
            {canPause && !isPaused && currentStage === 'downloading' && (
              <button onClick={pauseDownload} className="btn-secondary">
                ⏸️ 暂停下载
              </button>
            )}
            
            {isPaused && (
              <button onClick={resumeDownload} className="btn-primary">
                🔄 断点续传
              </button>
            )}
            
            <button onClick={cancelInitialization} className="btn-danger">
              ❌ 取消安装
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