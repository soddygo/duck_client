import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Row, 
  Col, 
  Form,
  Input,
  Button,
  Switch,
  Select,
  Typography, 
  Space, 
  message,
  Divider,
  Alert,
  Modal,
  Progress,
  Tag,
  Statistic
} from 'antd';
import {
  SettingOutlined,
  CloudOutlined,
  SafetyOutlined,
  BugOutlined,
  FolderOpenOutlined,
  SaveOutlined,
  ReloadOutlined,
  CheckCircleOutlined,
  InboxOutlined,
  FileTextOutlined,
  DatabaseOutlined,
  HddOutlined
} from '@ant-design/icons';
import { tauriAPI } from '../utils/tauri';
import type { DataDirectoryInfo } from '../types/index';

const { Title, Text } = Typography;
const { Option } = Select;

interface APIConfig {
  serverUrl: string;
  clientId: string;
  version: string;
  connected: boolean;
  lastSync: string;
}

interface UpdateConfig {
  autoCheck: boolean;
  checkInterval: 'startup' | 'daily' | 'weekly' | 'never';
  preRelease: boolean;
  autoDownload: boolean;
}

interface BackupConfig {
  defaultPath: string;
  autoCleanup: boolean;
  maxBackups: number;
  compressionLevel: 'none' | 'fast' | 'normal' | 'maximum';
}

interface LogConfig {
  level: 'error' | 'warn' | 'info' | 'debug';
  maxFileSize: number;
  maxFiles: number;
  enableFileLog: boolean;
}

const Settings: React.FC = () => {
  const [loading, setLoading] = useState(false);
  const [testingConnection, setTestingConnection] = useState(false);
  const [dataDirectoryInfo, setDataDirectoryInfo] = useState<DataDirectoryInfo | null>(null);
  const [loadingDataDir, setLoadingDataDir] = useState(false);
  
  const [apiConfig, setApiConfig] = useState<APIConfig>({
    serverUrl: 'https://api.duck-client.dev',
    clientId: 'duck_client_001',
    version: 'v1',
    connected: true,
    lastSync: '2024-01-20 10:30:00'
  });

  const [updateConfig, setUpdateConfig] = useState<UpdateConfig>({
    autoCheck: true,
    checkInterval: 'daily',
    preRelease: false,
    autoDownload: false
  });

  const [backupConfig, setBackupConfig] = useState<BackupConfig>({
    defaultPath: './backups',
    autoCleanup: true,
    maxBackups: 10,
    compressionLevel: 'normal'
  });

  const [logConfig, setLogConfig] = useState<LogConfig>({
    level: 'info',
    maxFileSize: 10,
    maxFiles: 5,
    enableFileLog: true
  });

  const [initModalVisible, setInitModalVisible] = useState(false);
  const [initProgress, setInitProgress] = useState(0);

  const [apiForm] = Form.useForm();
  const [updateForm] = Form.useForm();
  const [backupForm] = Form.useForm();
  const [logForm] = Form.useForm();

  useEffect(() => {
    loadSettings();
    loadDataDirectoryInfo();
  }, []);

  const loadSettings = async () => {
    setLoading(true);
    try {
      // TODO: 从后端加载配置
      await new Promise(resolve => setTimeout(resolve, 500));
    } catch (error) {
      message.error('加载配置失败');
    } finally {
      setLoading(false);
    }
  };

  // 加载数据目录信息
  const loadDataDirectoryInfo = async () => {
    setLoadingDataDir(true);
    try {
      const info = await tauriAPI.dataDirectory.getInfo();
      setDataDirectoryInfo(info);
    } catch (error) {
      console.error('Failed to load data directory info:', error);
      message.error('加载数据目录信息失败');
    } finally {
      setLoadingDataDir(false);
    }
  };

  // 打开工作目录
  const openDataDirectory = async () => {
    try {
      await tauriAPI.dataDirectory.openData();
      message.success('已打开工作目录');
    } catch (error) {
      console.error('Failed to open data directory:', error);
      message.error('打开工作目录失败');
    }
  };

  // 打开备份目录
  const openBackupDirectory = async () => {
    try {
      await tauriAPI.dataDirectory.openBackup();
      message.success('已打开备份目录');
    } catch (error) {
      console.error('Failed to open backup directory:', error);
      message.error('打开备份目录失败');
    }
  };

  // 打开缓存目录
  const openCacheDirectory = async () => {
    try {
      await tauriAPI.dataDirectory.openCache();
      message.success('已打开缓存目录');
    } catch (error) {
      console.error('Failed to open cache directory:', error);
      message.error(typeof error === 'string' ? error : '打开缓存目录失败');
    }
  };



  const testConnection = async () => {
    setTestingConnection(true);
    try {
      // TODO: 测试API连接
      await new Promise(resolve => setTimeout(resolve, 2000));
      
      const success = Math.random() > 0.3;
      if (success) {
        setApiConfig(prev => ({ ...prev, connected: true, lastSync: new Date().toLocaleString('zh-CN') }));
        message.success('连接测试成功');
      } else {
        setApiConfig(prev => ({ ...prev, connected: false }));
        message.error('连接测试失败，请检查服务器地址和网络连接');
      }
    } catch (error) {
      message.error('连接测试失败');
    } finally {
      setTestingConnection(false);
    }
  };

  const saveApiConfig = async (values: APIConfig) => {
    try {
      // TODO: 保存API配置
      setApiConfig(values);
      message.success('API配置已保存');
    } catch (error) {
      message.error('保存API配置失败');
    }
  };

  const saveUpdateConfig = async (values: UpdateConfig) => {
    try {
      // TODO: 保存更新配置
      setUpdateConfig(values);
      message.success('更新配置已保存');
    } catch (error) {
      message.error('保存更新配置失败');
    }
  };

  const saveBackupConfig = async (values: BackupConfig) => {
    try {
      // TODO: 保存备份配置
      setBackupConfig(values);
      message.success('备份配置已保存');
    } catch (error) {
      message.error('保存备份配置失败');
    }
  };

  const saveLogConfig = async (values: LogConfig) => {
    try {
      // TODO: 保存日志配置
      setLogConfig(values);
      message.success('日志配置已保存');
    } catch (error) {
      message.error('保存日志配置失败');
    }
  };

  const initializeClient = async () => {
    setInitModalVisible(true);
    setInitProgress(0);

    try {
      // 模拟初始化过程
      const steps = [
        '创建配置文件...',
        '初始化数据库...',
        '设置默认配置...',
        '检查服务连接...',
        '完成初始化...'
      ];

      for (let i = 0; i < steps.length; i++) {
        await new Promise(resolve => setTimeout(resolve, 1000));
        setInitProgress((i + 1) * 20);
      }

      message.success('客户端初始化完成');
    } catch (error) {
      message.error('初始化失败');
    } finally {
      setTimeout(() => setInitModalVisible(false), 1000);
    }
  };



  const clearCache = async () => {
    try {
      // TODO: 清理缓存
      message.success('缓存清理完成');
    } catch (error) {
      message.error('清理缓存失败');
    }
  };

  const exportConfig = () => {
    // TODO: 导出配置文件
    message.info('配置导出功能开发中...');
  };

  const importConfig = () => {
    // TODO: 导入配置文件
    message.info('配置导入功能开发中...');
  };

  return (
    <div>
      <Title level={2}>系统设置</Title>

      <Row gutter={[16, 24]}>
        {/* 数据目录配置 */}
        <Col span={24}>
          <Card 
            title={
              <Space>
                <DatabaseOutlined />
                <span>数据目录</span>
                <Tag color={dataDirectoryInfo?.exists ? 'green' : 'orange'}>
                  {dataDirectoryInfo?.exists ? '已初始化' : '未初始化'}
                </Tag>
              </Space>
            }
            loading={loadingDataDir}
          >
            <Row gutter={[16, 16]}>
              <Col span={24}>
                <Space direction="vertical" style={{ width: '100%' }}>
                  <div>
                    <Text strong>工作目录：</Text>
                    <br />
                    <Text code copyable style={{ fontSize: 12 }}>
                      {dataDirectoryInfo?.path || '加载中...'}
                    </Text>
                  </div>
                  
                  <Text type="secondary">
                    基于 duck-cli init 执行目录，包含所有 Docker 服务、备份、缓存数据
                  </Text>
                </Space>
              </Col>
            </Row>
            
            {/* 子目录详情 */}
            {dataDirectoryInfo && (
              <div style={{ marginTop: 16 }}>
                <Row gutter={[16, 16]}>
                  {/* 备份目录 */}
                  <Col span={8}>
                    <Card 
                      size="small" 
                      title={
                        <Space>
                          <SafetyOutlined />
                          <span>备份目录</span>
                          <Tag color={dataDirectoryInfo.backup_exists ? 'green' : 'default'}>
                            {dataDirectoryInfo.backup_exists ? '存在' : '不存在'}
                          </Tag>
                        </Space>
                      }
                    >
                      <Space direction="vertical" style={{ width: '100%' }}>
                        <Text code style={{ fontSize: 11 }}>./backups</Text>
                        <Button 
                          size="small"
                          icon={<FolderOpenOutlined />}
                          onClick={openBackupDirectory}
                          block
                        >
                          打开备份目录
                        </Button>
                      </Space>
                    </Card>
                  </Col>
                  
                  {/* 缓存目录 */}
                  <Col span={8}>
                    <Card 
                      size="small" 
                      title={
                        <Space>
                          <DatabaseOutlined />
                          <span>缓存目录</span>
                          <Tag color={dataDirectoryInfo.cache_exists ? 'green' : 'default'}>
                            {dataDirectoryInfo.cache_exists ? '存在' : '不存在'}
                          </Tag>
                        </Space>
                      }
                    >
                      <Space direction="vertical" style={{ width: '100%' }}>
                        <Text code style={{ fontSize: 11 }}>./cacheDuckData</Text>
                        <Button 
                          size="small"
                          icon={<FolderOpenOutlined />}
                          onClick={openCacheDirectory}
                          disabled={!dataDirectoryInfo.cache_exists}
                          block
                        >
                          打开缓存目录
                        </Button>
                      </Space>
                    </Card>
                  </Col>
                  
                  {/* Docker目录 */}
                  <Col span={8}>
                    <Card 
                      size="small" 
                      title={
                        <Space>
                          <HddOutlined />
                          <span>Docker目录</span>
                          <Tag color={dataDirectoryInfo.docker_exists ? 'green' : 'default'}>
                            {dataDirectoryInfo.docker_exists ? '存在' : '不存在'}
                          </Tag>
                        </Space>
                      }
                    >
                      <Space direction="vertical" style={{ width: '100%' }}>
                        <Text code style={{ fontSize: 11 }}>./docker</Text>
                        <Button 
                          size="small"
                          icon={<FolderOpenOutlined />}
                          onClick={openDataDirectory}
                          block
                        >
                          打开工作目录
                        </Button>
                      </Space>
                    </Card>
                  </Col>
                </Row>
                
                {/* 统计信息 */}
                <div style={{ marginTop: 16 }}>
                  <Row gutter={[16, 16]}>
                    <Col span={8}>
                      <Statistic
                        title="总占用空间"
                        value={dataDirectoryInfo.total_size_mb}
                        precision={2}
                        suffix="MB"
                        prefix={<HddOutlined />}
                      />
                    </Col>
                    <Col span={8}>
                      <Space style={{ width: '100%', justifyContent: 'center' }}>
                        <Button 
                          icon={<ReloadOutlined />}
                          onClick={loadDataDirectoryInfo}
                          loading={loadingDataDir}
                        >
                          刷新信息
                        </Button>
                        <Button 
                          type="primary"
                          icon={<FolderOpenOutlined />}
                          onClick={openDataDirectory}
                        >
                          打开工作目录
                        </Button>
                      </Space>
                    </Col>
                  </Row>
                </div>
              </div>
            )}
          </Card>
        </Col>

        {/* API配置 */}
        <Col span={24}>
          <Card 
            title={
              <Space>
                <CloudOutlined />
                <span>API配置</span>
                <Tag color={apiConfig.connected ? 'green' : 'red'}>
                  {apiConfig.connected ? '已连接' : '未连接'}
                </Tag>
              </Space>
            }
            loading={loading}
          >
            <Form
              form={apiForm}
              layout="vertical"
              initialValues={apiConfig}
              onFinish={saveApiConfig}
            >
              <Row gutter={16}>
                <Col span={12}>
                  <Form.Item
                    name="serverUrl"
                    label="服务器地址"
                    rules={[{ required: true, message: '请输入服务器地址' }]}
                  >
                    <Input placeholder="https://api.duck-client.dev" />
                  </Form.Item>
                </Col>
                <Col span={6}>
                  <Form.Item
                    name="version"
                    label="API版本"
                  >
                    <Select>
                      <Option value="v1">v1</Option>
                      <Option value="v2">v2 (Beta)</Option>
                    </Select>
                  </Form.Item>
                </Col>
                <Col span={6}>
                  <Form.Item label=" ">
                    <Space>
                      <Button 
                        onClick={testConnection}
                        loading={testingConnection}
                        icon={<CheckCircleOutlined />}
                      >
                        测试连接
                      </Button>
                    </Space>
                  </Form.Item>
                </Col>
              </Row>

              <Row gutter={16}>
                <Col span={12}>
                  <Form.Item
                    name="clientId"
                    label="客户端ID"
                    help="用于标识此客户端的唯一ID"
                  >
                    <Input disabled />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <div>
                    <Text strong>最后同步时间：</Text>
                    <Text type="secondary">{apiConfig.lastSync}</Text>
                  </div>
                </Col>
              </Row>

              <Form.Item>
                <Space>
                  <Button type="primary" htmlType="submit" icon={<SaveOutlined />}>
                    保存配置
                  </Button>
                  <Button 
                    icon={<ReloadOutlined />}
                    onClick={() => apiForm.resetFields()}
                  >
                    重置
                  </Button>
                </Space>
              </Form.Item>
            </Form>
          </Card>
        </Col>

        {/* 更新设置 */}
        <Col span={12}>
          <Card title={
            <Space>
              <SettingOutlined />
              <span>更新设置</span>
            </Space>
          }>
            <Form
              form={updateForm}
              layout="vertical"
              initialValues={updateConfig}
              onFinish={saveUpdateConfig}
            >
              <Form.Item name="autoCheck" valuePropName="checked">
                <Switch />
                <Text style={{ marginLeft: 8 }}>自动检查更新</Text>
              </Form.Item>

              <Form.Item
                name="checkInterval"
                label="检查频率"
              >
                <Select>
                  <Option value="startup">启动时检查</Option>
                  <Option value="daily">每天检查</Option>
                  <Option value="weekly">每周检查</Option>
                  <Option value="never">从不检查</Option>
                </Select>
              </Form.Item>

              <Form.Item name="preRelease" valuePropName="checked">
                <Switch />
                <Text style={{ marginLeft: 8 }}>接收预发布版本</Text>
              </Form.Item>

              <Form.Item name="autoDownload" valuePropName="checked">
                <Switch />
                <Text style={{ marginLeft: 8 }}>自动下载更新</Text>
              </Form.Item>

              <Form.Item>
                <Button type="primary" htmlType="submit" icon={<SaveOutlined />}>
                  保存设置
                </Button>
              </Form.Item>
            </Form>
          </Card>
        </Col>

        {/* 备份设置 */}
        <Col span={12}>
          <Card title={
            <Space>
              <SafetyOutlined />
              <span>备份设置</span>
            </Space>
          }>
            <Form
              form={backupForm}
              layout="vertical"
              initialValues={backupConfig}
              onFinish={saveBackupConfig}
            >
              <Form.Item
                name="defaultPath"
                label="默认备份路径"
                help="备份文件存储在工作目录下的 backups 文件夹中"
              >
                <Input 
                  disabled
                  value="./backups"
                  addonAfter={
                    <Button 
                      size="small" 
                      icon={<FolderOpenOutlined />}
                      onClick={openBackupDirectory}
                    >
                      打开
                    </Button>
                  }
                />
              </Form.Item>

              <Form.Item
                name="compressionLevel"
                label="压缩级别"
              >
                <Select>
                  <Option value="none">无压缩</Option>
                  <Option value="fast">快速压缩</Option>
                  <Option value="normal">标准压缩</Option>
                  <Option value="maximum">最大压缩</Option>
                </Select>
              </Form.Item>

              <Form.Item name="autoCleanup" valuePropName="checked">
                <Switch />
                <Text style={{ marginLeft: 8 }}>自动清理旧备份</Text>
              </Form.Item>

              <Form.Item
                name="maxBackups"
                label="最大备份数量"
              >
                <Select>
                  <Option value={5}>5个</Option>
                  <Option value={10}>10个</Option>
                  <Option value={20}>20个</Option>
                  <Option value={50}>50个</Option>
                </Select>
              </Form.Item>

              <Form.Item>
                <Button type="primary" htmlType="submit" icon={<SaveOutlined />}>
                  保存设置
                </Button>
              </Form.Item>
            </Form>
          </Card>
        </Col>

        {/* 日志设置 */}
        <Col span={12}>
          <Card title={
            <Space>
              <BugOutlined />
              <span>日志设置</span>
            </Space>
          }>
            <Form
              form={logForm}
              layout="vertical"
              initialValues={logConfig}
              onFinish={saveLogConfig}
            >
              <Form.Item
                name="level"
                label="日志级别"
              >
                <Select>
                  <Option value="error">错误</Option>
                  <Option value="warn">警告</Option>
                  <Option value="info">信息</Option>
                  <Option value="debug">调试</Option>
                </Select>
              </Form.Item>

              <Form.Item name="enableFileLog" valuePropName="checked">
                <Switch />
                <Text style={{ marginLeft: 8 }}>启用文件日志</Text>
              </Form.Item>

              <Form.Item
                name="maxFileSize"
                label="单个文件最大大小 (MB)"
              >
                <Select>
                  <Option value={5}>5 MB</Option>
                  <Option value={10}>10 MB</Option>
                  <Option value={20}>20 MB</Option>
                  <Option value={50}>50 MB</Option>
                </Select>
              </Form.Item>

              <Form.Item
                name="maxFiles"
                label="最大文件数量"
              >
                <Select>
                  <Option value={3}>3个</Option>
                  <Option value={5}>5个</Option>
                  <Option value={10}>10个</Option>
                  <Option value={20}>20个</Option>
                </Select>
              </Form.Item>

              <Form.Item>
                <Button type="primary" htmlType="submit" icon={<SaveOutlined />}>
                  保存设置
                </Button>
              </Form.Item>
            </Form>
          </Card>
        </Col>

        {/* 系统操作 */}
        <Col span={12}>
          <Card title={
            <Space>
              <SettingOutlined />
              <span>系统操作</span>
            </Space>
          }>
            <Space direction="vertical" style={{ width: '100%' }}>
              <Alert
                message="客户端初始化"
                description="首次使用时初始化客户端，创建配置文件和数据库"
                type="info"
                showIcon
              />
              
              <Button 
                type="primary"
                icon={<SettingOutlined />}
                onClick={initializeClient}
                block
              >
                初始化客户端
              </Button>

              <Divider />

              <Space style={{ width: '100%', justifyContent: 'space-between' }}>
                <Button onClick={clearCache}>
                  清理缓存
                </Button>
                <Button onClick={exportConfig} icon={<FileTextOutlined />}>
                  导出配置
                </Button>
                <Button onClick={importConfig} icon={<InboxOutlined />}>
                  导入配置
                </Button>
              </Space>
            </Space>
          </Card>
        </Col>
      </Row>

      {/* 初始化进度模态框 */}
      <Modal
        title="初始化客户端"
        open={initModalVisible}
        onCancel={() => setInitModalVisible(false)}
        footer={null}
        closable={initProgress >= 100}
        maskClosable={false}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Progress percent={initProgress} status="active" />
          <Text>正在初始化客户端，请稍候...</Text>
          {initProgress >= 100 && (
            <Alert
              message="初始化完成"
              description="客户端已成功初始化，可以正常使用。"
              type="success"
              showIcon
            />
          )}
        </Space>
      </Modal>
    </div>
  );
};

export default Settings; 