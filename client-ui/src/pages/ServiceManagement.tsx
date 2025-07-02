import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Row, 
  Col, 
  Button, 
  Table, 
  Tag, 
  Typography, 
  Space, 
  Tabs,
  Input,
  message,
  Modal
} from 'antd';
import {
  PlayCircleOutlined,
  PauseCircleOutlined,
  ReloadOutlined,
  DeleteOutlined,
  EyeOutlined,
  CloudDownloadOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import { tauriAPI } from '../utils/tauri';

const { Title, Text } = Typography;
const { TextArea } = Input;
const { TabPane } = Tabs;

interface Container {
  id: string;
  name: string;
  image: string;
  status: 'running' | 'stopped' | 'paused';
  created: string;
  ports: string[];
}

interface DockerImage {
  id: string;
  repository: string;
  tag: string;
  size: string;
  created: string;
}

interface ServiceInfo {
  name: string;
  status: 'running' | 'stopped';
  containers: Container[];
}

const ServiceManagement: React.FC = () => {
  const [loading, setLoading] = useState(false);
  const [, setServices] = useState<ServiceInfo[]>([]);
  const [containers, setContainers] = useState<Container[]>([]);
  const [images, setImages] = useState<DockerImage[]>([]);
  const [logs, setLogs] = useState('');
  const [selectedContainer, setSelectedContainer] = useState<string>('');
  const [logsModalVisible, setLogsModalVisible] = useState(false);
  const [activeTab, setActiveTab] = useState('services');

  // 模拟数据
  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setLoading(true);
    try {
      // 模拟服务数据
      setServices([
        {
          name: 'duck-compose',
          status: 'running',
          containers: []
        }
      ]);

      // 模拟容器数据
      setContainers([
        {
          id: 'container1',
          name: 'duck-redis',
          image: 'redis:7.0',
          status: 'running',
          created: '2024-01-20 10:00:00',
          ports: ['6379:6379']
        },
        {
          id: 'container2', 
          name: 'duck-mysql',
          image: 'mysql:8.0',
          status: 'running',
          created: '2024-01-20 10:01:00',
          ports: ['3306:3306']
        },
        {
          id: 'container3',
          name: 'duck-nginx',
          image: 'nginx:latest',
          status: 'running',
          created: '2024-01-20 10:02:00',
          ports: ['80:80', '443:443']
        },
        {
          id: 'container4',
          name: 'duck-app',
          image: 'duck/app:latest',
          status: 'running',
          created: '2024-01-20 10:03:00',
          ports: ['8080:8080']
        },
        {
          id: 'container5',
          name: 'duck-worker',
          image: 'duck/worker:latest',
          status: 'stopped',
          created: '2024-01-20 10:04:00',
          ports: []
        }
      ]);

      // 模拟镜像数据
      setImages([
        {
          id: 'img1',
          repository: 'redis',
          tag: '7.0',
          size: '113MB',
          created: '2024-01-20'
        },
        {
          id: 'img2',
          repository: 'mysql',
          tag: '8.0',
          size: '521MB',
          created: '2024-01-20'
        },
        {
          id: 'img3',
          repository: 'nginx',
          tag: 'latest',
          size: '187MB',
          created: '2024-01-20'
        },
        {
          id: 'img4',
          repository: 'duck/app',
          tag: 'latest',
          size: '256MB',
          created: '2024-01-20'
        },
        {
          id: 'img5',
          repository: 'duck/worker',
          tag: 'latest',
          size: '189MB',
          created: '2024-01-20'
        }
      ]);
    } catch (error) {
      console.error('Failed to load data:', error);
      message.error('加载数据失败');
    } finally {
      setLoading(false);
    }
  };

  const handleServiceControl = async (action: 'start' | 'stop' | 'restart') => {
    setLoading(true);
    try {
      await tauriAPI.service[action]();
      const actionText = action === 'start' ? '启动' : action === 'stop' ? '停止' : '重启';
      message.success(`服务${actionText}成功`);
      await loadData();
    } catch (error) {
      console.error(`Failed to ${action} service:`, error);
      const actionText = action === 'start' ? '启动' : action === 'stop' ? '停止' : '重启';
      message.error(`服务${actionText}失败`);
    } finally {
      setLoading(false);
    }
  };

  const handleContainerAction = async (_containerId: string, action: 'start' | 'stop' | 'restart') => {
    setLoading(true);
    try {
      // TODO: 调用容器操作API
      const actionText = action === 'start' ? '启动' : action === 'stop' ? '停止' : '重启';
      message.success(`容器${actionText}成功`);
      await loadData();
    } catch (error) {
      const actionText = action === 'start' ? '启动' : action === 'stop' ? '停止' : '重启';
      message.error(`容器${actionText}失败`);
    } finally {
      setLoading(false);
    }
  };

  const showContainerLogs = async (_containerId: string, containerName: string) => {
    setSelectedContainer(containerName);
    setLogsModalVisible(true);
    setLogs('正在加载日志...');
    
    try {
      // TODO: 调用获取日志API
      setTimeout(() => {
        setLogs(`[${new Date().toLocaleString()}] 容器 ${containerName} 启动成功
[${new Date().toLocaleString()}] 连接到数据库成功
[${new Date().toLocaleString()}] 服务监听端口 8080
[${new Date().toLocaleString()}] 准备就绪，等待请求...`);
      }, 1000);
    } catch (error) {
      setLogs('获取日志失败');
    }
  };

  const getStatusTag = (status: string) => {
    const statusMap = {
      'running': { color: 'green', text: '运行中' },
      'stopped': { color: 'red', text: '已停止' },
      'paused': { color: 'orange', text: '已暂停' }
    };
    const config = statusMap[status as keyof typeof statusMap] || { color: 'default', text: status };
    return <Tag color={config.color}>{config.text}</Tag>;
  };

  const containerColumns = [
    {
      title: '容器名称',
      dataIndex: 'name',
      key: 'name',
      render: (text: string) => <Text strong>{text}</Text>,
    },
    {
      title: '镜像',
      dataIndex: 'image',
      key: 'image',
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      render: (status: string) => getStatusTag(status),
    },
    {
      title: '端口映射',
      dataIndex: 'ports',
      key: 'ports',
      render: (ports: string[]) => (
        <div>
          {ports.map((port, index) => (
            <Tag key={index} color="blue">{port}</Tag>
          ))}
        </div>
      ),
    },
    {
      title: '创建时间',
      dataIndex: 'created',
      key: 'created',
    },
    {
      title: '操作',
      key: 'actions',
      render: (_: any, record: Container) => (
        <Space>
          {record.status === 'stopped' ? (
            <Button 
              size="small" 
              type="primary" 
              icon={<PlayCircleOutlined />}
              onClick={() => handleContainerAction(record.id, 'start')}
            >
              启动
            </Button>
          ) : (
            <Button 
              size="small" 
              danger
              icon={<PauseCircleOutlined />}
              onClick={() => handleContainerAction(record.id, 'stop')}
            >
              停止
            </Button>
          )}
          <Button 
            size="small"
            icon={<ReloadOutlined />}
            onClick={() => handleContainerAction(record.id, 'restart')}
          >
            重启
          </Button>
          <Button 
            size="small"
            icon={<EyeOutlined />}
            onClick={() => showContainerLogs(record.id, record.name)}
          >
            日志
          </Button>
        </Space>
      ),
    },
  ];

  const imageColumns = [
    {
      title: '仓库',
      dataIndex: 'repository',
      key: 'repository',
      render: (text: string) => <Text strong>{text}</Text>,
    },
    {
      title: '标签',
      dataIndex: 'tag',
      key: 'tag',
      render: (tag: string) => <Tag color="blue">{tag}</Tag>,
    },
    {
      title: '大小',
      dataIndex: 'size',
      key: 'size',
    },
    {
      title: '创建时间',
      dataIndex: 'created',
      key: 'created',
    },
    {
      title: '操作',
      key: 'actions',
      render: () => (
        <Space>
          <Button 
            size="small"
            icon={<DeleteOutlined />}
            danger
          >
            删除
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <Title level={2}>服务管理</Title>
      
      {/* 服务控制面板 */}
      <Card title="Docker Compose 服务控制" style={{ marginBottom: 24 }}>
        <Row gutter={[16, 16]} align="middle">
          <Col span={12}>
            <Space direction="vertical">
              <Text>当前服务状态：{getStatusTag('running')}</Text>
              <Text type="secondary">管理整个 Docker Compose 服务的启停</Text>
            </Space>
          </Col>
          <Col span={12}>
            <Space>
              <Button
                type="primary"
                icon={<PlayCircleOutlined />}
                onClick={() => handleServiceControl('start')}
                loading={loading}
              >
                启动所有服务
              </Button>
              <Button
                danger
                icon={<PauseCircleOutlined />}
                onClick={() => handleServiceControl('stop')}
                loading={loading}
              >
                停止所有服务
              </Button>
              <Button
                icon={<ReloadOutlined />}
                onClick={() => handleServiceControl('restart')}
                loading={loading}
              >
                重启所有服务
              </Button>
            </Space>
          </Col>
        </Row>
      </Card>

      {/* 详细管理标签页 */}
      <Card>
        <Tabs activeKey={activeTab} onChange={setActiveTab}>
          <TabPane tab="容器管理" key="containers">
            <div style={{ marginBottom: 16 }}>
              <Row justify="space-between" align="middle">
                <Col>
                  <Space>
                    <InfoCircleOutlined />
                    <Text>共 {containers.length} 个容器，{containers.filter(c => c.status === 'running').length} 个运行中</Text>
                  </Space>
                </Col>
                <Col>
                  <Button 
                    icon={<ReloadOutlined />} 
                    onClick={loadData}
                    loading={loading}
                  >
                    刷新
                  </Button>
                </Col>
              </Row>
            </div>
            
            <Table
              columns={containerColumns}
              dataSource={containers}
              rowKey="id"
              loading={loading}
              pagination={{ pageSize: 10 }}
              scroll={{ x: 'max-content' }}
            />
          </TabPane>

          <TabPane tab="镜像管理" key="images">
            <div style={{ marginBottom: 16 }}>
              <Row justify="space-between" align="middle">
                <Col>
                  <Space>
                    <InfoCircleOutlined />
                    <Text>共 {images.length} 个镜像</Text>
                  </Space>
                </Col>
                <Col>
                  <Space>
                    <Button 
                      icon={<CloudDownloadOutlined />} 
                      type="primary"
                    >
                      加载镜像
                    </Button>
                    <Button 
                      icon={<ReloadOutlined />} 
                      onClick={loadData}
                      loading={loading}
                    >
                      刷新
                    </Button>
                  </Space>
                </Col>
              </Row>
            </div>

            <Table
              columns={imageColumns}
              dataSource={images}
              rowKey="id"
              loading={loading}
              pagination={{ pageSize: 10 }}
            />
          </TabPane>

          <TabPane tab="系统信息" key="info">
            <Row gutter={[16, 16]}>
              <Col span={12}>
                <Card size="small" title="Docker 信息">
                  <Space direction="vertical" style={{ width: '100%' }}>
                    <div><Text strong>版本：</Text>Docker 24.0.6</div>
                    <div><Text strong>API版本：</Text>1.43</div>
                    <div><Text strong>运行时：</Text>runc</div>
                    <div><Text strong>存储驱动：</Text>overlay2</div>
                  </Space>
                </Card>
              </Col>
              <Col span={12}>
                <Card size="small" title="系统资源">
                  <Space direction="vertical" style={{ width: '100%' }}>
                    <div><Text strong>CPU使用率：</Text>15%</div>
                    <div><Text strong>内存使用：</Text>2.1GB / 8GB</div>
                    <div><Text strong>磁盘使用：</Text>45GB / 100GB</div>
                    <div><Text strong>网络：</Text>bridge, host</div>
                  </Space>
                </Card>
              </Col>
            </Row>
          </TabPane>
        </Tabs>
      </Card>

      {/* 容器日志模态框 */}
      <Modal
        title={`容器日志 - ${selectedContainer}`}
        open={logsModalVisible}
        onCancel={() => setLogsModalVisible(false)}
        footer={[
          <Button key="close" onClick={() => setLogsModalVisible(false)}>
            关闭
          </Button>
        ]}
        width={800}
      >
        <TextArea
          value={logs}
          rows={15}
          readOnly
          style={{ fontFamily: 'monospace', fontSize: '12px' }}
        />
      </Modal>
    </div>
  );
};

export default ServiceManagement; 