// Tauri命令返回的数据类型定义

export interface ServiceStatus {
  status: string;
  containers: number;
  uptime: string;
}

export interface VersionInfo {
  client_version: string;
  service_version: string;
}

export interface BackupInfo {
  id: number;
  name: string;
  created_at: string;
  size: number;
}

export interface ActivityLog {
  id: string;
  timestamp: string;
  type: "success" | "error" | "info";
  message: string;
}

// UI状态类型
export interface ServiceStatusUI {
  isRunning: boolean;
  containers: number;
  uptime: string;
}

export interface VersionInfoUI {
  clientVersion: string;
  serviceVersion: string;
  hasUpdate: boolean;
  latestVersion?: string;
}

// 更新数据目录信息接口
export interface DataDirectoryInfo {
  path: string;
  backup_path: string;
  cache_path: string;
  docker_path: string;
  exists: boolean;
  backup_exists: boolean;
  cache_exists: boolean;
  docker_exists: boolean;
  total_size_mb: number;
}
