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
  work_dir: string;
  backup_path: string;
  cache_path: string;
  docker_path: string;
  config_exists: boolean;
  backup_exists: boolean;
  cache_exists: boolean;
  docker_exists: boolean;
  total_size_mb: number;
  is_initialized: boolean;
}

// ==================== 应用状态相关类型 ====================

export type AppState = 
  | 'UNINITIALIZED' 
  | 'INITIALIZING' 
  | 'DOWNLOADING' 
  | 'DEPLOYING' 
  | 'READY' 
  | 'UPGRADING' 
  | 'ERROR';

export interface AppStateInfo {
  state: AppState;
  initialized: boolean;
  working_directory?: string;
  last_error?: string;
}

// ==================== 系统检查相关类型 ====================

export interface SystemRequirements {
  os_supported: boolean;
  docker_available: boolean;
  storage_sufficient: boolean;
  available_space_gb: number;
  required_space_gb: number;
  platform_specific: PlatformSpecificChecks;
}

export interface PlatformSpecificChecks {
  docker_desktop_installed: boolean;
  wsl_enabled: boolean; // Windows only
  homebrew_docker: boolean; // macOS only
  docker_group_member: boolean; // Linux only
}

export type Platform = 'windows' | 'macos' | 'linux';

// ==================== 下载和进度相关类型 ====================

export interface DownloadProgress {
  task_id: string;
  file_name: string;
  downloaded_bytes: number;
  total_bytes: number;
  download_speed: number; // bytes/sec
  eta_seconds: number;
  percentage: number;
  status: DownloadStatus;
}

export type DownloadStatus = 
  | 'Starting'
  | 'Downloading'
  | 'Paused'
  | 'Completed'
  | 'Failed'
  | 'Cancelled';

export interface InitProgress {
  task_id: string;
  stage: InitStage;
  message: string;
  percentage: number;
  current_step: number;
  total_steps: number;
}

export type InitStage = 
  | 'init'
  | 'download'
  | 'deploy'
  | 'downloading'
  | 'extracting'
  | 'loading'
  | 'starting'
  | 'configuring';

// ==================== 服务管理相关类型 ====================

export interface ServiceStatus {
  name: string;
  status: string;
  health: string;
  uptime_seconds?: number;
  cpu_usage: number;
  memory_usage_mb: number;
  ports: string[];
}

// ==================== 任务管理相关类型 ====================

export interface TaskHandle {
  task_id: string;
  task_type: string;
  status: TaskStatus;
  progress: number;
}

export type TaskStatus = 
  | 'starting'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled';

// ==================== 事件相关类型 ====================

export interface InitProgressEvent {
  task_id: string;
  stage: string;
  message: string;
  percentage: number;
  current_step: number;
  total_steps: number;
}

export interface InitCompletedEvent {
  task_id: string;
  success: boolean;
  error?: string;
}

export interface DownloadProgressEvent {
  task_id: string;
  file_name: string;
  downloaded_bytes: number;
  total_bytes: number;
  download_speed: number;
  eta_seconds: number;
  percentage: number;
  status: string;
}

export interface DownloadCompletedEvent {
  task_id: string;
  success: boolean;
  error?: string;
}

export interface ServiceStatusUpdateEvent {
  name: string;
  status: string;
  health: string;
  uptime_seconds?: number;
  cpu_usage: number;
  memory_usage_mb: number;
  ports: string[];
}

// ==================== 配置相关类型 ====================

export interface UIConfiguration {
  [key: string]: unknown;
}

// ==================== 存储空间相关类型 ====================

export interface StorageInfo {
  path: string;
  total_bytes: number;
  available_bytes: number;
  used_bytes: number;
  available_space_gb: number;
  required_space_gb: number;
  sufficient: boolean;
}

// ==================== 路径建议类型 ====================

export interface PlatformPaths {
  suggested_work_dir: string;
  docker_install_guide: string;
  platform_tips: string[];
}

// ==================== 错误处理类型 ====================

export interface AppError {
  code: string;
  message: string;
  details?: string;
  recovery_suggestions?: string[];
}

// ==================== 组件Props类型 ====================

export interface ProgressBarProps {
  value: number;
  max?: number;
  showPercentage?: boolean;
  showSpeed?: boolean;
  speed?: number;
  eta?: number;
  className?: string;
}

export interface StatusIndicatorProps {
  status: 'success' | 'warning' | 'error' | 'loading' | 'idle';
  label?: string;
  size?: 'small' | 'medium' | 'large';
}

export interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  size?: 'small' | 'medium' | 'large';
}

// ==================== 页面路由类型 ====================

export type PageRoute = 
  | 'welcome'
  | 'dashboard'
  | 'services'
  | 'upgrades'
  | 'backups'
  | 'settings'
  | 'about';

export interface NavigationItem {
  route: PageRoute;
  label: string;
  icon: string;
  disabled?: boolean;
}

// ==================== Tauri API 封装类型 ====================

export interface TauriCommand<T = unknown> {
  command: string;
  args?: Record<string, unknown>;
  response?: T;
}

export interface TauriEvent<T = unknown> {
  event: string;
  payload: T;
}

// ==================== 钩子返回类型 ====================

export interface UseAppStateReturn {
  state: AppStateInfo | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export interface UseDownloadProgressReturn {
  progress: DownloadProgress | null;
  isDownloading: boolean;
  error: string | null;
  startDownload: (url: string, targetDir: string) => Promise<string>;
  cancelDownload: (taskId: string) => Promise<void>;
}

export interface UseServicesReturn {
  services: ServiceStatus[];
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  startMonitoring: () => Promise<void>;
  stopMonitoring: () => void;
}

// ==================== 工具函数类型 ====================

export interface FormatSizeOptions {
  precision?: number;
  locale?: string;
}

export interface FormatTimeOptions {
  showSeconds?: boolean;
  compact?: boolean;
}

// ==================== 常量定义 ====================

export const STORAGE_REQUIREMENTS = {
  MINIMUM_GB: 60,
  RECOMMENDED_GB: 80,
  DOCKER_PACKAGE_GB: 14,
  EXTRACTED_GB: 25,
  DATA_LOGS_GB: 10,
  BACKUP_RESERVE_GB: 15,
} as const;

export const DOWNLOAD_CHUNK_SIZE = 1024 * 1024; // 1MB

export const PLATFORMS: Platform[] = ['windows', 'macos', 'linux'];

export const APP_STATES: AppState[] = [
  'UNINITIALIZED',
  'INITIALIZING', 
  'DOWNLOADING',
  'DEPLOYING',
  'READY',
  'UPGRADING',
  'ERROR'
];

export const INIT_STAGES: InitStage[] = [
  'downloading',
  'extracting',
  'loading',
  'starting',
  'configuring'
];
