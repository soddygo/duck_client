import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { WelcomeSetup } from './pages/WelcomeSetup.tsx';
import { InitializationProgress } from './pages/InitializationProgress.tsx';
import Dashboard from './pages/Dashboard.tsx';
import type { AppStateInfo } from './types/index.ts';
import { globalEventManager } from './utils/tauri.ts';
import { message } from 'antd';
import './App.css';

type AppPage = 'welcome' | 'initialization' | 'dashboard';

function App() {
  const [currentPage, setCurrentPage] = useState<AppPage>('welcome');
  const [appState, setAppState] = useState<AppStateInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // 检查应用状态
  useEffect(() => {
    const checkAppState = async () => {
      try {
        // 首先初始化应用状态（加载保存的工作目录等）
        await invoke('initialize_app_state');
        console.log('应用状态初始化完成');
        
        // 然后获取当前应用状态
        const state = await invoke<AppStateInfo>('get_app_state');
        console.log('应用状态:', state);
        setAppState(state);
        
        // 根据状态决定显示哪个页面
        if (state.initialized) {
          console.log('已初始化，进入dashboard');
          setCurrentPage('dashboard');
        } else if (state.working_directory) {
          console.log('有工作目录，进入initialization:', state.working_directory);
          setCurrentPage('initialization');
        } else {
          console.log('无工作目录，进入welcome');
          setCurrentPage('welcome');
        }
      } catch (error) {
        console.error('获取应用状态失败:', error);
        setCurrentPage('welcome');
      } finally {
        setIsLoading(false);
      }
    };

    checkAppState();
  }, []);

  // 设置事件监听器
  useEffect(() => {
    const setupEventListeners = async () => {
      // 监听应用状态变化
      await globalEventManager.onAppStateChanged((newState: AppStateInfo) => {
        console.log('应用状态已变化:', newState);
        setAppState(newState);
        
        // 根据新状态自动导航
        if (newState.initialized) {
          console.log('检测到已初始化状态，跳转到dashboard');
          setCurrentPage('dashboard');
          message.success('应用状态已更新，进入主界面');
        } else if (newState.working_directory) {
          console.log('检测到有工作目录但未初始化，跳转到initialization');
          setCurrentPage('initialization');
          message.info('检测到工作目录变更，需要重新初始化');
        } else {
          console.log('检测到无工作目录，跳转到welcome');
          setCurrentPage('welcome');
          message.info('请选择工作目录');
        }
      });

      // 监听需要初始化事件
      await globalEventManager.onRequireInitialization((event: any) => {
        console.log('收到需要初始化事件:', event);
        
        const { working_directory, reason } = event;
        
        // 显示提示信息
        if (reason) {
          message.warning(reason);
        }
        
        // 更新应用状态
        setAppState(prev => ({
          ...prev,
          state: 'UNINITIALIZED',
          initialized: false,
          working_directory,
        }));
        
        // 导航到初始化页面
        if (working_directory) {
          console.log('需要重新初始化，跳转到initialization页面');
          setCurrentPage('initialization');
        } else {
          console.log('需要选择工作目录，跳转到welcome页面');
          setCurrentPage('welcome');
        }
      });
    };

    setupEventListeners().catch(console.error);

    // 清理函数
    return () => {
      globalEventManager.cleanupEvent('app-state-changed');
      globalEventManager.cleanupEvent('require-initialization');
    };
  }, []);

  // 处理欢迎页面完成
  const handleWelcomeComplete = (workingDir: string) => {
    setCurrentPage('initialization');
    setAppState(prev => prev ? { ...prev, working_directory: workingDir } : null);
  };

  // 处理初始化完成
  const handleInitializationComplete = () => {
    setCurrentPage('dashboard');
    setAppState(prev => prev ? { ...prev, initialized: true } : null);
  };

  // 处理从初始化页面返回
  const handleInitializationBack = () => {
    setCurrentPage('welcome');
    setAppState(prev => prev ? { ...prev, working_directory: undefined } : null);
  };

  // 加载状态
  if (isLoading) {
    return (
      <div className="app-loading">
        <div className="loading-content">
          <div className="loading-spinner"></div>
          <h2>🦆 Duck Client</h2>
          <p>正在加载应用...</p>
        </div>
      </div>
    );
  }

  // 渲染对应的页面
  return (
    <div className="app">
      {currentPage === 'welcome' && (
        <WelcomeSetup onComplete={handleWelcomeComplete} />
      )}
      
      {currentPage === 'initialization' && (
        <InitializationProgress 
          onComplete={handleInitializationComplete} 
          onBack={handleInitializationBack}
        />
      )}
      
      {currentPage === 'dashboard' && (
        <Dashboard />
      )}
    </div>
  );
}

export default App;
