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
  message
} from 'antd';
import {
  PlayCircleOutlined,
  PauseCircleOutlined,
  ReloadOutlined,
  UploadOutlined,
  ClockCircleOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
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
    isRunning: false,
    containers: 0,
    uptime: '0分钟'
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
      const status: any = await invoke('get_service_status');
      setServiceStatus({
        isRunning: status.is_running,
        containers: status.containers,
        uptime: status.uptime
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
      const updateInfo: any = await invoke('check_updates');
      setVersionInfo({
        clientVersion: updateInfo.client_version,
        serviceVersion: updateInfo.service_version,
        hasUpdate: updateInfo.has_update,
        latestVersion: updateInfo.latest_version
      });
    } catch (error) {
      console.error('Failed to check updates:', error);
      message.error('检查更新失败');
    }
  };

  const handleServiceControl = async (action: 'start' | 'stop' | 'restart') => {
    setLoading(true);
    try {
      await invoke(`${action}_service`);
      
      const actionText = action === 'start' ? '启动' : action === 'stop' ? '停止' : '重启';
      message.success(`服务${actionText}成功`);
      
      const newLog: ActivityLog = {
        id: Date.now().toString(),
        timestamp: new Date().toLocaleString('zh-CN'),
        type: 'success',
        message: `服务${actionText}成功`
      };
      
      setActivityLogs(prev => [newLog, ...prev.slice(0, 9)]);
      
      // 重新加载服务状态
      await loadServiceStatus();
    } catch (error) {
      console.error(`Failed to ${action} service:`, error);
      const actionText = action === 'start' ? '启动' : action === 'stop' ? '停止' : '重启';
      message.error(`服务${actionText}失败`);
    } finally {
      setLoading(false);
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

  return (
    <div>
      <Title level={2}>仪表盘</Title>
      
      {/* 服务状态卡片 */}
      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={12} md={8}>
          <Card>
            <div>
              <div style={{ marginBottom: 8, color: '#00000073', fontSize: 14 }}>服务状态</div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                {getStatusIcon()}
                {getStatusBadge()}
              </div>
            </div>
          </Card>
        </Col>
        <Col xs={24} sm={12} md={8}>
          <Card>
            <Statistic
              title="运行容器"
              value={serviceStatus.containers}
              suffix="个"
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={8}>
          <Card>
            <Statistic
              title="运行时间"
              value={serviceStatus.uptime}
              prefix={<ClockCircleOutlined />}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        {/* 服务控制面板 */}
        <Col xs={24} md={12}>
          <Card title="服务控制" size="small">
            <Space direction="vertical" style={{ width: '100%' }}>
              <Row gutter={8}>
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
            </Space>
          </Card>
        </Col>

        {/* 版本信息 */}
        <Col xs={24} md={12}>
          <Card title="版本信息" size="small">
            <Space direction="vertical" style={{ width: '100%' }}>
              <div>
                <Text>客户端版本：</Text>
                <Text strong>{versionInfo.clientVersion}</Text>
              </div>
              <div>
                <Text>服务版本：</Text>
                <Text strong>{versionInfo.serviceVersion}</Text>
              </div>
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
            </Space>
          </Card>
        </Col>
      </Row>

      {/* 最近活动 */}
      <Row style={{ marginTop: 16 }}>
        <Col span={24}>
          <Card title="最近活动" size="small">
            <List
              size="small"
              dataSource={activityLogs}
              renderItem={(item) => (
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
                    description={item.timestamp}
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