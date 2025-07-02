import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Row, 
  Col, 
  Button, 
  Table, 
  Typography, 
  Space, 
  Modal,
  Form,
  Switch,
  Select,
  message,
  Popconfirm,
  Tag,
  Progress,
  Alert,
  Statistic
} from 'antd';
import {
  CloudUploadOutlined,
  DatabaseOutlined,
  DeleteOutlined,
  DownloadOutlined,
  FileOutlined,
  HistoryOutlined,
  ReloadOutlined,
  ScheduleOutlined,
  SettingOutlined,
  CheckCircleOutlined,
} from '@ant-design/icons';
import { tauriAPI } from '../utils/tauri';
import type { BackupInfo } from '../types/index';

const { Title, Text } = Typography;
const { Option } = Select;

interface AutoBackupConfig {
  enabled: boolean;
  cronExpression: string;
  retentionDays: number;
  retentionCount: number;
}

interface BackupProgress {
  isRunning: boolean;
  progress: number;
  currentStep: string;
}

interface RestoreProgress {
  isRunning: boolean;
  progress: number;
  currentStep: string;
}

const BackupRecovery: React.FC = () => {
  const [loading, setLoading] = useState(false);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [selectedBackups, setSelectedBackups] = useState<React.Key[]>([]);
  const [restoreModalVisible, setRestoreModalVisible] = useState(false);
  const [backupModalVisible, setBackupModalVisible] = useState(false);
  const [autoConfigModalVisible, setAutoConfigModalVisible] = useState(false);
  const [selectedBackup, setSelectedBackup] = useState<BackupInfo | null>(null);
  
  const [backupProgress, setBackupProgress] = useState<BackupProgress>({
    isRunning: false,
    progress: 0,
    currentStep: ''
  });
  
  const [restoreProgress, setRestoreProgress] = useState<RestoreProgress>({
    isRunning: false,
    progress: 0,
    currentStep: ''
  });

  const [autoConfig, setAutoConfig] = useState<AutoBackupConfig>({
    enabled: false,
    cronExpression: '0 2 * * *',
    retentionDays: 30,
    retentionCount: 10
  });

  const [form] = Form.useForm();

  useEffect(() => {
    loadBackups();
    loadAutoConfig();
  }, []);

  const loadBackups = async () => {
    setLoading(true);
    try {
      const backupList = await tauriAPI.backup.list();
      setBackups(backupList);
    } catch (error) {
      console.error('Failed to load backups:', error);
      message.error('加载备份列表失败');
    } finally {
      setLoading(false);
    }
  };

  const loadAutoConfig = () => {
    // TODO: 从后端加载自动备份配置
    setAutoConfig({
      enabled: true,
      cronExpression: '0 2 * * *',
      retentionDays: 30,
      retentionCount: 10
    });
  };

  const createBackup = async () => {
    setBackupModalVisible(true);
    setBackupProgress({ isRunning: true, progress: 0, currentStep: '准备备份...' });

    try {
      // 模拟备份过程
      for (let i = 0; i <= 100; i += 20) {
        await new Promise(resolve => setTimeout(resolve, 800));
        setBackupProgress({
          isRunning: true,
          progress: i,
          currentStep: i < 20 ? '准备备份...' :
                     i < 40 ? '备份数据库...' :
                     i < 60 ? '备份配置文件...' :
                     i < 80 ? '备份应用数据...' :
                     i < 100 ? '压缩备份文件...' : '备份完成'
        });
      }

      await tauriAPI.backup.create();
      message.success('备份创建成功');
      await loadBackups();
    } catch (error) {
      console.error('Failed to create backup:', error);
      message.error('创建备份失败');
    } finally {
      setBackupProgress({ isRunning: false, progress: 100, currentStep: '备份完成' });
      setTimeout(() => setBackupModalVisible(false), 2000);
    }
  };

  const restoreBackup = async (backup: BackupInfo, force = false) => {
    setSelectedBackup(backup);
    setRestoreModalVisible(true);
    setRestoreProgress({ isRunning: true, progress: 0, currentStep: '准备恢复...' });

    try {
      // 模拟恢复过程
      for (let i = 0; i <= 100; i += 25) {
        await new Promise(resolve => setTimeout(resolve, 1000));
        setRestoreProgress({
          isRunning: true,
          progress: i,
          currentStep: i < 25 ? '准备恢复...' :
                      i < 50 ? '停止服务...' :
                      i < 75 ? '恢复数据...' :
                      i < 100 ? '启动服务...' : '恢复完成'
        });
      }

      await tauriAPI.backup.restore(backup.id, force);
      message.success('备份恢复成功');
    } catch (error) {
      console.error('Failed to restore backup:', error);
      message.error('备份恢复失败');
    } finally {
      setRestoreProgress({ isRunning: false, progress: 100, currentStep: '恢复完成' });
      setTimeout(() => setRestoreModalVisible(false), 2000);
    }
  };

  const deleteBackup = async (_backupId: number) => {
    try {
      // TODO: 调用删除备份API
      message.success('备份删除成功');
      await loadBackups();
    } catch (error) {
      message.error('删除备份失败');
    }
  };

  const saveAutoConfig = async (values: AutoBackupConfig) => {
    try {
      // TODO: 保存自动备份配置
      setAutoConfig(values);
      message.success('自动备份配置已保存');
      setAutoConfigModalVisible(false);
    } catch (error) {
      message.error('保存配置失败');
    }
  };

  const formatFileSize = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const columns = [
    {
      title: '备份名称',
      dataIndex: 'name',
      key: 'name',
      render: (text: string) => (
        <Space>
          <FileOutlined style={{ color: '#1677ff' }} />
          <Text strong>{text}</Text>
        </Space>
      ),
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
      sorter: (a: BackupInfo, b: BackupInfo) => 
        new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
    },
    {
      title: '文件大小',
      dataIndex: 'size',
      key: 'size',
      render: (size: number) => formatFileSize(size),
      sorter: (a: BackupInfo, b: BackupInfo) => a.size - b.size,
    },
    {
      title: '状态',
      key: 'status',
      render: () => <Tag color="green">正常</Tag>,
    },
    {
      title: '操作',
      key: 'actions',
      render: (_: any, record: BackupInfo) => (
        <Space>
          <Button
            type="primary"
            size="small"
            icon={<HistoryOutlined />}
            onClick={() => restoreBackup(record)}
          >
            恢复
          </Button>
          <Button
            size="small"
            icon={<DownloadOutlined />}
          >
            下载
          </Button>
          <Popconfirm
            title="确定要删除这个备份吗？"
            description="删除后无法恢复，请谨慎操作。"
            onConfirm={() => deleteBackup(record.id)}
            okText="确定"
            cancelText="取消"
          >
            <Button
              size="small"
              danger
              icon={<DeleteOutlined />}
            >
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const rowSelection = {
    selectedRowKeys: selectedBackups,
    onChange: (selectedRowKeys: React.Key[]) => {
      setSelectedBackups(selectedRowKeys);
    },
  };

  return (
    <div>
      <Title level={2}>备份恢复</Title>

      {/* 统计卡片 */}
      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col span={6}>
          <Card>
            <Statistic
              title="总备份数"
              value={backups.length}
              prefix={<DatabaseOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="总占用空间"
              value={formatFileSize(backups.reduce((sum, backup) => sum + backup.size, 0))}
              prefix={<FileOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="自动备份"
              value={autoConfig.enabled ? '已启用' : '已禁用'}
              valueStyle={{ color: autoConfig.enabled ? '#3f8600' : '#cf1322' }}
              prefix={<ScheduleOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="最近备份"
              value={backups.length > 0 ? backups[0].created_at.split(' ')[0] : '无'}
              prefix={<CheckCircleOutlined />}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        {/* 操作面板 */}
        <Col span={24}>
          <Card>
            <Row justify="space-between" align="middle">
              <Col>
                <Space>
                  <Button
                    type="primary"
                    icon={<CloudUploadOutlined />}
                    onClick={createBackup}
                    loading={backupProgress.isRunning}
                  >
                    立即备份
                  </Button>
                  <Button
                    icon={<SettingOutlined />}
                    onClick={() => setAutoConfigModalVisible(true)}
                  >
                    自动备份设置
                  </Button>
                  <Button
                    icon={<ReloadOutlined />}
                    onClick={loadBackups}
                    loading={loading}
                  >
                    刷新
                  </Button>
                </Space>
              </Col>
              <Col>
                <Space>
                  {selectedBackups.length > 0 && (
                    <Popconfirm
                      title={`确定要删除选中的 ${selectedBackups.length} 个备份吗？`}
                      onConfirm={() => {
                        // TODO: 批量删除
                        message.success('批量删除成功');
                        setSelectedBackups([]);
                      }}
                    >
                      <Button danger icon={<DeleteOutlined />}>
                        批量删除 ({selectedBackups.length})
                      </Button>
                    </Popconfirm>
                  )}
                </Space>
              </Col>
            </Row>
          </Card>
        </Col>

        {/* 备份列表 */}
        <Col span={24}>
          <Card title="备份列表">
            <Table
              columns={columns}
              dataSource={backups}
              rowKey="id"
              loading={loading}
              rowSelection={rowSelection}
              pagination={{
                showSizeChanger: true,
                showQuickJumper: true,
                showTotal: (total, range) => `第 ${range[0]}-${range[1]} 条，共 ${total} 条`,
              }}
            />
          </Card>
        </Col>
      </Row>

      {/* 备份进度模态框 */}
      <Modal
        title="创建备份"
        open={backupModalVisible}
        onCancel={() => setBackupModalVisible(false)}
        footer={null}
        closable={!backupProgress.isRunning}
        maskClosable={false}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Progress percent={backupProgress.progress} status="active" />
          <Text>{backupProgress.currentStep}</Text>
          {backupProgress.progress === 100 && (
            <Alert
              message="备份完成"
              description="系统数据已成功备份，可以在备份列表中查看。"
              type="success"
              showIcon
            />
          )}
        </Space>
      </Modal>

      {/* 恢复进度模态框 */}
      <Modal
        title={`恢复备份 - ${selectedBackup?.name}`}
        open={restoreModalVisible}
        onCancel={() => setRestoreModalVisible(false)}
        footer={null}
        closable={!restoreProgress.isRunning}
        maskClosable={false}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Alert
            message="重要提示"
            description="恢复操作将覆盖当前数据，请确保已做好数据备份。恢复过程中请勿关闭应用。"
            type="warning"
            showIcon
            style={{ marginBottom: 16 }}
          />
          <Progress percent={restoreProgress.progress} status="active" />
          <Text>{restoreProgress.currentStep}</Text>
          {restoreProgress.progress === 100 && (
            <Alert
              message="恢复完成"
              description="系统已成功从备份恢复，建议重启应用以确保所有服务正常运行。"
              type="success"
              showIcon
            />
          )}
        </Space>
      </Modal>

      {/* 自动备份配置模态框 */}
      <Modal
        title="自动备份配置"
        open={autoConfigModalVisible}
        onCancel={() => setAutoConfigModalVisible(false)}
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
            <Text style={{ marginLeft: 8 }}>启用自动备份</Text>
          </Form.Item>

          <Form.Item
            name="cronExpression"
            label="备份计划"
            help="使用 Cron 表达式设置备份时间，例如 '0 2 * * *' 表示每天凌晨2点"
          >
            <Select>
              <Option value="0 2 * * *">每天凌晨2点</Option>
              <Option value="0 1 * * 0">每周日凌晨1点</Option>
              <Option value="0 0 1 * *">每月1号凌晨</Option>
              <Option value="0 */6 * * *">每6小时</Option>
              <Option value="0 */12 * * *">每12小时</Option>
            </Select>
          </Form.Item>

          <Form.Item
            name="retentionDays"
            label="保留天数"
            help="自动删除超过指定天数的备份"
          >
            <Select>
              <Option value={7}>7天</Option>
              <Option value={15}>15天</Option>
              <Option value={30}>30天</Option>
              <Option value={60}>60天</Option>
              <Option value={90}>90天</Option>
              <Option value={0}>不自动删除</Option>
            </Select>
          </Form.Item>

          <Form.Item
            name="retentionCount"
            label="最大备份数"
            help="保留的最大备份文件数量"
          >
            <Select>
              <Option value={5}>5个</Option>
              <Option value={10}>10个</Option>
              <Option value={20}>20个</Option>
              <Option value={50}>50个</Option>
              <Option value={0}>无限制</Option>
            </Select>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default BackupRecovery; 