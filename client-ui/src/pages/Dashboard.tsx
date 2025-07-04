import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { 
  Card, 
  Row, 
  Col, 
  Button, 
  Badge, 
  Typography, 
  Space, 
  Alert,
  List,
  Spin,
  Statistic,
  message,
  Divider,
  Modal,
  Progress,
  Descriptions,
  InputNumber,
  Switch,
  Select,
  Input
} from 'antd';
import {
  PlayCircleOutlined,
  PauseCircleOutlined,
  ReloadOutlined,
  UploadOutlined,
  ClockCircleOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  SettingOutlined,
  DatabaseOutlined,
  CloudUploadOutlined,
  PoweroffOutlined,
  StopOutlined,
  SecurityScanOutlined,
  AppstoreOutlined,
  ControlOutlined
} from '@ant-design/icons';
import { listen } from '@tauri-apps/api/event';

const { Title, Text } = Typography;

interface ServiceStatus {
  isRunning: boolean;
  containers: number;
  uptime: string;
}

interface VersionInfo {
  client_version: string;
  service_version: string;
  has_update: boolean;
  latest_version?: string;
}

interface ActivityLog {
  id: string;
  timestamp: string;
  type: 'success' | 'error' | 'info';
  message: string;
}

interface UpgradeInfo {
  current_version: string;
  latest_version: string;
  has_update: boolean;
  release_notes?: string;
  download_size_mb?: number;
  estimated_download_time?: string;
}

const Dashboard: React.FC = () => {
  const [serviceStatus, setServiceStatus] = useState<ServiceStatus>({
    isRunning: true,
    containers: 5,
    uptime: '2小时30分钟'
  });
  
  const [versionInfo, setVersionInfo] = useState<VersionInfo>({
    client_version: '加载中...',
    service_version: '加载中...',
    has_update: false
  });
  
  const [activityLogs, setActivityLogs] = useState<ActivityLog[]>([
    {
      id: '1',
      timestamp: '2024-01-20 10:30',
      type: 'success',
      message: '服务启动成功'
    },
    {
      id: '2', 
      timestamp: '2024-01-20 09:15',
      type: 'info',
      message: '检查更新完成，当前已是最新版本'
    },
    {
      id: '3', 
      timestamp: '2024-01-20 09:10',
      type: 'success',
      message: 'Duck Client 初始化完成'
    }
  ]);
  
  const [loading, setLoading] = useState(false);
  const [upgradeModalVisible, setUpgradeModalVisible] = useState(false);
  const [upgradeInfo, setUpgradeInfo] = useState<UpgradeInfo | null>(null);
  const [upgradeDownloading, setUpgradeDownloading] = useState(false);
  const [upgradeProgress, setUpgradeProgress] = useState(0);
  const [serviceSettingsVisible, setServiceSettingsVisible] = useState(false);
  
  // 工作目录相关状态
  const [currentWorkingDirectory, setCurrentWorkingDirectory] = useState<string>('');
  const [workingDirectoryModalVisible, setWorkingDirectoryModalVisible] = useState(false);
  const [newWorkingDirectory, setNewWorkingDirectory] = useState<string>('');

  // 模拟数据加载
  useEffect(() => {
    loadServiceStatus();
    loadVersionInfo();
    checkForUpdates();
    loadWorkingDirectory();
    
    // 设置升级进度事件监听
    const setupUpgradeListeners = async () => {
      // 监听升级进度事件
      const unlistenProgress = await listen('upgrade-progress', (event: any) => {
        const progressData = event.payload;
        console.log('升级进度:', progressData);
        
        setUpgradeProgress(progressData.percentage);
        
        // 添加进度日志
        const newLog: ActivityLog = {
          id: Date.now().toString(),
          timestamp: new Date().toLocaleString('zh-CN'),
          type: progressData.status === 'failed' ? 'error' : 'info',
          message: `${progressData.stage}: ${progressData.message}`
        };
        setActivityLogs(prev => [newLog, ...prev.slice(0, 9)]);
      });
      
      // 监听升级完成事件
      const unlistenCompleted = await listen('upgrade-download-completed', (event: any) => {
        const completedData = event.payload;
        console.log('升级完成:', completedData);
        
        setUpgradeDownloading(false);
        
        if (completedData.success) {
          setUpgradeProgress(100);
          setUpgradeModalVisible(false);
          
          const successLog: ActivityLog = {
            id: Date.now().toString(),
            timestamp: new Date().toLocaleString('zh-CN'),
            type: 'success',
            message: '升级包下载完成'
          };
          setActivityLogs(prev => [successLog, ...prev.slice(0, 9)]);
          
          message.success('升级包下载完成');
        } else {
          const errorMessage = completedData.error || '升级下载失败';
          
          const errorLog: ActivityLog = {
            id: Date.now().toString(),
            timestamp: new Date().toLocaleString('zh-CN'),
            type: 'error',
            message: `升级下载失败: ${errorMessage}`
          };
          setActivityLogs(prev => [errorLog, ...prev.slice(0, 9)]);
          
          message.error(`升级下载失败: ${errorMessage}`);
        }
      });
      
      // 清理函数
      return () => {
        unlistenProgress();
        unlistenCompleted();
      };
    };
    
    let cleanup: (() => void) | undefined;
    setupUpgradeListeners().then(cleanupFn => {
      cleanup = cleanupFn;
    });
    
    return () => {
      if (cleanup) {
        cleanup();
      }
    };
  }, []);

  const loadServiceStatus = async () => {
    setLoading(true);
    try {
      // 实际应用中应该调用真实的API
      // const status: any = await invoke('get_service_status');
      setServiceStatus({
        isRunning: true,
        containers: 5,
        uptime: '2小时30分钟'
      });
    } catch (error) {
      console.error('Failed to load service status:', error);
      message.error('获取服务状态失败');
    } finally {
      setLoading(false);
    }
  };

  const loadVersionInfo = async () => {
    try {
      const versionData = await invoke('get_version_info') as VersionInfo;
      setVersionInfo(versionData);
      
      const newLog: ActivityLog = {
        id: Date.now().toString(),
        timestamp: new Date().toLocaleString('zh-CN'),
        type: 'success',
        message: `版本信息加载完成 - 客户端: ${versionData.client_version}, 服务: ${versionData.service_version}`
      };
      setActivityLogs(prev => [newLog, ...prev.slice(0, 9)]);
    } catch (error) {
      console.error('Failed to load version info:', error);
      setVersionInfo({
        client_version: '获取失败',
        service_version: '获取失败',
        has_update: false
      });
      
      const errorLog: ActivityLog = {
        id: Date.now().toString(),
        timestamp: new Date().toLocaleString('zh-CN'),
        type: 'error',
        message: '版本信息加载失败: ' + (error as string)
      };
      setActivityLogs(prev => [errorLog, ...prev.slice(0, 9)]);
    }
  };

  const checkForUpdates = async () => {
    try {
      // 检查升级时也更新版本信息中的has_update字段
      const upgradeInfo = await invoke('check_upgrade_available') as any;
      
      setVersionInfo(prev => ({
        ...prev,
        has_update: upgradeInfo.has_update,
        latest_version: upgradeInfo.latest_version
      }));
      
      if (upgradeInfo.has_update) {
        message.info(`发现新版本 ${upgradeInfo.latest_version}`);
        
        const newLog: ActivityLog = {
          id: Date.now().toString(),
          timestamp: new Date().toLocaleString('zh-CN'),
          type: 'info',
          message: `检查更新完成，发现新版本 ${upgradeInfo.latest_version}`
        };
        setActivityLogs(prev => [newLog, ...prev.slice(0, 9)]);
      } else {
        message.info('检查更新完成，当前已是最新版本');
        
        const newLog: ActivityLog = {
          id: Date.now().toString(),
          timestamp: new Date().toLocaleString('zh-CN'),
          type: 'success',
          message: '检查更新完成，当前已是最新版本'
        };
        setActivityLogs(prev => [newLog, ...prev.slice(0, 9)]);
      }
    } catch (error) {
      console.error('Failed to check updates:', error);
      message.error('检查更新失败: ' + (error as string));
      
      const errorLog: ActivityLog = {
        id: Date.now().toString(),
        timestamp: new Date().toLocaleString('zh-CN'),
        type: 'error',
        message: '检查更新失败: ' + (error as string)
      };
      setActivityLogs(prev => [errorLog, ...prev.slice(0, 9)]);
    }
  };

  const handleServiceControl = async (action: 'start' | 'stop' | 'restart') => {
    setLoading(true);
    try {
      // 模拟服务控制
      // await invoke(`${action}_service`);
      
      const actionText = action === 'start' ? '启动' : action === 'stop' ? '停止' : '重启';
      message.success(`服务${actionText}成功`);
      
      const newLog: ActivityLog = {
        id: Date.now().toString(),
        timestamp: new Date().toLocaleString('zh-CN'),
        type: 'success',
        message: `服务${actionText}成功`
      };
      
      setActivityLogs(prev => [newLog, ...prev.slice(0, 9)]);
      
      // 模拟状态更新
      if (action === 'start') {
        setServiceStatus(prev => ({ ...prev, isRunning: true }));
      } else if (action === 'stop') {
        setServiceStatus(prev => ({ ...prev, isRunning: false, uptime: '0分钟' }));
      }
      
    } catch (error) {
      console.error(`Failed to ${action} service:`, error);
      const actionText = action === 'start' ? '启动' : action === 'stop' ? '停止' : '重启';
      message.error(`服务${actionText}失败`);
    } finally {
      setLoading(false);
    }
  };

  const handleUpgrade = async () => {
    try {
      const hide = message.loading('正在检查升级...', 0);
      
      // 调用 Tauri 命令检查升级
      const upgradeInfo = await invoke('check_upgrade_available') as UpgradeInfo;
      
      hide();
      
      if (upgradeInfo.has_update) {
        setUpgradeInfo(upgradeInfo);
        setUpgradeModalVisible(true);
        
        const newLog: ActivityLog = {
          id: Date.now().toString(),
          timestamp: new Date().toLocaleString('zh-CN'),
          type: 'info',
          message: `发现新版本 ${upgradeInfo.latest_version}`
        };
        setActivityLogs(prev => [newLog, ...prev.slice(0, 9)]);
      } else {
        message.success('当前已是最新版本');
        
        const newLog: ActivityLog = {
          id: Date.now().toString(),
          timestamp: new Date().toLocaleString('zh-CN'),
          type: 'success',
          message: '检查更新完成，当前已是最新版本'
        };
        setActivityLogs(prev => [newLog, ...prev.slice(0, 9)]);
      }
    } catch (error) {
      message.error('升级检查失败: ' + (error as string));
      console.error('Failed to check upgrade:', error);
    }
  };

  const handleServiceSettings = () => {
    setServiceSettingsVisible(true);
  };

  const handleStartUpgradeDownload = async () => {
    if (!upgradeInfo) return;
    
    try {
      setUpgradeDownloading(true);
      setUpgradeProgress(0);
      
      // 调用 Tauri 命令开始升级下载
      const taskId = await invoke('start_upgrade_download') as string;
      
      const newLog: ActivityLog = {
        id: Date.now().toString(),
        timestamp: new Date().toLocaleString('zh-CN'),
        type: 'info',
        message: `开始下载升级包 ${upgradeInfo.latest_version} (任务ID: ${taskId})`
      };
      setActivityLogs(prev => [newLog, ...prev.slice(0, 9)]);
      
      message.success('开始下载升级包');
      
    } catch (error) {
      message.error('开始升级下载失败: ' + (error as string));
      setUpgradeDownloading(false);
      console.error('Failed to start upgrade download:', error);
    }
  };

  const getStatusBadge = () => {
    if (loading) {
      return <Badge status="processing" text="处理中..." />;
    }
    return serviceStatus.isRunning 
      ? <Badge status="success" text="运行中" />
      : <Badge status="error" text="已停止" />;
  };

  const getStatusIcon = () => {
    if (loading) return <Spin size="small" />;
    return serviceStatus.isRunning 
      ? <CheckCircleOutlined style={{ color: '#52c41a', fontSize: 24 }} />
      : <ExclamationCircleOutlined style={{ color: '#ff4d4f', fontSize: 24 }} />;
  };

  // 加载当前工作目录
  const loadWorkingDirectory = async () => {
    try {
      const appState = await invoke('get_app_state') as any;
      if (appState.working_directory) {
        setCurrentWorkingDirectory(appState.working_directory);
      }
    } catch (error) {
      console.error('获取工作目录失败:', error);
    }
  };

  // 重设工作目录
  const handleResetWorkingDirectory = async () => {
    if (!newWorkingDirectory.trim()) {
      message.error('请输入有效的工作目录路径');
      return;
    }

    try {
      await invoke('set_working_directory', { directory: newWorkingDirectory });
      
      // 显示警告对话框
      Modal.confirm({
        title: '⚠️ 重要警告',
        content: (
          <div>
            <p>您已成功更改工作目录，但请注意：</p>
            <ul style={{ marginTop: 16, paddingLeft: 20 }}>
              <li>切换工作目录需要重新初始化应用</li>
              <li>原工作目录中的所有数据将无法访问</li>
              <li>如需保留数据，请提前做好备份</li>
              <li>应用将重新启动以应用更改</li>
            </ul>
          </div>
        ),
        okText: '我已备份，继续重启',
        cancelText: '取消',
        onOk: () => {
          // 重启应用
          window.location.reload();
        },
        onCancel: () => {
          // 恢复到原来的工作目录
          invoke('set_working_directory', { directory: currentWorkingDirectory });
        }
      });

      setWorkingDirectoryModalVisible(false);
      setNewWorkingDirectory('');
      
    } catch (error) {
      message.error('设置工作目录失败: ' + (error as string));
    }
  };

  // 打开工作目录
  const handleOpenWorkingDirectory = async () => {
    try {
      if (currentWorkingDirectory) {
        await invoke('open_file_manager', { path: currentWorkingDirectory });
      }
    } catch (error) {
      message.error('打开工作目录失败: ' + (error as string));
    }
  };

  return (
    <div style={{ padding: '24px', background: '#f0f2f5', minHeight: '100vh' }}>
      {/* 页面标题 */}
      <div style={{ marginBottom: '24px' }}>
        <Title level={2} style={{ margin: 0, display: 'flex', alignItems: 'center', gap: '12px' }}>
          🦆 Duck Client 控制面板
        </Title>
        <Text type="secondary">Docker 服务管理中心</Text>
      </div>
      
      {/* 服务状态卡片 */}
      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={12} md={8}>
          <Card hoverable>
            <Statistic
              title={
                <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <DatabaseOutlined />
                  服务状态
                </span>
              }
              value=""
              formatter={() => (
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                {getStatusIcon()}
                {getStatusBadge()}
              </div>
              )}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={8}>
          <Card hoverable>
            <Statistic
              title="运行容器"
              value={serviceStatus.containers}
              suffix="个"
              valueStyle={{ color: '#3f8600' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={8}>
          <Card hoverable>
            <Statistic
              title="运行时间"
              value={serviceStatus.uptime}
              prefix={<ClockCircleOutlined />}
              valueStyle={{ color: '#1890ff' }}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        {/* 服务控制面板 */}
        <Col xs={24} lg={12}>
          <Card 
            title={
              <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <SettingOutlined />
                服务控制
              </span>
            } 
            hoverable
          >
            <Space direction="vertical" style={{ width: '100%' }}>
              <Row gutter={[8, 8]}>
                <Col span={8}>
                  <Button
                    type="primary"
                    icon={<PlayCircleOutlined />}
                    onClick={() => handleServiceControl('start')}
                    disabled={serviceStatus.isRunning || loading}
                    block
                  >
                    启动
                  </Button>
                </Col>
                <Col span={8}>
                  <Button
                    danger
                    icon={<PauseCircleOutlined />}
                    onClick={() => handleServiceControl('stop')}
                    disabled={!serviceStatus.isRunning || loading}
                    block
                  >
                    停止
                  </Button>
                </Col>
                <Col span={8}>
                  <Button
                    icon={<ReloadOutlined />}
                    onClick={() => handleServiceControl('restart')}
                    disabled={loading}
                    block
                  >
                    重启
                  </Button>
                </Col>
              </Row>
              
              <Divider style={{ margin: '16px 0' }} />
              
              <Row gutter={[8, 8]}>
                <Col span={12}>
                  <Button
                    icon={<CloudUploadOutlined />}
                    onClick={handleUpgrade}
                    block
                  >
                    检查升级
                  </Button>
                </Col>
                <Col span={12}>
                  <Button
                    icon={<SettingOutlined />}
                    onClick={handleServiceSettings}
                    block
                  >
                    服务设置
                  </Button>
                </Col>
              </Row>
            </Space>
          </Card>
        </Col>

        {/* 版本信息 */}
        <Col xs={24} lg={12}>
          <Card 
            title={
              <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <UploadOutlined />
                版本信息
              </span>
            } 
            hoverable
          >
            <Space direction="vertical" style={{ width: '100%' }}>
              <Row>
                <Col span={12}>
                  <Text strong>客户端版本：</Text>
                </Col>
                <Col span={12}>
                  <Text code>{versionInfo.client_version}</Text>
                </Col>
              </Row>
              <Row>
                <Col span={12}>
                  <Text strong>服务版本：</Text>
                </Col>
                <Col span={12}>
                  <Text code>{versionInfo.service_version}</Text>
                </Col>
              </Row>
              
              {versionInfo.has_update && versionInfo.latest_version && (
                <Alert
                  message={`发现新版本 ${versionInfo.latest_version}`}
                  type="info"
                  showIcon
                  action={
                    <Button size="small" type="primary" icon={<UploadOutlined />} onClick={handleUpgrade}>
                      立即升级
                    </Button>
                  }
                />
              )}
              
              {!versionInfo.has_update && versionInfo.client_version !== '加载中...' && (
                <Alert
                  message="当前已是最新版本"
                  type="success"
                  showIcon
                />
              )}
            </Space>
          </Card>
        </Col>
      </Row>

      {/* 最近活动 */}
      <Row>
        <Col span={24}>
          <Card title="最近活动" hoverable>
            <List
              size="small"
              dataSource={activityLogs}
                             renderItem={(item: ActivityLog) => (
                <List.Item>
                  <List.Item.Meta
                    avatar={
                      item.type === 'success' ? (
                        <CheckCircleOutlined style={{ color: '#52c41a' }} />
                      ) : item.type === 'error' ? (
                        <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />
                      ) : (
                        <ClockCircleOutlined style={{ color: '#1677ff' }} />
                      )
                    }
                    title={item.message}
                    description={
                      <Text type="secondary" style={{ fontSize: '12px' }}>
                        {item.timestamp}
                      </Text>
                    }
                  />
                </List.Item>
              )}
            />
          </Card>
        </Col>
      </Row>

      {/* 升级确认对话框 */}
      <Modal
        title="发现新版本"
        open={upgradeModalVisible}
        onCancel={() => setUpgradeModalVisible(false)}
        footer={null}
        width={600}
      >
        {upgradeInfo && (
          <div>
            <Alert
              message={`发现新版本 ${upgradeInfo.latest_version}`}
              description={`当前版本: ${upgradeInfo.current_version}`}
              type="info"
              showIcon
              style={{ marginBottom: 16 }}
            />
            
            {upgradeInfo.release_notes && (
              <div style={{ marginBottom: 16 }}>
                <Text strong>更新说明：</Text>
                <div style={{ 
                  marginTop: 8, 
                  padding: 12, 
                  background: '#f5f5f5', 
                  borderRadius: 4,
                  whiteSpace: 'pre-line'
                }}>
                  {upgradeInfo.release_notes}
                </div>
              </div>
            )}
            
            <Space direction="vertical" style={{ width: '100%', marginBottom: 16 }}>
              {upgradeInfo.download_size_mb && (
                <div>
                  <Text strong>下载大小：</Text>
                  <Text>{(upgradeInfo.download_size_mb / 1024).toFixed(1)} GB</Text>
                </div>
              )}
              {upgradeInfo.estimated_download_time && (
                <div>
                  <Text strong>预计时间：</Text>
                  <Text>{upgradeInfo.estimated_download_time}</Text>
                </div>
              )}
            </Space>
            
            {upgradeDownloading && (
              <div style={{ marginBottom: 16 }}>
                <Text strong>下载进度：</Text>
                <Progress 
                  percent={upgradeProgress} 
                  status={upgradeProgress === 100 ? 'success' : 'active'}
                  style={{ marginTop: 8 }}
                />
              </div>
            )}
            
            <div style={{ textAlign: 'right' }}>
              <Space>
                <Button 
                  onClick={() => setUpgradeModalVisible(false)}
                  disabled={upgradeDownloading}
                >
                  取消
                </Button>
                <Button 
                  type="primary" 
                  onClick={handleStartUpgradeDownload}
                  loading={upgradeDownloading}
                  icon={<CloudUploadOutlined />}
                >
                  {upgradeDownloading ? '下载中...' : '确认下载'}
                </Button>
              </Space>
            </div>
          </div>
        )}
      </Modal>

      {/* 服务设置对话框 */}
      <Modal
        title="服务设置"
        open={serviceSettingsVisible}
        onCancel={() => setServiceSettingsVisible(false)}
        footer={null}
        width={700}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Card size="small" title="端口配置">
            <Row gutter={16}>
              <Col span={12}>
                <Text strong>HTTP端口：</Text>
                <Text code style={{ marginLeft: 8 }}>8080</Text>
              </Col>
              <Col span={12}>
                <Text strong>HTTPS端口：</Text>
                <Text code style={{ marginLeft: 8 }}>8443</Text>
              </Col>
            </Row>
            <Row gutter={16} style={{ marginTop: 8 }}>
              <Col span={12}>
                <Text strong>数据库端口：</Text>
                <Text code style={{ marginLeft: 8 }}>5432</Text>
              </Col>
              <Col span={12}>
                <Text strong>Redis端口：</Text>
                <Text code style={{ marginLeft: 8 }}>6379</Text>
              </Col>
            </Row>
          </Card>

          <Card size="small" title="资源限制">
            <Row gutter={16}>
              <Col span={12}>
                <Text strong>CPU限制：</Text>
                <Text code style={{ marginLeft: 8 }}>2 核</Text>
              </Col>
              <Col span={12}>
                <Text strong>内存限制：</Text>
                <Text code style={{ marginLeft: 8 }}>4GB</Text>
              </Col>
            </Row>
            <Row gutter={16} style={{ marginTop: 8 }}>
              <Col span={12}>
                <Text strong>磁盘限制：</Text>
                <Text code style={{ marginLeft: 8 }}>50GB</Text>
              </Col>
              <Col span={12}>
                <Text strong>网络限制：</Text>
                <Text code style={{ marginLeft: 8 }}>100Mbps</Text>
              </Col>
            </Row>
          </Card>

          <Card size="small" title="日志设置">
            <Row gutter={16}>
              <Col span={12}>
                <Text strong>日志级别：</Text>
                <Text code style={{ marginLeft: 8 }}>INFO</Text>
              </Col>
              <Col span={12}>
                <Text strong>日志保留天数：</Text>
                <Text code style={{ marginLeft: 8 }}>30天</Text>
              </Col>
            </Row>
            <Row gutter={16} style={{ marginTop: 8 }}>
              <Col span={12}>
                <Text strong>最大日志大小：</Text>
                <Text code style={{ marginLeft: 8 }}>100MB</Text>
              </Col>
              <Col span={12}>
                <Text strong>日志滚动：</Text>
                <Text code style={{ marginLeft: 8 }}>启用</Text>
              </Col>
            </Row>
          </Card>

          <Card size="small" title="自动化设置">
            <Row gutter={16}>
              <Col span={12}>
                <Text strong>自动启动：</Text>
                <Badge status="success" text="已启用" />
              </Col>
              <Col span={12}>
                <Text strong>自动重启：</Text>
                <Badge status="success" text="已启用" />
              </Col>
            </Row>
            <Row gutter={16} style={{ marginTop: 8 }}>
              <Col span={12}>
                <Text strong>健康检查：</Text>
                <Badge status="success" text="已启用" />
              </Col>
              <Col span={12}>
                <Text strong>自动更新：</Text>
                <Badge status="default" text="已禁用" />
              </Col>
            </Row>
          </Card>

          <Card size="small" title="工作目录管理">
            <Space direction="vertical" style={{ width: '100%' }}>
              <div>
                <Text strong>当前工作目录：</Text>
                <div style={{ 
                  marginTop: 8, 
                  padding: 8, 
                  background: '#f5f5f5', 
                  borderRadius: 4,
                  fontFamily: 'monospace',
                  fontSize: '13px',
                  wordBreak: 'break-all'
                }}>
                  {currentWorkingDirectory || '未设置'}
                </div>
              </div>
              
              <Row gutter={8}>
                <Col span={12}>
                  <Button 
                    icon={<ControlOutlined />}
                    onClick={handleOpenWorkingDirectory}
                    disabled={!currentWorkingDirectory}
                    block
                  >
                    打开目录
                  </Button>
                </Col>
                <Col span={12}>
                  <Button 
                    icon={<SettingOutlined />}
                    onClick={() => {
                      setNewWorkingDirectory(currentWorkingDirectory);
                      setWorkingDirectoryModalVisible(true);
                    }}
                    block
                    type="primary"
                    danger
                  >
                    重设目录
                  </Button>
                </Col>
              </Row>
              
              <Alert
                message="注意事项"
                description="重设工作目录会导致应用重新初始化，原有数据将无法访问，请提前备份重要数据。"
                type="warning"
                showIcon
                style={{ fontSize: '12px' }}
              />
            </Space>
          </Card>

          <div style={{ textAlign: 'right', marginTop: 16 }}>
            <Space>
              <Button onClick={() => setServiceSettingsVisible(false)}>
                关闭
              </Button>
              <Button type="primary">
                保存设置
              </Button>
            </Space>
          </div>
        </Space>
      </Modal>

      {/* 重设工作目录对话框 */}
      <Modal
        title="⚠️ 重设工作目录"
        open={workingDirectoryModalVisible}
        onCancel={() => {
          setWorkingDirectoryModalVisible(false);
          setNewWorkingDirectory('');
        }}
        footer={null}
        width={600}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Alert
            message="重要警告"
            description="更改工作目录是一个危险操作，请仔细阅读以下说明："
            type="error"
            showIcon
          />
          
          <div style={{ padding: 16, background: '#fff2f0', border: '1px solid #ffccc7', borderRadius: 4 }}>
            <Text strong style={{ color: '#cf1322' }}>操作风险：</Text>
            <ul style={{ marginTop: 8, marginBottom: 0, paddingLeft: 20 }}>
              <li>应用将重新初始化，当前所有设置将丢失</li>
              <li>原工作目录中的数据库、配置文件等将无法访问</li>
              <li>所有服务状态和历史记录将重置</li>
              <li>如需保留数据，请提前手动备份</li>
            </ul>
          </div>
          
          <div>
            <Text strong>当前工作目录：</Text>
            <div style={{ 
              marginTop: 8, 
              padding: 8, 
              background: '#f5f5f5', 
              borderRadius: 4,
              fontFamily: 'monospace',
              fontSize: '13px',
              wordBreak: 'break-all'
            }}>
              {currentWorkingDirectory}
            </div>
          </div>
          
          <div>
            <Text strong>新工作目录：</Text>
            <Input
              value={newWorkingDirectory}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setNewWorkingDirectory(e.target.value)}
              placeholder="请输入新的工作目录路径，例如：/Users/username/Documents/DuckClient"
              style={{ marginTop: 8 }}
            />
          </div>
          
          <Alert
            message="确认操作"
            description="点击'确认重设'后，应用将立即重启并使用新的工作目录。请确保已备份重要数据。"
            type="info"
            showIcon
          />
          
          <div style={{ textAlign: 'right' }}>
            <Space>
              <Button onClick={() => {
                setWorkingDirectoryModalVisible(false);
                setNewWorkingDirectory('');
              }}>
                取消
              </Button>
              <Button 
                type="primary" 
                danger
                onClick={handleResetWorkingDirectory}
                disabled={!newWorkingDirectory.trim() || newWorkingDirectory === currentWorkingDirectory}
              >
                确认重设
              </Button>
            </Space>
          </div>
        </Space>
      </Modal>
    </div>
  );
};

export default Dashboard; 