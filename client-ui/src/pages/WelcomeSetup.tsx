import React, { useState, useEffect } from 'react';
import { Platform, SystemRequirements, StorageInfo } from '../types/index.ts';
import { getCurrentPlatform, getStoragePathSuggestion, openFileManager } from '../utils/tauri.ts';

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
      const requirements: SystemRequirements = await invoke('check_system_requirements');
      setSystemChecks(requirements);
      
      // 检查存储空间
      if (workingDir) {
        const storage: StorageInfo = await invoke('check_storage_space', { path: workingDir });
        setStorageInfo(storage);
        
        // 判断是否可以继续
        const canContinue = requirements.os_supported && 
                           requirements.docker_available && 
                           storage.available_bytes >= 60 * 1024 * 1024 * 1024; // 60GB
        setCanProceed(canContinue);
      }
    } catch (error) {
      console.error('系统检查失败:', error);
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
    try {
      await invoke('set_working_directory', { directory: workingDir });
      onComplete(workingDir);
    } catch (error) {
      console.error('设置工作目录失败:', error);
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
    return tips[platform] || [];
  };

  return (
    <div className="welcome-setup">
      <div className="container">
        {/* 标题部分 */}
        <div className="header">
          <h1>🦆 Duck Client</h1>
          <h2>Docker 服务管理平台</h2>
          <p>欢迎使用 Duck Client！让我们开始配置您的第一个服务吧</p>
        </div>

        {/* 工作目录选择 */}
        <div className="section">
          <h3>📁 选择工作目录</h3>
          <div className="directory-selector">
            <input
              type="text"
              value={workingDir}
              onChange={(e) => setWorkingDir(e.target.value)}
              placeholder={`推荐路径: ${suggestedPath}`}
              className="directory-input"
            />
            <button onClick={selectWorkingDirectory} className="browse-button">
              浏览...
            </button>
          </div>
        </div>

        {/* 存储空间要求 */}
        {storageInfo && (
          <div className="section">
            <h3>💾 存储空间要求</h3>
            <div className="storage-info">
              <div className="storage-item">
                <span>可用空间:</span>
                <span className={(storageInfo?.available_bytes ?? 0) >= 60 * 1024 * 1024 * 1024 ? 'sufficient' : 'insufficient'}>
                  {formatBytes(storageInfo?.available_bytes ?? 0)} {(storageInfo?.available_bytes ?? 0) >= 60 * 1024 * 1024 * 1024 ? '✅' : '❌'}
                </span>
              </div>
              <div className="storage-item">
                <span>所需空间:</span>
                <span>至少 60 GB</span>
              </div>
              <div className="requirements">
                <div>• Docker 服务包: ~14 GB</div>
                <div>• 解压后文件: ~25 GB</div>
                <div>• 数据和日志: ~10 GB</div>
                <div>• 备份预留: ~15 GB</div>
              </div>
            </div>
          </div>
        )}

        {/* 时间预估 */}
        <div className="section">
          <h3>⏰ 时间预估</h3>
          <div className="time-estimates">
            <div>• 首次部署需要 30-60 分钟</div>
            <div>• 包含下载、解压、镜像加载等步骤</div>
            <div>• 支持断点续传，网络中断不会丢失进度</div>
            <div>• 可随时暂停和恢复下载</div>
          </div>
        </div>

        {/* 网络要求 */}
        <div className="section">
          <h3>📶 网络要求</h3>
          <div className="network-requirements">
            <div>• 建议稳定的网络连接（10 Mbps 以上）</div>
            <div>• 支持断点续传，网络不稳定时会自动重试</div>
            <div>• 可在网络条件好的时候分批下载</div>
          </div>
        </div>

        {/* 平台特定提示 */}
        <div className="section">
          <h3>{platform.charAt(0).toUpperCase() + platform.slice(1)} 平台提示</h3>
          <div className="platform-tips">
            {getPlatformTips().map((tip, index) => (
              <div key={index}>{tip}</div>
            ))}
          </div>
        </div>

        {/* 系统检查结果 */}
        {systemChecks && (
          <div className="section">
            <h3>🔍 系统检查</h3>
            <div className="system-checks">
              <div className={`check-item ${systemChecks.os_supported ? 'pass' : 'fail'}`}>
                操作系统支持: {systemChecks.os_supported ? '✅' : '❌'}
              </div>
              <div className={`check-item ${systemChecks.docker_available ? 'pass' : 'fail'}`}>
                Docker 可用: {systemChecks.docker_available ? '✅' : '❌'}
              </div>
              <div className={`check-item ${(storageInfo?.available_bytes ?? 0) >= 60 * 1024 * 1024 * 1024 ? 'pass' : 'fail'}`}>
                存储空间充足: {(storageInfo?.available_bytes ?? 0) >= 60 * 1024 * 1024 * 1024 ? '✅' : '❌'}
              </div>
            </div>
          </div>
        )}

        {/* 操作按钮 */}
        <div className="actions">
          {isChecking ? (
            <button disabled className="button-primary">
              🔍 检查系统中...
            </button>
          ) : canProceed ? (
            <button onClick={startInitialization} className="button-primary">
              🚀 开始初始化
            </button>
          ) : (
            <div>
              <button onClick={performSystemChecks} className="button-secondary">
                🔄 重新检查
              </button>
              <p className="warning">
                ⚠️ 请解决上述问题后再继续
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
} 