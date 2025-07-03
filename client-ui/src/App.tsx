
import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { WelcomeSetup } from './pages/WelcomeSetup.tsx';
import { InitializationProgress } from './pages/InitializationProgress.tsx';
import type { AppStateInfo } from './types/index.ts';
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
        const state = await invoke<AppStateInfo>('get_app_state');
        setAppState(state);
        
        // 根据状态决定显示哪个页面
        if (state.initialized) {
          setCurrentPage('dashboard');
        } else if (state.working_directory) {
          setCurrentPage('initialization');
        } else {
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
        <InitializationProgress onComplete={handleInitializationComplete} />
      )}
      
      {currentPage === 'dashboard' && (
        <div className="dashboard">
          <h1>🦆 Duck Client Dashboard</h1>
          <p>欢迎使用 Duck Client！</p>
          <p>这里将是主控制面板...</p>
        </div>
      )}
    </div>
  );
}

export default App;
