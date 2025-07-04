import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  AppStateInfo,
  SystemRequirements,
  ServiceStatus,
  TaskHandle,
  UIConfiguration,
  InitProgressEvent,
  InitCompletedEvent,
  DownloadProgressEvent,
  DownloadCompletedEvent,
  ServiceStatusUpdateEvent,
  Platform,
} from '../types/index.ts';

// ==================== 应用状态管理 API ====================

export async function getAppState(): Promise<AppStateInfo> {
  return await invoke('get_app_state');
}

export async function setWorkingDirectory(directory: string): Promise<void> {
  return await invoke('set_working_directory', { directory });
}

export async function getWorkingDirectory(): Promise<string> {
  return await invoke('get_working_directory');
}

export async function resetWorkingDirectory(): Promise<void> {
  return await invoke('reset_working_directory');
}

// ==================== 系统检查 API ====================

export async function checkSystemRequirements(
  directory?: string
): Promise<SystemRequirements> {
  return await invoke('check_system_requirements', { directory });
}

// ==================== 初始化和下载 API ====================

export async function initClientWithProgress(): Promise<string> {
  return await invoke('init_client_with_progress');
}

export async function downloadAndDeployServices(): Promise<string> {
  return await invoke('download_and_deploy_services');
}

export async function downloadPackageWithProgress(
  url: string,
  targetDir: string
): Promise<string> {
  return await invoke('download_package_with_progress', { url, targetDir });
}

// ==================== 服务管理 API ====================

export async function getServicesStatus(): Promise<ServiceStatus[]> {
  return await invoke('get_services_status');
}

export async function startServicesMonitoring(): Promise<void> {
  return await invoke('start_services_monitoring');
}

export async function startServices(): Promise<string> {
  return await invoke('start_services');
}

export async function stopServices(): Promise<string> {
  return await invoke('stop_services');
}

export async function restartServices(): Promise<string> {
  return await invoke('restart_services');
}

// ==================== 配置管理 API ====================

export async function getUIConfiguration(): Promise<UIConfiguration> {
  return await invoke('get_ui_configuration');
}

export async function updateUIConfiguration(config: UIConfiguration): Promise<void> {
  return await invoke('update_ui_configuration', { config });
}

// ==================== 任务管理 API ====================

export async function getCurrentTasks(): Promise<TaskHandle[]> {
  return await invoke('get_current_tasks');
}

export async function cancelTask(taskId: string): Promise<void> {
  return await invoke('cancel_task', { taskId });
}

// ==================== 事件监听器管理 ====================

export class EventManager {
  private listeners: Map<string, () => void> = new Map();

  // 监听初始化进度
  async onInitProgress(callback: (event: InitProgressEvent) => void): Promise<void> {
    const unlisten = await listen('init-progress', (event) => {
      callback(event.payload as InitProgressEvent);
    });
    this.listeners.set('init-progress', unlisten);
  }

  // 监听初始化完成
  async onInitCompleted(callback: (event: InitCompletedEvent) => void): Promise<void> {
    const unlisten = await listen('init-completed', (event) => {
      callback(event.payload as InitCompletedEvent);
    });
    this.listeners.set('init-completed', unlisten);
  }

  // 监听下载进度
  async onDownloadProgress(callback: (event: DownloadProgressEvent) => void): Promise<void> {
    const unlisten = await listen('download-progress', (event) => {
      callback(event.payload as DownloadProgressEvent);
    });
    this.listeners.set('download-progress', unlisten);
  }

  // 监听下载完成
  async onDownloadCompleted(callback: (event: DownloadCompletedEvent) => void): Promise<void> {
    const unlisten = await listen('download-completed', (event) => {
      callback(event.payload as DownloadCompletedEvent);
    });
    this.listeners.set('download-completed', unlisten);
  }

  // 监听服务状态更新
  async onServiceStatusUpdate(callback: (event: ServiceStatusUpdateEvent) => void): Promise<void> {
    const unlisten = await listen('service-status-update', (event) => {
      callback(event.payload as ServiceStatusUpdateEvent);
    });
    this.listeners.set('service-status-update', unlisten);
  }

  // 监听应用状态变化
  async onAppStateChanged(callback: (event: any) => void): Promise<void> {
    const unlisten = await listen('app-state-changed', (event) => {
      callback(event.payload);
    });
    this.listeners.set('app-state-changed', unlisten);
  }

  // 监听需要初始化事件
  async onRequireInitialization(callback: (event: any) => void): Promise<void> {
    const unlisten = await listen('require-initialization', (event) => {
      callback(event.payload);
    });
    this.listeners.set('require-initialization', unlisten);
  }

  // 清理所有监听器
  cleanup(): void {
    for (const [eventName, unlisten] of this.listeners) {
      unlisten();
      console.log(`已清理事件监听器: ${eventName}`);
    }
    this.listeners.clear();
  }

  // 清理特定监听器
  cleanupEvent(eventName: string): void {
    const unlisten = this.listeners.get(eventName);
    if (unlisten) {
      unlisten();
      this.listeners.delete(eventName);
      console.log(`已清理事件监听器: ${eventName}`);
    }
  }
}

// ==================== 工具函数 ====================

// 格式化文件大小
export function formatFileSize(bytes: number, precision = 2): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let size = bytes;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  return `${size.toFixed(precision)} ${units[unitIndex]}`;
}

// 格式化下载速度
export function formatDownloadSpeed(bytesPerSecond: number): string {
  return `${formatFileSize(bytesPerSecond)}/s`;
}

// 格式化剩余时间
export function formatETA(seconds: number): string {
  if (seconds < 60) {
    return `${Math.round(seconds)} 秒`;
  } else if (seconds < 3600) {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.round(seconds % 60);
    return `${minutes} 分 ${remainingSeconds} 秒`;
  } else {
    const hours = Math.floor(seconds / 3600);
    const remainingMinutes = Math.floor((seconds % 3600) / 60);
    return `${hours} 小时 ${remainingMinutes} 分钟`;
  }
}

// 获取平台特定的路径建议
export function getPlatformDefaultPath(platform: string): string {
  switch (platform.toLowerCase()) {
    case 'windows':
      return 'C:\\Users\\%USERNAME%\\Documents\\DuckClient';
    case 'macos':
      return '~/Documents/DuckClient';
    case 'linux':
      return '~/DuckClient';
    default:
      return './DuckClient';
  }
}

// 获取平台特定的Docker安装指南链接
export function getDockerInstallGuide(platform: string): string {
  switch (platform.toLowerCase()) {
    case 'windows':
      return 'https://docs.docker.com/desktop/install/windows-install/';
    case 'macos':
      return 'https://docs.docker.com/desktop/install/mac-install/';
    case 'linux':
      return 'https://docs.docker.com/engine/install/';
    default:
      return 'https://docs.docker.com/get-docker/';
  }
}

// 获取平台特定的提示信息
export function getPlatformTips(platform: string): string[] {
  switch (platform.toLowerCase()) {
    case 'windows':
      return [
        '建议选择非系统盘(如D盘)以获得更好性能',
        '确保 Windows Defender 已将工作目录添加到排除列表',
        '如使用 WSL，请确保 WSL2 已启用',
      ];
    case 'macos':
      return [
        '避免选择 iCloud Drive 同步的目录',
        '建议使用 Documents 或专门的开发目录',
        '确保 Docker Desktop 已安装并运行',
      ];
    case 'linux':
      return [
        '确保有足够的磁盘空间和 inodes',
        '检查目录权限，避免需要 sudo 的路径',
        '确保当前用户在 docker 组中',
      ];
    default:
      return ['请确保 Docker 已安装并运行'];
  }
}

// ==================== 错误处理工具 ====================

export class TauriAPIError extends Error {
  public code: string;
  public details?: string;

  constructor(message: string, code: string = 'UNKNOWN_ERROR', details?: string) {
    super(message);
    this.name = 'TauriAPIError';
    this.code = code;
    this.details = details;
  }
}

// 错误处理包装器
export async function withErrorHandling<T>(
  apiCall: () => Promise<T>,
  errorContext?: string
): Promise<T> {
  try {
    return await apiCall();
  } catch (error) {
    const message = error instanceof Error ? error.message : '未知错误';
    const context = errorContext ? `${errorContext}: ` : '';
    
    console.error(`${context}${message}`, error);
    
    throw new TauriAPIError(
      `${context}${message}`,
      'API_CALL_FAILED',
      error instanceof Error ? error.stack : undefined
    );
  }
}

// ==================== 状态验证工具 ====================

export function validateAppState(state: string): state is AppStateInfo['state'] {
  const validStates = [
    'UNINITIALIZED',
    'INITIALIZING',
    'DOWNLOADING',
    'DEPLOYING',
    'READY',
    'UPGRADING',
    'ERROR'
  ];
  return validStates.includes(state);
}

export function validateDownloadStatus(status: string): boolean {
  const validStatuses = [
    'Starting',
    'Downloading',
    'Paused',
    'Completed',
    'Failed',
    'Cancelled'
  ];
  return validStatuses.includes(status);
}

// ==================== 单例事件管理器 ====================

// 全局事件管理器实例
export const globalEventManager = new EventManager();

// 在应用卸载时清理
if (typeof window !== 'undefined') {
  window.addEventListener('beforeunload', () => {
    globalEventManager.cleanup();
  });
}

// ==================== 平台相关工具 ====================

// 获取当前平台信息
export async function getCurrentPlatform(): Promise<Platform> {
  try {
    const platformName = await invoke('get_platform');
    switch (platformName) {
      case 'windows':
        return 'windows';
      case 'macos':
        return 'macos';
      case 'linux':
        return 'linux';
      default:
        return 'linux';
    }
  } catch (error) {
    console.error('获取平台信息失败:', error);
    return 'linux';
  }
}

// 获取存储路径建议（别名函数）
export function getStoragePathSuggestion(platform: Platform): string {
  return getPlatformDefaultPath(platform);
}

// 打开文件管理器
export async function openFileManager(path: string): Promise<void> {
  try {
    await invoke('open_file_manager', { path });
  } catch (error) {
    console.error('打开文件管理器失败:', error);
  }
}

// ==================== 统一 API 对象 ====================

// 为了兼容前端页面的调用方式，提供统一的 API 对象
export const tauriAPI = {
  service: {
    start: startServices,
    stop: stopServices,
    restart: restartServices,
    getStatus: getServicesStatus,
    startMonitoring: startServicesMonitoring,
  },
  system: {
    checkRequirements: checkSystemRequirements,
    getCurrentPlatform,
    openFileManager,
  },
  directory: {
    getAppState,
    setWorkingDirectory,
    getWorkingDirectory,
    resetWorkingDirectory,
  },
  init: {
    initClientWithProgress,
    downloadAndDeployServices,
    downloadPackageWithProgress,
  },
  tasks: {
    getCurrentTasks,
    cancelTask,
  },
  ui: {
    getConfiguration: getUIConfiguration,
    updateConfiguration: updateUIConfiguration,
  },
  events: globalEventManager,
  utils: {
    formatFileSize,
    formatDownloadSpeed,
    formatETA,
    getPlatformDefaultPath,
    getDockerInstallGuide,
    getPlatformTips,
    getStoragePathSuggestion,
    withErrorHandling,
    validateAppState,
    validateDownloadStatus,
  },
}; 