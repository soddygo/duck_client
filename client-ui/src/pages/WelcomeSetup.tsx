import React, { useState, useEffect } from 'react';
import { Platform, SystemRequirements, StorageInfo } from '../types/index.ts';
import { getCurrentPlatform, getStoragePathSuggestion } from '../utils/tauri.ts';

import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

interface WelcomeSetupProps {
  onComplete: (workingDir: string) => void;
}

export function WelcomeSetup({ onComplete }: WelcomeSetupProps) {
  const [platform, setPlatform] = useState<Platform>('linux');
  const [workingDir, setWorkingDir] = useState<string>('');
  const [suggestedPath, setSuggestedPath] = useState<string>('');
  const [systemChecks, setSystemChecks] = useState<SystemRequirements | null>(null);
  const [storageInfo, setStorageInfo] = useState<StorageInfo | null>(null);
  const [isChecking, setIsChecking] = useState(false);
  const [canProceed, setCanProceed] = useState(false);
  const [initError, setInitError] = useState<string | null>(null);
  const [isInitializing, setIsInitializing] = useState(false);

  // 初始化平台检测和路径建议
  useEffect(() => {
    async function initPlatform() {
      try {
        const currentPlatform = await getCurrentPlatform();
        setPlatform(currentPlatform);
        
        const suggested = getStoragePathSuggestion(currentPlatform);
        setSuggestedPath(suggested);
        setWorkingDir(suggested);
        
        // 自动执行系统检查
        await performSystemChecks();
      } catch (error) {
        console.error('平台初始化失败:', error);
      }
    }
    initPlatform();
  }, []);

  // 执行系统要求检查
  const performSystemChecks = async () => {
    setIsChecking(true);
    try {
      console.log('开始系统检查, workingDir:', workingDir);
      const requirements: SystemRequirements = await invoke('check_system_requirements');
      setSystemChecks(requirements);
      console.log('系统要求检查完成:', requirements);
      
      // 设置存储空间推荐信息（不实际检测）
      console.log('设置存储空间推荐信息');
      setStorageInfo({
        path: '系统磁盘',
        total_bytes: 0,
        available_bytes: 0,
        used_bytes: 0,
        available_space_gb: 0,
        required_space_gb: 60,
        sufficient: true, // 设为true避免警告
      });
      
      // 只要有工作目录就可以继续，所有检查都是警告性质
      setCanProceed(!!workingDir);
    } catch (error) {
      console.error('系统检查失败:', error);
      // 即使检查失败，也允许用户继续
      setCanProceed(!!workingDir);
    } finally {
      setIsChecking(false);
    }
  };

  // 选择工作目录
  const selectWorkingDirectory = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: workingDir,
      });
      
      if (selected && typeof selected === 'string') {
        setWorkingDir(selected);
        await performSystemChecks();
      }
    } catch (error) {
      console.error('选择目录失败:', error);
    }
  };

  // 开始初始化
  const startInitialization = async () => {
    setInitError(null);
    setIsInitializing(true);
    
    try {
      await invoke('set_working_directory', { directory: workingDir });
      onComplete(workingDir);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      setInitError(errorMessage);
      console.error('设置工作目录失败:', error);
    } finally {
      setIsInitializing(false);
    }
  };

  // 格式化文件大小
  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  // 获取检查项状态
  const getCheckStatus = (condition: boolean, isWarning: boolean = false): 'success' | 'warning' | 'error' => {
    if (condition) return 'success';
    return isWarning ? 'warning' : 'error';
  };

  // 平台特定提示
  const getPlatformTips = (): string[] => {
    const tips: Record<Platform, string[]> = {
      windows: [
        '• 建议选择非系统盘(如D盘)以获得更好性能',
        '• 确保 Windows Defender 已将工作目录添加到排除列表',
        '• 如使用 WSL，请确保 WSL2 已启用'
      ],
      macos: [
        '• 避免选择 iCloud Drive 同步的目录',
        '• 建议使用 Documents 或专门的开发目录',
        '• 确保 Docker Desktop 已安装并运行'
      ],
      linux: [
        '• 确保有足够的磁盘空间和 inodes',
        '• 检查目录权限，避免需要 sudo 的路径',
        '• 确保当前用户在 docker 组中'
      ]
    };
    return tips[platform as Platform] || [];
  };

  // 获取警告信息
  const getWarnings = (): string[] => {
    const warnings: string[] = [];
    
    if (systemChecks && !systemChecks.os_supported) {
      warnings.push('操作系统可能不完全支持，建议升级系统');
    }
    
    if (systemChecks && !systemChecks.docker_available) {
      warnings.push('Docker 不可用，需要先安装并启动 Docker');
    }
    
    // 存储空间检查已移除，只显示推荐信息
    
    return warnings;
  };

  return (
    <div className="h-screen w-screen bg-gradient-to-br from-blue-400 via-purple-500 to-purple-600 flex flex-col">
      {/* 固定标题栏 */}
      <div className="flex-shrink-0 pt-8 pb-6 px-4">
        <div className="text-center text-white space-y-3">
          <h1 className="text-4xl md:text-5xl font-bold">🦆 Duck Client</h1>
          <h2 className="text-xl md:text-2xl font-semibold opacity-90">Docker 服务管理平台</h2>
          <p className="text-base md:text-lg opacity-80 max-w-2xl mx-auto">
            欢迎使用 Duck Client！让我们开始配置您的第一个服务吧
          </p>
        </div>
      </div>

      {/* 可滚动内容区域 */}
      <div className="flex-1 overflow-y-auto px-4 pb-8">
        <div className="max-w-4xl mx-auto space-y-6">
          {/* 工作目录选择 */}
          <div className="bg-white/95 backdrop-blur-md border border-white/20 shadow-xl rounded-2xl p-6">
            <h3 className="text-xl font-semibold text-gray-800 mb-4 flex items-center gap-2">
              📁 选择工作目录
            </h3>
            <div className="flex gap-3">
              <input
                type="text"
                value={workingDir}
                onChange={(e) => setWorkingDir(e.target.value)}
                placeholder={`推荐路径: ${suggestedPath}`}
                className="flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all duration-200"
              />
              <button 
                onClick={selectWorkingDirectory}
                className="px-6 py-3 bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors duration-200 whitespace-nowrap"
              >
                浏览...
              </button>
            </div>
          </div>

          {/* 存储空间信息 */}
          <div className="bg-white/95 backdrop-blur-md border border-white/20 shadow-xl rounded-2xl p-6">
            <h3 className="text-xl font-semibold text-gray-800 mb-4 flex items-center gap-2">
              💾 存储空间要求
            </h3>
            <div className="bg-blue-50 rounded-lg p-4 space-y-3">
              <div className="flex justify-between items-center">
                <span className="text-gray-600">推荐可用空间:</span>
                <span className="font-semibold text-blue-600">
                  至少 60 GB
                </span>
              </div>
              <div className="pt-2 border-t border-blue-200 text-sm text-blue-800 space-y-1">
                <div>• Docker 服务包: ~14 GB</div>
                <div>• 解压后文件: ~25 GB</div>
                <div>• 数据和日志: ~10 GB</div>
                <div>• 备份预留: ~15 GB</div>
              </div>
              <div className="pt-2 border-t border-blue-200 text-sm text-blue-800">
                ✅ 请确保您的磁盘有足够的可用空间
              </div>
            </div>
          </div>

          {/* 时间预估 */}
          <div className="bg-white/95 backdrop-blur-md border border-white/20 shadow-xl rounded-2xl p-6">
            <h3 className="text-xl font-semibold text-gray-800 mb-4 flex items-center gap-2">
              ⏰ 时间预估
            </h3>
            <div className="bg-blue-50 rounded-lg p-4 space-y-2 text-blue-800">
              <div>• 首次部署需要 30-60 分钟</div>
              <div>• 包含下载、解压、镜像加载等步骤</div>
              <div>• 支持断点续传，网络中断不会丢失进度</div>
              <div>• 可随时暂停和恢复下载</div>
            </div>
          </div>

          {/* 网络要求 */}
          <div className="bg-white/95 backdrop-blur-md border border-white/20 shadow-xl rounded-2xl p-6">
            <h3 className="text-xl font-semibold text-gray-800 mb-4 flex items-center gap-2">
              📶 网络要求
            </h3>
            <div className="bg-blue-50 rounded-lg p-4 space-y-2 text-blue-800">
              <div>• 建议稳定的网络连接（10 Mbps 以上）</div>
              <div>• 支持断点续传，网络不稳定时会自动重试</div>
              <div>• 可在网络条件好的时候分批下载</div>
            </div>
          </div>

          {/* 平台特定提示 */}
          <div className="bg-white/95 backdrop-blur-md border border-white/20 shadow-xl rounded-2xl p-6">
            <h3 className="text-xl font-semibold text-gray-800 mb-4">
              {platform.charAt(0).toUpperCase() + platform.slice(1)} 平台提示
            </h3>
            <div className="bg-purple-50 rounded-lg p-4 space-y-2 text-purple-800">
              {getPlatformTips().map((tip, index) => (
                <div key={index}>{tip}</div>
              ))}
            </div>
          </div>

          {/* 系统检查结果 */}
          {systemChecks && (
            <div className="bg-white/95 backdrop-blur-md border border-white/20 shadow-xl rounded-2xl p-6">
              <h3 className="text-xl font-semibold text-gray-800 mb-4 flex items-center gap-2">
                🔍 系统检查
              </h3>
              <div className="space-y-3">
                <div className="flex justify-between items-center p-3 rounded-lg bg-gray-50">
                  <span className="font-medium text-gray-700">操作系统支持</span>
                  <span className={`px-3 py-1 rounded-full text-sm font-medium ${
                    systemChecks.os_supported 
                      ? 'bg-green-100 text-green-800' 
                      : 'bg-amber-100 text-amber-800'
                  }`}>
                    {systemChecks.os_supported ? '✅ 支持' : '⚠️ 不支持'}
                  </span>
                </div>
                <div className="flex justify-between items-center p-3 rounded-lg bg-gray-50">
                  <span className="font-medium text-gray-700">Docker 可用</span>
                  <span className={`px-3 py-1 rounded-full text-sm font-medium ${
                    systemChecks.docker_available 
                      ? 'bg-green-100 text-green-800' 
                      : 'bg-amber-100 text-amber-800'
                  }`}>
                    {systemChecks.docker_available ? '✅ 可用' : '⚠️ 不可用'}
                  </span>
                </div>
                <div className="flex justify-between items-center p-3 rounded-lg bg-gray-50">
                  <span className="font-medium text-gray-700">存储空间要求</span>
                  <span className="px-3 py-1 rounded-full text-sm font-medium bg-blue-100 text-blue-800">
                    💡 至少 60 GB
                  </span>
                </div>
              </div>
            </div>
          )}

          {/* 警告信息 */}
          {getWarnings().length > 0 && (
            <div className="bg-white/95 backdrop-blur-md border border-white/20 shadow-xl rounded-2xl p-6">
              <h3 className="text-xl font-semibold text-gray-800 mb-4 flex items-center gap-2">
                ⚠️ 注意事项
              </h3>
              <div className="bg-amber-50 border border-amber-200 rounded-lg p-4">
                <div className="space-y-2 text-amber-800">
                  {getWarnings().map((warning, index) => (
                    <div key={index}>• {warning}</div>
                  ))}
                </div>
                <div className="mt-4 p-3 bg-blue-50 rounded-lg">
                  <div className="text-blue-800 text-sm">
                    💡 您可以继续初始化，但建议在使用前解决这些问题
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* 错误信息 */}
          {initError && (
            <div className="bg-white/95 backdrop-blur-md border border-red-200 shadow-xl rounded-2xl p-6">
              <h3 className="text-xl font-semibold text-red-800 mb-4 flex items-center gap-2">
                ❌ 初始化失败
              </h3>
              <div className="bg-red-50 border border-red-200 rounded-lg p-4">
                <p className="text-red-700">{initError}</p>
              </div>
            </div>
          )}

          {/* 操作按钮 */}
          <div className="flex gap-4 justify-center pt-6 pb-8">
            {isChecking ? (
              <button disabled className="bg-gradient-to-r from-blue-500 to-purple-600 text-white font-semibold px-6 py-3 rounded-lg shadow-lg opacity-50 cursor-not-allowed">
                🔍 检查系统中...
              </button>
            ) : isInitializing ? (
              <button disabled className="bg-gradient-to-r from-blue-500 to-purple-600 text-white font-semibold px-6 py-3 rounded-lg shadow-lg opacity-50 cursor-not-allowed">
                🚀 初始化中...
              </button>
            ) : (
              <>
                <button 
                  onClick={startInitialization} 
                  className="bg-gradient-to-r from-blue-500 to-purple-600 text-white font-semibold px-6 py-3 rounded-lg shadow-lg hover:shadow-xl transform hover:-translate-y-0.5 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:transform-none"
                  disabled={!workingDir}
                >
                  🚀 开始初始化
                </button>
                <button 
                  onClick={performSystemChecks} 
                  className="bg-transparent border-2 border-blue-500 text-blue-500 font-semibold px-6 py-3 rounded-lg hover:bg-blue-500 hover:text-white transition-all duration-200"
                >
                  🔄 重新检查
                </button>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
} 