import { useState, useEffect } from 'react';
import { 
  Card, 
  Form, 
  Input, 
  Button, 
  Select, 
  Switch, 
  message, 
  Space, 
  Typography, 
  Alert,
  Tag,
  Row,
  Col,
  Spin,
  Modal,
  InputNumber
} from 'antd';
import { 
  FolderOutlined, 
  DatabaseOutlined, 
  SettingOutlined, 
  ExclamationCircleOutlined,
  CheckCircleOutlined,
  FolderOpenOutlined,
  ReloadOutlined,
  SafetyOutlined,
  BugOutlined
} from '@ant-design/icons';
import { 
  getDataDirectory,
  selectWorkDirectory,
  openDataDirectory,
  openBackupDirectory,
  openCacheDirectory,
  clearCache,
  initClient,
  autoDeployService,
  initAndDeploy
} from '../utils/tauri';
import type { DataDirectoryInfo } from '../types';

const { Title, Text } = Typography;
const { Option } = Select;

function Settings() {
  const [loading, setLoading] = useState(false);
  const [dataDirectory, setDataDirectory] = useState<DataDirectoryInfo | null>(null);
  const [directoryLoading, setDirectoryLoading] = useState(false);
  const [deployLoading, setDeployLoading] = useState(false);
  const [port, setPort] = useState<number>(3000);

  // 加载数据目录信息
  const loadDataDirectory = async () => {
    setDirectoryLoading(true);
    try {
      const data = await getDataDirectory() as DataDirectoryInfo;
      setDataDirectory(data);
    } catch (error: any) {
      console.warn('获取数据目录信息失败:', error);
      // 如果是因为未设置工作目录，不显示错误信息
      if (!error.includes('未设置工作目录')) {
        message.error('获取数据目录信息失败: ' + error);
      }
    } finally {
      setDirectoryLoading(false);
    }
  };

  useEffect(() => {
    loadDataDirectory();
  }, []);

  // 选择工作目录
  const handleSelectWorkDirectory = async () => {
    try {
      const result = await selectWorkDirectory();
      if (result) {
        message.success('工作目录设置成功');
        await loadDataDirectory();
      }
    } catch (error: any) {
      message.error('选择工作目录失败: ' + error);
    }
  };

  // 打开目录函数
  const handleOpenDataDirectory = async () => {
    try {
      await openDataDirectory();
    } catch (error: any) {
      message.error('打开工作目录失败: ' + error);
    }
  };

  const handleOpenBackupDirectory = async () => {
    try {
      await openBackupDirectory();
    } catch (error: any) {
      message.error('打开备份目录失败: ' + error);
    }
  };

  const handleOpenCacheDirectory = async () => {
    try {
      await openCacheDirectory();
    } catch (error: any) {
      message.error('打开缓存目录失败: ' + error);
    }
  };

  // 清理缓存
  const handleClearCache = async () => {
    Modal.confirm({
      title: '确认清理缓存',
      content: '这将删除所有缓存文件，包括下载的Docker文件。您确定要继续吗？',
      onOk: async () => {
        try {
          setLoading(true);
          await clearCache();
          message.success('缓存清理完成');
          loadDataDirectory(); // 重新加载目录信息
        } catch (error) {
          message.error(`清理缓存失败: ${error}`);
        } finally {
          setLoading(false);
        }
      },
    });
  };

  // 自动部署服务
  const handleAutoDeploy = async () => {
    try {
      setDeployLoading(true);
      const result = await autoDeployService(port);
      message.success(result as string);
      loadDataDirectory(); // 重新加载目录信息
    } catch (error) {
      message.error(`自动部署失败: ${error}`);
    } finally {
      setDeployLoading(false);
    }
  };

  // 一键初始化并部署
  const handleInitAndDeploy = async () => {
    Modal.confirm({
      title: '一键初始化并部署',
      content: (
        <div>
          <p>这将执行以下操作：</p>
          <ul>
            <li>初始化客户端配置</li>
            <li>从服务器拉取最新的Docker服务</li>
            <li>自动部署服务到指定端口</li>
          </ul>
          <p style={{ marginTop: 16 }}>
            <strong>端口号: </strong>
            <InputNumber 
              value={port} 
              onChange={(value) => setPort(value || 3000)} 
              min={1000} 
              max={65535} 
              style={{ width: 120 }}
            />
          </p>
        </div>
      ),
      onOk: async () => {
        try {
          setDeployLoading(true);
          const result = await initAndDeploy(port);
          message.success(result as string);
          loadDataDirectory(); // 重新加载目录信息
        } catch (error) {
          message.error(`初始化和部署失败: ${error}`);
        } finally {
          setDeployLoading(false);
        }
      },
    });
  };

  // 初始化客户端
  const handleInitClient = async () => {
    if (!dataDirectory?.work_dir) {
      message.warning('请先选择工作目录');
      return;
    }

    Modal.confirm({
      title: '确认初始化客户端',
      content: '这将在当前工作目录创建配置文件和数据库。如果文件已存在将被覆盖。确定要继续吗？',
      onOk: async () => {
        try {
          setLoading(true);
          const result = await initClient() as string;
          message.success(result);
          loadDataDirectory(); // 重新加载数据目录信息
        } catch (error: any) {
          message.error('初始化客户端失败: ' + error);
        } finally {
          setLoading(false);
        }
      },
    });
  };

  // API相关状态和函数
  const [apiConfig, setApiConfig] = useState({
    serverUrl: 'https://api.duck-client.dev',
    version: 'v1',
    clientId: 'duck_client_001',
    lastSync: '2024-01-20 10:30:00'
  });

  const testConnection = async () => {
    try {
      setLoading(true);
      // 模拟测试连接
      await new Promise(resolve => setTimeout(resolve, 1000));
      message.success('连接测试成功');
    } catch (error) {
      message.error('连接测试失败');
    } finally {
      setLoading(false);
    }
  };

  // 渲染数据目录卡片
  const renderDataDirectoryCard = () => {
    if (directoryLoading) {
      return (
        <Card title="数据目录" loading={true}>
          <Spin size="large" />
        </Card>
      );
    }

    if (!dataDirectory) {
      return (
        <Card 
          title={
            <Space>
              <FolderOutlined />
              数据目录
              <Tag color="red">未配置</Tag>
            </Space>
          }
        >
          <Alert
            message="未设置工作目录"
            description="请选择一个目录作为 Duck CLI 的工作目录。该目录用于存储配置文件、备份和缓存数据。"
            type="warning"
            showIcon
            style={{ marginBottom: 16 }}
          />
          <Button type="primary" onClick={handleSelectWorkDirectory}>
            <FolderOpenOutlined />
            选择工作目录
          </Button>
        </Card>
      );
    }

    return (
      <Card 
        title={
          <Space>
            <FolderOutlined />
            数据目录
            <Tag color={dataDirectory.is_initialized ? "green" : "orange"}>
              {dataDirectory.is_initialized ? "已初始化" : "未初始化"}
            </Tag>
          </Space>
        }
        extra={
          <Button icon={<ReloadOutlined />} onClick={loadDataDirectory}>
            刷新信息
          </Button>
        }
      >
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          {/* 工作目录信息 */}
          <Space direction="vertical" size="middle">
            <Text strong>工作目录: {dataDirectory.work_dir}</Text>
            <Text type="secondary">
              基于 duck-cli init 执行目录，包含所有 Docker 服务、备份、缓存数据
            </Text>
            
            {/* 端口设置 */}
            {dataDirectory.is_initialized && (
              <div>
                <Text strong>部署端口设置: </Text>
                <InputNumber 
                  value={port} 
                  onChange={(value) => setPort(value || 3000)} 
                  min={1000} 
                  max={65535} 
                  style={{ width: 120, marginLeft: 8 }}
                  placeholder="端口号"
                />
                <Text type="secondary" style={{ marginLeft: 8 }}>
                  (自动部署时使用此端口)
                </Text>
              </div>
            )}
          </Space>

          {/* 目录状态 */}
          <Row gutter={[16, 16]}>
            <Col span={8}>
              <Space>
                {dataDirectory.backup_exists ? (
                  <CheckCircleOutlined style={{ color: '#52c41a' }} />
                ) : (
                  <ExclamationCircleOutlined style={{ color: '#faad14' }} />
                )}
                <Text strong>备份目录</Text>
              </Space>
              <div style={{ marginTop: 4 }}>
                <Text type="secondary">./backups</Text>
                <br />
                <Button type="link" size="small" onClick={handleOpenBackupDirectory}>
                  打开备份目录
                </Button>
              </div>
            </Col>
            
            <Col span={8}>
              <Space>
                {dataDirectory.cache_exists ? (
                  <CheckCircleOutlined style={{ color: '#52c41a' }} />
                ) : (
                  <ExclamationCircleOutlined style={{ color: '#faad14' }} />
                )}
                <Text strong>缓存目录</Text>
              </Space>
              <div style={{ marginTop: 4 }}>
                <Text type="secondary">./cacheDuckData</Text>
                <br />
                <Button type="link" size="small" onClick={handleOpenCacheDirectory}>
                  打开缓存目录
                </Button>
              </div>
            </Col>
            
            <Col span={8}>
              <Space>
                {dataDirectory.docker_exists ? (
                  <CheckCircleOutlined style={{ color: '#52c41a' }} />
                ) : (
                  <ExclamationCircleOutlined style={{ color: '#faad14' }} />
                )}
                <Text strong>Docker目录</Text>
              </Space>
              <div style={{ marginTop: 4 }}>
                <Text type="secondary">./docker</Text>
                <br />
                <Button type="link" size="small" onClick={handleOpenDataDirectory}>
                  打开工作目录
                </Button>
              </div>
            </Col>
          </Row>

          {/* 总占用空间 */}
          <div style={{ padding: '12px', backgroundColor: '#f5f5f5', borderRadius: '6px' }}>
            <Text strong>总占用空间: </Text>
            <Text code>{dataDirectory.total_size_mb.toFixed(2)} MB</Text>
          </div>

          {/* 操作按钮 */}
          <div style={{ marginTop: 16, display: 'flex', gap: 8, flexWrap: 'wrap' }}>
            <Button 
              onClick={handleSelectWorkDirectory} 
              loading={loading}
              disabled={!dataDirectory}
            >
              更换工作目录
            </Button>
            
            <Button 
              danger 
              onClick={handleClearCache} 
              loading={loading}
              disabled={!dataDirectory?.cache_exists}
            >
              清理缓存
            </Button>
            
            {!dataDirectory.is_initialized && (
              <Button type="primary" onClick={handleInitAndDeploy} loading={deployLoading}>
                一键初始化并部署
              </Button>
            )}
            
            {!dataDirectory.is_initialized && (
              <Button onClick={handleInitClient} loading={loading}>
                仅初始化客户端
              </Button>
            )}
            
            {dataDirectory.is_initialized && (
              <Button 
                type="primary" 
                onClick={handleAutoDeploy} 
                loading={deployLoading}
                style={{ backgroundColor: '#52c41a', borderColor: '#52c41a' }}
              >
                自动部署服务
              </Button>
            )}
          </div>
        </Space>
      </Card>
    );
  };

  return (
    <div style={{ padding: '24px', maxWidth: '1200px' }}>
      <Title level={2}>系统设置</Title>
      
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        {/* 数据目录管理 */}
        {renderDataDirectoryCard()}

        {/* API配置 */}
        <Card 
          title={
            <Space>
              <DatabaseOutlined />
              API配置
              <Tag color="green">已连接</Tag>
            </Space>
          }
        >
          <Form layout="vertical">
            <Row gutter={16}>
              <Col span={12}>
                <Form.Item label="服务器地址">
                  <Input 
                    value={apiConfig.serverUrl}
                    onChange={(e) => setApiConfig({...apiConfig, serverUrl: e.target.value})}
                    placeholder="https://api.duck-client.dev"
                  />
                </Form.Item>
              </Col>
              <Col span={12}>
                <Form.Item label="API版本">
                  <Select 
                    value={apiConfig.version}
                    onChange={(value) => setApiConfig({...apiConfig, version: value})}
                  >
                    <Option value="v1">v1</Option>
                  </Select>
                </Form.Item>
              </Col>
            </Row>
            
            <Form.Item label="客户端ID">
              <Input 
                value={apiConfig.clientId}
                readOnly
                addonAfter={
                  <Button size="small" type="link">
                    复制
                  </Button>
                }
              />
            </Form.Item>
            
            <Form.Item label="最后同步时间">
              <Text type="secondary">{apiConfig.lastSync}</Text>
            </Form.Item>
            
            <Space>
              <Button type="primary" loading={loading}>
                保存配置
              </Button>
              <Button onClick={testConnection} loading={loading}>
                测试连接
              </Button>
            </Space>
          </Form>
        </Card>

        {/* 更新设置 */}
        <Card 
          title={
            <Space>
              <SettingOutlined />
              更新设置
            </Space>
          }
        >
          <Form layout="vertical">
            <Form.Item label="检查更新频率">
              <Select defaultValue="daily">
                <Option value="manual">手动检查</Option>
                <Option value="daily">每天</Option>
                <Option value="weekly">每周</Option>
                <Option value="monthly">每月</Option>
              </Select>
            </Form.Item>
            
            <Form.Item label="自动下载更新">
              <Switch defaultChecked />
              <Text type="secondary" style={{ marginLeft: 8 }}>
                检测到新版本时自动下载
              </Text>
            </Form.Item>
            
            <Form.Item label="更新通知">
              <Switch defaultChecked />
              <Text type="secondary" style={{ marginLeft: 8 }}>
                有新版本时显示桌面通知
              </Text>
            </Form.Item>
            
            <Button type="primary">
              保存更新设置
            </Button>
          </Form>
        </Card>

        {/* 备份设置 */}
        <Card 
          title={
            <Space>
              <SafetyOutlined />
              备份设置
            </Space>
          }
        >
          <Form layout="vertical">
            <Form.Item label="自动备份">
              <Switch defaultChecked />
              <Text type="secondary" style={{ marginLeft: 8 }}>
                升级前自动创建备份
              </Text>
            </Form.Item>
            
            <Form.Item label="备份保留策略">
              <Select defaultValue="keep-10">
                <Option value="keep-5">保留最近5个备份</Option>
                <Option value="keep-10">保留最近10个备份</Option>
                <Option value="keep-20">保留最近20个备份</Option>
                <Option value="keep-all">保留所有备份</Option>
              </Select>
            </Form.Item>
            
            <Form.Item label="备份压缩">
              <Switch defaultChecked />
              <Text type="secondary" style={{ marginLeft: 8 }}>
                压缩备份文件以节省空间
              </Text>
            </Form.Item>
            
            <Button type="primary">
              保存备份设置
            </Button>
          </Form>
        </Card>

        {/* 日志配置 */}
        <Card 
          title={
            <Space>
              <BugOutlined />
              日志配置
            </Space>
          }
        >
          <Form layout="vertical">
            <Form.Item label="日志级别">
              <Select defaultValue="info">
                <Option value="error">Error - 仅错误信息</Option>
                <Option value="warn">Warning - 警告及以上</Option>
                <Option value="info">Info - 信息及以上</Option>
                <Option value="debug">Debug - 调试及以上</Option>
                <Option value="trace">Trace - 所有信息</Option>
              </Select>
            </Form.Item>
            
            <Form.Item label="日志文件大小限制">
              <Select defaultValue="10mb">
                <Option value="5mb">5MB</Option>
                <Option value="10mb">10MB</Option>
                <Option value="20mb">20MB</Option>
                <Option value="50mb">50MB</Option>
              </Select>
            </Form.Item>
            
            <Form.Item label="日志文件保留天数">
              <Select defaultValue="30">
                <Option value="7">7天</Option>
                <Option value="30">30天</Option>
                <Option value="90">90天</Option>
                <Option value="365">1年</Option>
              </Select>
            </Form.Item>
            
            <Button type="primary">
              保存日志配置
            </Button>
          </Form>
        </Card>

        {/* 系统操作 */}
        <Card title="系统操作" type="inner">
          <Alert 
            message="危险操作" 
            description="以下操作可能影响系统正常运行，请谨慎使用。"
            type="warning" 
            showIcon 
            style={{ marginBottom: 16 }}
          />
          
          <Space wrap>
            <Button danger>
              重置所有设置
            </Button>
            <Button danger>
              清除所有数据
            </Button>
            <Button>
              导出配置
            </Button>
            <Button>
              导入配置
            </Button>
          </Space>
        </Card>
      </Space>
    </div>
  );
}

export default Settings; 