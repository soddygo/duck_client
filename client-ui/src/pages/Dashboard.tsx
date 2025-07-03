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
  Divider
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
  CloudUploadOutlined
} from '@ant-design/icons';

const { Title, Text } = Typography;

interface ServiceStatus {
  isRunning: boolean;
  containers: number;
  uptime: string;
}

interface VersionInfo {
  clientVersion: string;
  serviceVersion: string;
  hasUpdate: boolean;
  latestVersion?: string;
}

interface ActivityLog {
  id: string;
  timestamp: string;
  type: 'success' | 'error' | 'info';
  message: string;
}

const Dashboard: React.FC = () => {
  const [serviceStatus, setServiceStatus] = useState<ServiceStatus>({
    isRunning: true,
    containers: 5,
    uptime: '2小时30分钟'
  });
  
  const [versionInfo, setVersionInfo] = useState<VersionInfo>({
    clientVersion: '1.0.10',
    serviceVersion: '1.2.0',
    hasUpdate: false
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

  // 模拟数据加载
  useEffect(() => {
    loadServiceStatus();
    checkForUpdates();
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

  const checkForUpdates = async () => {
    try {
      // 模拟检查更新
      setVersionInfo({
        clientVersion: '1.0.10',
        serviceVersion: '1.2.0',
        hasUpdate: false
      });
      message.info('检查更新完成，当前已是最新版本');
    } catch (error) {
      console.error('Failed to check updates:', error);
      message.error('检查更新失败');
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
      message.loading('正在检查升级...', 2);
      // 模拟升级检查
      setTimeout(() => {
        message.success('当前已是最新版本');
      }, 2000);
    } catch (error) {
      message.error('升级检查失败');
    }
  };

  const handleServiceSettings = () => {
    message.info('服务设置功能开发中，敬请期待！');
    // TODO: 打开服务设置页面或模态框
    // 可以在这里添加跳转到设置页面的逻辑
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
                  <Text code>{versionInfo.clientVersion}</Text>
                </Col>
              </Row>
              <Row>
                <Col span={12}>
                  <Text strong>服务版本：</Text>
                </Col>
                <Col span={12}>
                  <Text code>{versionInfo.serviceVersion}</Text>
                </Col>
              </Row>
              
              {versionInfo.hasUpdate && (
                <Alert
                  message={`发现新版本 ${versionInfo.latestVersion}`}
                  type="info"
                  showIcon
                  action={
                    <Button size="small" type="primary" icon={<UploadOutlined />}>
                      立即升级
                    </Button>
                  }
                />
              )}
              
              {!versionInfo.hasUpdate && (
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
    </div>
  );
};

export default Dashboard; 