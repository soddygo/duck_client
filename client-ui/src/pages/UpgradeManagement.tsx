import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Row, 
  Col, 
  Button, 
  Typography, 
  Space, 
  Alert,
  Progress,
  Steps,
  Modal,
  Form,
  Select,
  Switch,
  Input,
  message,
  Timeline,
  Tag,
  InputNumber
} from 'antd';
import {
  UploadOutlined,
  CheckCircleOutlined,
  ClockCircleOutlined,
  WarningOutlined,
  ReloadOutlined,
  SettingOutlined,
  HistoryOutlined,
  RocketOutlined,
} from '@ant-design/icons';
import { tauriAPI, autoDeployService } from '../utils/tauri';

const { Title, Text } = Typography;
const { Step } = Steps;
const { Option } = Select;

interface UpdateInfo {
  hasUpdate: boolean;
  currentVersion: string;
  latestVersion: string;
  releaseNotes: string;
  downloadSize: number;
  releaseDate: string;
}

interface UpgradeHistory {
  id: string;
  version: string;
  date: string;
  status: 'success' | 'failed' | 'in-progress';
  type: 'manual' | 'auto';
  duration: string;
}

interface AutoUpgradeConfig {
  enabled: boolean;
  schedule: string;
  delayTime: number;
  delayUnit: 'hours' | 'minutes' | 'days';
}

const UpgradeManagement: React.FC = () => {
  const [loading, setLoading] = useState(false);
  const [deployLoading, setDeployLoading] = useState(false);
  const [port, setPort] = useState<number>(3000);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo>({
    hasUpdate: false,
    currentVersion: '1.2.0',
    latestVersion: '1.2.0',
    releaseNotes: '',
    downloadSize: 0,
    releaseDate: ''
  });
  
  const [upgradeProgress, setUpgradeProgress] = useState(0);
  const [upgradeStatus, setUpgradeStatus] = useState<'idle' | 'downloading' | 'installing' | 'complete' | 'error'>('idle');
  const [currentStep, setCurrentStep] = useState(0);
  const [upgradeModalVisible, setUpgradeModalVisible] = useState(false);
  const [scheduleModalVisible, setScheduleModalVisible] = useState(false);
  
  const [autoConfig, setAutoConfig] = useState<AutoUpgradeConfig>({
    enabled: false,
    schedule: '',
    delayTime: 2,
    delayUnit: 'hours'
  });
  
  const [upgradeHistory, setUpgradeHistory] = useState<UpgradeHistory[]>([
    {
      id: '1',
      version: '1.2.0',
      date: '2024-01-20 10:30:00',
      status: 'success',
      type: 'manual',
      duration: '5分钟'
    },
    {
      id: '2',
      version: '1.1.5',
      date: '2024-01-15 02:00:00',
      status: 'success',
      type: 'auto',
      duration: '4分钟'
    },
    {
      id: '3',
      version: '1.1.0',
      date: '2024-01-10 11:20:00',
      status: 'failed',
      type: 'manual',
      duration: '2分钟'
    }
  ]);

  const [form] = Form.useForm();

  useEffect(() => {
    checkForUpdates();
    loadAutoConfig();
  }, []);

  const checkForUpdates = async () => {
    setLoading(true);
    try {
      const updateData = await tauriAPI.update.checkUpdates();
      
      // 模拟有更新的情况
      const hasUpdate = Math.random() > 0.6;
      setUpdateInfo({
        hasUpdate,
        currentVersion: updateData.service_version,
        latestVersion: hasUpdate ? '1.3.0' : updateData.service_version,
        releaseNotes: hasUpdate ? `## 版本 1.3.0 更新内容

### 新增功能
- 🚀 新增自动备份调度功能
- 📊 增强的性能监控面板
- 🔐 改进的安全认证机制

### 优化改进
- ⚡ 提升启动速度 50%
- 🐛 修复已知问题 15 项
- 💾 减少内存占用 20%

### 注意事项
- 本次更新需要重启服务
- 建议在业务低峰期进行升级` : '',
        downloadSize: hasUpdate ? 156 * 1024 * 1024 : 0, // 156MB
        releaseDate: hasUpdate ? '2024-01-25' : ''
      });
      
      if (hasUpdate) {
        message.info('发现新版本！');
      } else {
        message.success('当前已是最新版本');
      }
    } catch (error) {
      console.error('Failed to check updates:', error);
      message.error('检查更新失败');
    } finally {
      setLoading(false);
    }
  };

  const loadAutoConfig = () => {
    // TODO: 从后端加载自动升级配置
    setAutoConfig({
      enabled: false,
      schedule: '',
      delayTime: 2,
      delayUnit: 'hours'
    });
  };

  const startUpgrade = async (immediate = true) => {
    if (!immediate) {
      setScheduleModalVisible(true);
      return;
    }

    setUpgradeModalVisible(true);
    setUpgradeStatus('downloading');
    setCurrentStep(0);
    setUpgradeProgress(0);

    try {
      // 模拟下载过程
      for (let i = 0; i <= 50; i += 10) {
        setUpgradeProgress(i);
        await new Promise(resolve => setTimeout(resolve, 500));
      }

      setUpgradeStatus('installing');
      setCurrentStep(1);

      // 模拟安装过程
      for (let i = 50; i <= 100; i += 10) {
        setUpgradeProgress(i);
        await new Promise(resolve => setTimeout(resolve, 800));
      }

      setUpgradeStatus('complete');
      setCurrentStep(2);
      message.success('升级完成！');
      
      // 更新历史记录
      const newRecord: UpgradeHistory = {
        id: Date.now().toString(),
        version: updateInfo.latestVersion,
        date: new Date().toLocaleString('zh-CN'),
        status: 'success',
        type: 'manual',
        duration: '6分钟'
      };
      setUpgradeHistory(prev => [newRecord, ...prev]);
      
      // 更新版本信息
      setUpdateInfo(prev => ({
        ...prev,
        hasUpdate: false,
        currentVersion: prev.latestVersion
      }));

    } catch (error) {
      setUpgradeStatus('error');
      message.error('升级失败');
    }
  };

  const saveAutoConfig = async (values: any) => {
    try {
      // TODO: 保存自动升级配置到后端
      const newConfig = {
        enabled: values.enabled,
        schedule: values.schedule,
        delayTime: values.delayTime,
        delayUnit: values.delayUnit
      };
      setAutoConfig(newConfig);
      message.success('自动升级配置已保存');
      setScheduleModalVisible(false);
    } catch (error) {
      message.error('保存配置失败');
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'success':
        return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
      case 'failed':
        return <WarningOutlined style={{ color: '#ff4d4f' }} />;
      case 'in-progress':
        return <ClockCircleOutlined style={{ color: '#1677ff' }} />;
      default:
        return null;
    }
  };

  const formatFileSize = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  // 执行升级
  // 自动部署服务
  const handleAutoDeploy = async () => {
    Modal.confirm({
      title: '自动部署服务',
      content: (
        <div>
          <p>将从服务器拉取最新的Docker服务并自动部署到本地。</p>
          <p style={{ marginTop: 16 }}>
            <strong>部署端口: </strong>
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
          const result = await autoDeployService(port);
          message.success(result as string);
        } catch (error) {
          message.error(`自动部署失败: ${error}`);
        } finally {
          setDeployLoading(false);
        }
      },
    });
  };

  return (
    <div>
      <Title level={2}>升级管理</Title>

      {/* 版本状态卡片 */}
      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col span={24}>
          <Card>
            <Row align="middle">
              <Col span={16}>
                <Space direction="vertical" size="small">
                  <div>
                    <Text strong style={{ fontSize: 16 }}>当前版本：</Text>
                    <Tag color="blue" style={{ fontSize: 14 }}>{updateInfo.currentVersion}</Tag>
                  </div>
                  {updateInfo.hasUpdate && (
                    <div>
                      <Text strong style={{ fontSize: 16 }}>最新版本：</Text>
                      <Tag color="green" style={{ fontSize: 14 }}>{updateInfo.latestVersion}</Tag>
                    </div>
                  )}
                  <Text type="secondary">
                    {updateInfo.hasUpdate 
                      ? `发现新版本，大小：${formatFileSize(updateInfo.downloadSize)}`
                      : '当前已是最新版本'
                    }
                  </Text>
                </Space>
              </Col>
              <Col span={8} style={{ textAlign: 'right' }}>
                <Space>
                  <Button 
                    icon={<ReloadOutlined />} 
                    onClick={checkForUpdates}
                    loading={loading}
                  >
                    检查更新
                  </Button>
                  {updateInfo.hasUpdate && (
                    <>
                      <Button 
                        type="primary"
                        icon={<UploadOutlined />}
                        onClick={() => startUpgrade(true)}
                        disabled={upgradeStatus === 'downloading' || upgradeStatus === 'installing'}
                      >
                        立即升级
                      </Button>
                      <Button 
                        icon={<ClockCircleOutlined />}
                        onClick={() => startUpgrade(false)}
                      >
                        预约升级
                      </Button>
                    </>
                  )}
                  <Button 
                    type="primary" 
                    icon={<RocketOutlined />}
                    onClick={handleAutoDeploy}
                    loading={deployLoading}
                    style={{ backgroundColor: '#52c41a', borderColor: '#52c41a' }}
                  >
                    自动部署服务
                  </Button>
                </Space>
              </Col>
            </Row>
          </Card>
        </Col>
      </Row>

      {/* 更新内容预览 */}
      {updateInfo.hasUpdate && (
        <Card title="更新内容" style={{ marginBottom: 24 }}>
          <div style={{ whiteSpace: 'pre-wrap', lineHeight: 1.6 }}>
            {updateInfo.releaseNotes}
          </div>
        </Card>
      )}

      <Row gutter={[16, 16]}>
        {/* 自动升级配置 */}
        <Col span={12}>
          <Card title="自动升级配置" extra={<SettingOutlined />}>
            <Space direction="vertical" style={{ width: '100%' }}>
              <div>
                <Text strong>状态：</Text>
                <Tag color={autoConfig.enabled ? 'green' : 'red'}>
                  {autoConfig.enabled ? '已启用' : '已禁用'}
                </Tag>
              </div>
              {autoConfig.enabled && autoConfig.schedule && (
                <div>
                  <Text strong>调度计划：</Text>
                  <Text>{autoConfig.schedule}</Text>
                </div>
              )}
              <div>
                <Text strong>延迟升级：</Text>
                <Text>{autoConfig.delayTime} {autoConfig.delayUnit === 'hours' ? '小时' : autoConfig.delayUnit === 'minutes' ? '分钟' : '天'}</Text>
              </div>
              <Button 
                type="primary"
                onClick={() => setScheduleModalVisible(true)}
                style={{ marginTop: 8 }}
              >
                配置自动升级
              </Button>
            </Space>
          </Card>
        </Col>

        {/* 升级历史 */}
        <Col span={12}>
          <Card title="升级历史" extra={<HistoryOutlined />}>
            <Timeline
              items={upgradeHistory.slice(0, 5).map(record => ({
                dot: getStatusIcon(record.status),
                children: (
                  <div>
                    <div>
                      <Text strong>版本 {record.version}</Text>
                      <Tag color={record.type === 'auto' ? 'blue' : 'orange'} style={{ marginLeft: 8 }}>
                        {record.type === 'auto' ? '自动' : '手动'}
                      </Tag>
                    </div>
                    <div>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        {record.date} · 耗时 {record.duration}
                      </Text>
                    </div>
                  </div>
                )
              }))}
            />
            {upgradeHistory.length > 5 && (
              <Button type="link" size="small">
                查看更多历史记录
              </Button>
            )}
          </Card>
        </Col>
      </Row>

      {/* 升级进度模态框 */}
      <Modal
        title="升级进度"
        open={upgradeModalVisible}
        onCancel={() => setUpgradeModalVisible(false)}
        footer={upgradeStatus === 'complete' ? [
          <Button key="close" type="primary" onClick={() => setUpgradeModalVisible(false)}>
            完成
          </Button>
        ] : null}
        closable={upgradeStatus === 'complete' || upgradeStatus === 'error'}
        maskClosable={false}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Steps current={currentStep} status={upgradeStatus === 'error' ? 'error' : 'process'}>
            <Step title="下载更新包" />
            <Step title="安装更新" />
            <Step title="完成" />
          </Steps>
          
          <Progress 
            percent={upgradeProgress} 
            status={upgradeStatus === 'error' ? 'exception' : 'active'}
            style={{ margin: '24px 0' }}
          />
          
          {upgradeStatus === 'downloading' && (
            <Text>正在下载更新包...</Text>
          )}
          {upgradeStatus === 'installing' && (
            <Text>正在安装更新，请勿关闭应用...</Text>
          )}
          {upgradeStatus === 'complete' && (
            <Alert
              message="升级完成"
              description="系统已成功升级到最新版本，请重启应用以应用更改。"
              type="success"
              showIcon
            />
          )}
          {upgradeStatus === 'error' && (
            <Alert
              message="升级失败"
              description="升级过程中出现错误，请稍后重试或联系支持。"
              type="error"
              showIcon
            />
          )}
        </Space>
      </Modal>

      {/* 预约升级配置模态框 */}
      <Modal
        title="自动升级配置"
        open={scheduleModalVisible}
        onCancel={() => setScheduleModalVisible(false)}
        onOk={() => form.submit()}
        okText="保存"
        cancelText="取消"
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={saveAutoConfig}
          initialValues={autoConfig}
        >
          <Form.Item name="enabled" valuePropName="checked">
            <Switch checkedChildren="启用" unCheckedChildren="禁用" />
            <Text style={{ marginLeft: 8 }}>启用自动升级</Text>
          </Form.Item>

          <Form.Item
            name="schedule"
            label="升级时间计划"
            help="设置自动升级的执行时间"
          >
            <Input placeholder="例如：每天凌晨2点" />
          </Form.Item>

          <Form.Item label="延迟升级">
            <Input.Group compact>
              <Form.Item name="delayTime" noStyle>
                <Input style={{ width: '60%' }} placeholder="延迟时间" />
              </Form.Item>
              <Form.Item name="delayUnit" noStyle>
                <Select style={{ width: '40%' }}>
                  <Option value="minutes">分钟</Option>
                  <Option value="hours">小时</Option>
                  <Option value="days">天</Option>
                </Select>
              </Form.Item>
            </Input.Group>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default UpgradeManagement; 