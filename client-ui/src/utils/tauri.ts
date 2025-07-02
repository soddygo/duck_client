import { invoke } from '@tauri-apps/api/core';
import type { ServiceStatus, VersionInfo, BackupInfo, DataDirectoryInfo } from '../types/index';

// 服务状态相关API
export const serviceAPI = {
  // 获取服务状态
  async getStatus(): Promise<ServiceStatus> {
    return await invoke('get_service_status');
  },

  // 启动服务
  async start(): Promise<void> {
    return await invoke('start_service');
  },

  // 停止服务
  async stop(): Promise<void> {
    return await invoke('stop_service');
  },

  // 重启服务
  async restart(): Promise<void> {
    return await invoke('restart_service');
  },
};

// 更新相关API
export const updateAPI = {
  // 检查更新
  async checkUpdates(): Promise<VersionInfo> {
    return await invoke('check_updates');
  },

  // 执行升级
  async performUpgrade(full = false, force = false): Promise<void> {
    return await invoke('perform_upgrade', { full, force });
  },
};

// 备份相关API
export const backupAPI = {
  // 创建备份
  async create(): Promise<BackupInfo> {
    return await invoke('create_backup');
  },

  // 列出备份
  async list(): Promise<BackupInfo[]> {
    return await invoke('list_backups');
  },

  // 从备份恢复
  async restore(backupId: number, force = false): Promise<void> {
    return await invoke('restore_backup', { backupId, force });
  },
};

// 数据目录相关API
export const dataDirectoryAPI = {
  // 获取数据目录信息
  async getInfo(): Promise<DataDirectoryInfo> {
    return await invoke('get_data_directory');
  },

  // 打开数据目录（工作目录）
  async openData(): Promise<void> {
    return await invoke('open_data_directory');
  },

  // 打开备份目录
  async openBackup(): Promise<void> {
    return await invoke('open_backup_directory');
  },

  // 打开缓存目录
  async openCache(): Promise<void> {
    return await invoke('open_cache_directory');
  },
};

// 数据目录管理
export async function getDataDirectory() {
  return await invoke('get_data_directory');
}

export async function setWorkDirectory(path: string) {
  return await invoke('set_work_directory', { path });
}

export async function selectWorkDirectory() {
  return await invoke('select_work_directory');
}

export async function openDataDirectory() {
  return await invoke('open_data_directory');
}

export async function openBackupDirectory() {
  return await invoke('open_backup_directory');
}

export async function openCacheDirectory() {
  return await invoke('open_cache_directory');
}

export async function clearCache() {
  return await invoke('clear_cache');
}

export async function initClient() {
  return await invoke('init_client');
}

export async function autoDeployService(port?: number) {
  return await invoke('auto_deploy_service', { port });
}

export async function initAndDeploy(port?: number) {
  return await invoke('init_and_deploy', { port });
}

// 统一的API对象
export const tauriAPI = {
  service: serviceAPI,
  update: updateAPI,
  backup: backupAPI,
  dataDirectory: dataDirectoryAPI,
}; 