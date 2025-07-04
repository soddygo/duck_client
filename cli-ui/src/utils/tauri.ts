import { invoke } from '@tauri-apps/api/core';
import { Command } from '@tauri-apps/plugin-shell';
import { 
  open as openDialog, 
  save as saveDialog,
  message,
  ask,
  confirm
} from '@tauri-apps/plugin-dialog';
import {
  readDir,
  readTextFile,
  writeTextFile,
  exists,
  mkdir,
  remove,
  stat
} from '@tauri-apps/plugin-fs';
import {
  check as checkUpdate
} from '@tauri-apps/plugin-updater';
import { exit, relaunch } from '@tauri-apps/plugin-process';

// ============ Shell Commands ============
export class ShellManager {
  /**
   * 执行 duck-cli 命令（Sidecar 方式）
   */
  static async executeDuckCli(args: string[], workingDir?: string): Promise<{ stdout: string; stderr: string; code: number }> {
    try {
      const cmd = Command.sidecar('duck-cli', args, {
        cwd: workingDir
      });

      const output = await cmd.execute();
      return {
        stdout: output.stdout,
        stderr: output.stderr,
        code: output.code ?? 0
      };
    } catch (error) {
      console.error('Sidecar command failed:', error);
      throw error;
    }
  }

  /**
   * 执行系统 duck-cli 命令（Shell 方式）
   */
  static async executeSystemDuckCli(args: string[], workingDir?: string): Promise<{ stdout: string; stderr: string; code: number }> {
    try {
      const cmd = Command.create('duck-cli', args, {
        cwd: workingDir
      });

      const output = await cmd.execute();
      return {
        stdout: output.stdout,
        stderr: output.stderr,
        code: output.code ?? 0
      };
    } catch (error) {
      console.error('System command failed:', error);
      throw error;
    }
  }

  /**
   * 智能执行 duck-cli 命令（混合策略）
   */
  static async executeDuckCliSmart(args: string[], workingDir?: string): Promise<{ stdout: string; stderr: string; code: number }> {
    try {
      // 优先使用 Sidecar 方式
      return await this.executeDuckCli(args, workingDir);
    } catch (sidecarError) {
      console.warn('Sidecar failed, fallback to system command:', sidecarError);
      
      try {
        // 降级到系统命令
        return await this.executeSystemDuckCli(args, workingDir);
      } catch (systemError) {
        console.error('Both sidecar and system commands failed');
        throw new Error(`CLI execution failed: ${systemError}`);
      }
    }
  }
}

// ============ Dialog Manager ============
export class DialogManager {
  /**
   * 选择目录
   */
  static async selectDirectory(title = '选择工作目录'): Promise<string | null> {
    try {
      const selected = await openDialog({
        title,
        directory: true,
        multiple: false
      });
      return selected;
    } catch (error) {
      console.error('Directory selection failed:', error);
      return null;
    }
  }

  /**
   * 选择文件
   */
  static async selectFile(title = '选择文件', filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
    try {
      const selected = await openDialog({
        title,
        directory: false,
        multiple: false,
        filters
      });
      return selected;
    } catch (error) {
      console.error('File selection failed:', error);
      return null;
    }
  }

  /**
   * 保存文件对话框
   */
  static async saveFile(title = '保存文件', defaultPath?: string): Promise<string | null> {
    try {
      const path = await saveDialog({
        title,
        defaultPath
      });
      return path;
    } catch (error) {
      console.error('Save dialog failed:', error);
      return null;
    }
  }

  /**
   * 显示消息
   */
  static async showMessage(title: string, content: string, kind: 'info' | 'warning' | 'error' = 'info'): Promise<void> {
    await message(content, { title, kind });
  }

  /**
   * 询问用户
   */
  static async askUser(title: string, message: string): Promise<boolean> {
    return await ask(message, { title });
  }

  /**
   * 确认对话框
   */
  static async confirmAction(title: string, message: string): Promise<boolean> {
    return await confirm(message, { title });
  }
}

// ============ File System Manager ============
export class FileSystemManager {
  /**
   * 检查路径是否存在
   */
  static async pathExists(path: string): Promise<boolean> {
    try {
      return await exists(path);
    } catch {
      return false;
    }
  }

  /**
   * 获取目录内容
   */
  static async listDirectory(path: string): Promise<string[]> {
    try {
      const entries = await readDir(path);
      return entries.map(entry => entry.name);
    } catch (error) {
      console.error('Read directory failed:', error);
      return [];
    }
  }

  /**
   * 读取文本文件
   */
  static async readTextFile(path: string): Promise<string | null> {
    try {
      return await readTextFile(path);
    } catch (error) {
      console.error('Read file failed:', error);
      return null;
    }
  }

  /**
   * 写入文本文件
   */
  static async writeTextFile(path: string, content: string): Promise<boolean> {
    try {
      await writeTextFile(path, content);
      return true;
    } catch (error) {
      console.error('Write file failed:', error);
      return false;
    }
  }

  /**
   * 创建目录
   */
  static async createDirectory(path: string): Promise<boolean> {
    try {
      await mkdir(path, { recursive: true });
      return true;
    } catch (error) {
      console.error('Create directory failed:', error);
      return false;
    }
  }

  /**
   * 删除文件或目录
   */
  static async remove(path: string): Promise<boolean> {
    try {
      await remove(path, { recursive: true });
      return true;
    } catch (error) {
      console.error('Remove failed:', error);
      return false;
    }
  }

  /**
   * 获取文件信息
   */
  static async getFileInfo(path: string) {
    try {
      return await stat(path);
    } catch (error) {
      console.error('Get file info failed:', error);
      return null;
    }
  }

  /**
   * 验证目录是否有效（存在且有权限）
   */
  static async validateDirectory(path: string): Promise<{ valid: boolean; error?: string }> {
    try {
      const pathExists = await this.pathExists(path);
      if (!pathExists) {
        return { valid: false, error: '目录不存在' };
      }

      const info = await this.getFileInfo(path);
      if (!info?.isDirectory) {
        return { valid: false, error: '路径不是目录' };
      }

      // 尝试读取目录内容来测试权限
      await this.listDirectory(path);
      
      return { valid: true };
    } catch (error) {
      return { valid: false, error: `目录访问失败: ${error}` };
    }
  }
}

// ============ Update Manager ============
export class UpdateManager {
  /**
   * 检查更新
   */
  static async checkForUpdates() {
    try {
      const update = await checkUpdate();
      return update;
    } catch (error) {
      console.error('Check update failed:', error);
      throw error;
    }
  }

  /**
   * 下载并安装更新
   */
  static async downloadAndInstallUpdate(
    onProgress?: (downloaded: number, total: number) => void
  ): Promise<void> {
    try {
      // 先检查更新
      const update = await this.checkForUpdates();
      if (!update) {
        throw new Error('没有可用的更新');
      }

      // 下载并安装更新
      await update.downloadAndInstall((event: any) => {
        switch (event.event) {
          case 'Started':
            console.log(`开始下载 ${event.data?.contentLength ?? 0} bytes`);
            break;
          case 'Progress':
            if (onProgress && event.data) {
              onProgress(event.data.chunkLength, event.data.contentLength ?? 0);
            }
            break;
          case 'Finished':
            console.log('下载完成');
            break;
        }
      });
    } catch (error) {
      console.error('Update failed:', error);
      throw error;
    }
  }
}

// ============ Process Manager ============
export class ProcessManager {
  /**
   * 重启应用
   */
  static async restartApp(): Promise<void> {
    await relaunch();
  }

  /**
   * 退出应用
   */
  static async exitApp(code = 0): Promise<void> {
    await exit(code);
  }
}

// ============ Configuration Manager ============
export class ConfigManager {
  private static readonly CONFIG_DIR = 'duck-client';
  private static readonly CONFIG_FILE = 'config.json';

  /**
   * 获取配置文件路径
   */
  private static getConfigPath(): string {
    return `${this.CONFIG_DIR}/${this.CONFIG_FILE}`;
  }

  /**
   * 读取配置
   */
  static async loadConfig(): Promise<any> {
    try {
      const configPath = this.getConfigPath();
      const content = await FileSystemManager.readTextFile(configPath);
      return content ? JSON.parse(content) : {};
    } catch (error) {
      console.error('Load config failed:', error);
      return {};
    }
  }

  /**
   * 保存配置
   */
  static async saveConfig(config: any): Promise<boolean> {
    try {
      const configPath = this.getConfigPath();
      
      // 确保配置目录存在
      await FileSystemManager.createDirectory(this.CONFIG_DIR);
      
      const content = JSON.stringify(config, null, 2);
      return await FileSystemManager.writeTextFile(configPath, content);
    } catch (error) {
      console.error('Save config failed:', error);
      return false;
    }
  }

  /**
   * 获取工作目录
   */
  static async getWorkingDirectory(): Promise<string | null> {
    const config = await this.loadConfig();
    return config.workingDirectory || null;
  }

  /**
   * 设置工作目录
   */
  static async setWorkingDirectory(path: string): Promise<boolean> {
    const config = await this.loadConfig();
    config.workingDirectory = path;
    return await this.saveConfig(config);
  }
}

// ============ Duck CLI Manager ============
export class DuckCliManager {
  /**
   * 初始化项目
   */
  static async initialize(workingDir: string): Promise<{ success: boolean; output: string; error?: string }> {
    try {
      const result = await ShellManager.executeDuckCliSmart(['init'], workingDir);
      return {
        success: result.code === 0,
        output: result.stdout,
        error: result.stderr
      };
    } catch (error) {
      return {
        success: false,
        output: '',
        error: `初始化失败: ${error}`
      };
    }
  }

  /**
   * 检查服务状态
   */
  static async checkStatus(workingDir: string): Promise<{ success: boolean; output: string; error?: string }> {
    try {
      const result = await ShellManager.executeDuckCliSmart(['status'], workingDir);
      return {
        success: result.code === 0,
        output: result.stdout,
        error: result.stderr
      };
    } catch (error) {
      return {
        success: false,
        output: '',
        error: `状态检查失败: ${error}`
      };
    }
  }

  /**
   * 启动服务
   */
  static async startService(workingDir: string): Promise<{ success: boolean; output: string; error?: string }> {
    try {
      const result = await ShellManager.executeDuckCliSmart(['docker-service', 'start'], workingDir);
      return {
        success: result.code === 0,
        output: result.stdout,
        error: result.stderr
      };
    } catch (error) {
      return {
        success: false,
        output: '',
        error: `启动服务失败: ${error}`
      };
    }
  }

  /**
   * 停止服务
   */
  static async stopService(workingDir: string): Promise<{ success: boolean; output: string; error?: string }> {
    try {
      const result = await ShellManager.executeDuckCliSmart(['docker-service', 'stop'], workingDir);
      return {
        success: result.code === 0,
        output: result.stdout,
        error: result.stderr
      };
    } catch (error) {
      return {
        success: false,
        output: '',
        error: `停止服务失败: ${error}`
      };
    }
  }

  /**
   * 重启服务
   */
  static async restartService(workingDir: string): Promise<{ success: boolean; output: string; error?: string }> {
    try {
      const result = await ShellManager.executeDuckCliSmart(['docker-service', 'restart'], workingDir);
      return {
        success: result.code === 0,
        output: result.stdout,
        error: result.stderr
      };
    } catch (error) {
      return {
        success: false,
        output: '',
        error: `重启服务失败: ${error}`
      };
    }
  }

  /**
   * 一键部署
   */
  static async autoUpgradeDeploy(workingDir: string): Promise<{ success: boolean; output: string; error?: string }> {
    try {
      const result = await ShellManager.executeDuckCliSmart(['auto-upgrade-deploy', 'run'], workingDir);
      return {
        success: result.code === 0,
        output: result.stdout,
        error: result.stderr
      };
    } catch (error) {
      return {
        success: false,
        output: '',
        error: `一键部署失败: ${error}`
      };
    }
  }

  /**
   * 检查更新
   */
  static async checkCliUpdate(workingDir: string): Promise<{ success: boolean; output: string; error?: string }> {
    try {
      const result = await ShellManager.executeDuckCliSmart(['check-update', 'check'], workingDir);
      return {
        success: result.code === 0,
        output: result.stdout,
        error: result.stderr
      };
    } catch (error) {
      return {
        success: false,
        output: '',
        error: `检查更新失败: ${error}`
      };
    }
  }

  /**
   * 升级服务
   */
  static async upgradeService(workingDir: string, full = false): Promise<{ success: boolean; output: string; error?: string }> {
    try {
      const args = full ? ['upgrade', '--full'] : ['upgrade'];
      const result = await ShellManager.executeDuckCliSmart(args, workingDir);
      return {
        success: result.code === 0,
        output: result.stdout,
        error: result.stderr
      };
    } catch (error) {
      return {
        success: false,
        output: '',
        error: `升级服务失败: ${error}`
      };
    }
  }

  /**
   * 创建备份
   */
  static async createBackup(workingDir: string): Promise<{ success: boolean; output: string; error?: string }> {
    try {
      const result = await ShellManager.executeDuckCliSmart(['backup', 'create'], workingDir);
      return {
        success: result.code === 0,
        output: result.stdout,
        error: result.stderr
      };
    } catch (error) {
      return {
        success: false,
        output: '',
        error: `创建备份失败: ${error}`
      };
    }
  }

  /**
   * 回滚服务
   */
  static async rollbackService(workingDir: string): Promise<{ success: boolean; output: string; error?: string }> {
    try {
      const result = await ShellManager.executeDuckCliSmart(['backup', 'restore', '--latest'], workingDir);
      return {
        success: result.code === 0,
        output: result.stdout,
        error: result.stderr
      };
    } catch (error) {
      return {
        success: false,
        output: '',
        error: `回滚服务失败: ${error}`
      };
    }
  }
} 