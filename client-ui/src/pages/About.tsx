import React, { useState } from 'react';
import { 
  Card, 
  Row, 
  Col, 
  Typography, 
  Space, 
  Button,
  Tag,
  Descriptions,
  Avatar,
  Timeline,
  Modal
} from 'antd';
import {
  InfoCircleOutlined,
  GithubOutlined,
  GlobalOutlined,
  MailOutlined,
  LikeOutlined,
  StarOutlined,
  TeamOutlined,
  SafetyCertificateOutlined,
  BugOutlined,
  QuestionCircleOutlined,
  BookOutlined,
  HeartOutlined
} from '@ant-design/icons';

const { Title, Text, Paragraph } = Typography;

interface AppInfo {
  name: string;
  version: string;
  buildNumber: string;
  buildDate: string;
  platform: string;
  architecture: string;
  tauri_version: string;
  description: string;
}

interface Contributor {
  name: string;
  avatar: string;
  role: string;
  github?: string;
  email?: string;
}

interface VersionHistory {
  version: string;
  date: string;
  changes: string[];
}

const About: React.FC = () => {
  const [appInfo] = useState<AppInfo>({
    name: 'Duck Client',
    version: '1.0.10',
    buildNumber: '20240120.1',
    buildDate: '2024-01-20 15:30:00',
    platform: 'macOS',
    architecture: 'arm64',
    tauri_version: '2.0.0',
    description: '一个现代化的 Docker 服务管理和自动化部署工具，提供图形化界面管理 Docker Compose 服务。'
  });

  const [contributors] = useState<Contributor[]>([
    {
      name: 'soddygo',
      avatar: 'https://avatars.githubusercontent.com/u/soddygo',
      role: '项目创始人 & 主要开发者',
      github: 'https://github.com/soddygo',
      email: 'soddygo@example.com'
    },
    {
      name: 'Claude AI',
      avatar: 'https://avatars.githubusercontent.com/u/anthropic',
      role: 'AI 助手 & 代码贡献者',
      github: 'https://anthropic.com'
    }
  ]);

  const [versionHistory] = useState<VersionHistory[]>([
    {
      version: '1.0.10',
      date: '2024-01-20',
      changes: [
        '🚀 全新 Tauri 2.0 桌面客户端',
        '📊 增强的仪表盘和监控功能', 
        '🔧 改进的服务管理界面',
        '💾 自动备份和恢复功能',
        '⚡ 性能优化和bug修复'
      ]
    },
    {
      version: '1.0.9',
      date: '2024-01-15',
      changes: [
        '🐛 修复服务启动问题',
        '📈 性能监控优化',
        '🔐 安全性增强'
      ]
    },
    {
      version: '1.0.8',
      date: '2024-01-10',
      changes: [
        '✨ 新增自动升级功能',
        '🎨 UI界面优化',
        '📝 文档更新'
      ]
    }
  ]);

  const [licenseModalVisible, setLicenseModalVisible] = useState(false);

  const openExternalLink = (url: string) => {
    window.open(url, '_blank');
  };

  const licenseText = `MIT License

Copyright (c) 2024 soddygo

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.`;

  return (
    <div>
      <Title level={2}>关于</Title>

      <Row gutter={[16, 16]}>
        {/* 应用信息 */}
        <Col span={16}>
          <Card title={
            <Space>
              <InfoCircleOutlined />
              <span>应用信息</span>
            </Space>
          }>
            <Space direction="vertical" style={{ width: '100%' }} size="large">
              <div style={{ textAlign: 'center', padding: '20px 0' }}>
                <Avatar size={80} src="/tauri.svg" />
                <Title level={3} style={{ marginTop: 16, marginBottom: 8 }}>
                  {appInfo.name}
                </Title>
                <Text type="secondary">{appInfo.description}</Text>
              </div>

              <Descriptions column={2} bordered>
                <Descriptions.Item label="版本号">{appInfo.version}</Descriptions.Item>
                <Descriptions.Item label="构建编号">{appInfo.buildNumber}</Descriptions.Item>
                <Descriptions.Item label="构建日期">{appInfo.buildDate}</Descriptions.Item>
                <Descriptions.Item label="运行平台">{appInfo.platform}</Descriptions.Item>
                <Descriptions.Item label="系统架构">{appInfo.architecture}</Descriptions.Item>
                <Descriptions.Item label="Tauri 版本">{appInfo.tauri_version}</Descriptions.Item>
              </Descriptions>

              <Space style={{ width: '100%', justifyContent: 'center' }}>
                <Button 
                  type="primary" 
                  icon={<GithubOutlined />}
                  onClick={() => openExternalLink('https://github.com/soddygo/duck_client')}
                >
                  查看源码
                </Button>
                <Button 
                  icon={<BookOutlined />}
                  onClick={() => openExternalLink('https://duck-client.dev/docs')}
                >
                  使用文档
                </Button>
                <Button 
                  icon={<BugOutlined />}
                  onClick={() => openExternalLink('https://github.com/soddygo/duck_client/issues')}
                >
                  报告问题
                </Button>
                <Button 
                  icon={<SafetyCertificateOutlined />}
                  onClick={() => setLicenseModalVisible(true)}
                >
                  许可证
                </Button>
              </Space>
            </Space>
          </Card>
        </Col>

        {/* 技术栈 */}
        <Col span={8}>
          <Card title="技术栈" style={{ marginBottom: 16 }}>
            <Space direction="vertical" style={{ width: '100%' }}>
              <div>
                <Text strong>前端技术</Text>
                <div style={{ marginTop: 8 }}>
                  <Tag color="blue">React 18</Tag>
                  <Tag color="blue">TypeScript</Tag>
                  <Tag color="blue">Ant Design</Tag>
                  <Tag color="blue">Vite</Tag>
                </div>
              </div>
              
              <div>
                <Text strong>后端技术</Text>
                <div style={{ marginTop: 8 }}>
                  <Tag color="orange">Rust</Tag>
                  <Tag color="orange">Tauri 2.0</Tag>
                  <Tag color="orange">Tokio</Tag>
                  <Tag color="orange">DuckDB</Tag>
                </div>
              </div>

              <div>
                <Text strong>系统集成</Text>
                <div style={{ marginTop: 8 }}>
                  <Tag color="green">Docker</Tag>
                  <Tag color="green">Docker Compose</Tag>
                  <Tag color="green">SystemD</Tag>
                </div>
              </div>
            </Space>
          </Card>

          {/* 快速链接 */}
          <Card title="快速链接">
            <Space direction="vertical" style={{ width: '100%' }}>
              <Button 
                type="link" 
                block 
                icon={<GlobalOutlined />}
                onClick={() => openExternalLink('https://duck-client.dev')}
              >
                官方网站
              </Button>
              <Button 
                type="link" 
                block 
                icon={<BookOutlined />}
                onClick={() => openExternalLink('https://duck-client.dev/docs')}
              >
                使用文档
              </Button>
              <Button 
                type="link" 
                block 
                icon={<QuestionCircleOutlined />}
                onClick={() => openExternalLink('https://duck-client.dev/faq')}
              >
                常见问题
              </Button>
              <Button 
                type="link" 
                block 
                icon={<HeartOutlined />}
                onClick={() => openExternalLink('https://duck-client.dev/support')}
              >
                支持项目
              </Button>
            </Space>
          </Card>
        </Col>

        {/* 开发团队 */}
        <Col span={12}>
          <Card title={
            <Space>
              <TeamOutlined />
              <span>开发团队</span>
            </Space>
          }>
            <Space direction="vertical" style={{ width: '100%' }} size="large">
              {contributors.map((contributor, index) => (
                <div key={index} style={{ padding: '12px 0' }}>
                  <Space align="start">
                    <Avatar size={48} src={contributor.avatar} />
                    <div style={{ flex: 1 }}>
                      <div>
                        <Text strong style={{ fontSize: 16 }}>{contributor.name}</Text>
                      </div>
                      <div>
                        <Text type="secondary">{contributor.role}</Text>
                      </div>
                      <Space style={{ marginTop: 8 }}>
                        {contributor.github && (
                          <Button 
                            size="small" 
                            type="link" 
                            icon={<GithubOutlined />}
                            onClick={() => openExternalLink(contributor.github!)}
                          >
                            GitHub
                          </Button>
                        )}
                        {contributor.email && (
                          <Button 
                            size="small" 
                            type="link" 
                            icon={<MailOutlined />}
                            onClick={() => openExternalLink(`mailto:${contributor.email}`)}
                          >
                            Email
                          </Button>
                        )}
                      </Space>
                    </div>
                  </Space>
                </div>
              ))}
            </Space>
          </Card>
        </Col>

        {/* 版本历史 */}
        <Col span={12}>
          <Card title={
            <Space>
              <StarOutlined />
              <span>版本历史</span>
            </Space>
          }>
            <Timeline
              items={versionHistory.map(version => ({
                children: (
                  <div>
                    <div style={{ marginBottom: 8 }}>
                      <Tag color="blue">{version.version}</Tag>
                      <Text type="secondary" style={{ marginLeft: 8 }}>
                        {version.date}
                      </Text>
                    </div>
                    <ul style={{ margin: 0, paddingLeft: 20 }}>
                      {version.changes.map((change, index) => (
                        <li key={index}>
                          <Text>{change}</Text>
                        </li>
                      ))}
                    </ul>
                  </div>
                )
              }))}
            />
            <div style={{ marginTop: 16, textAlign: 'center' }}>
              <Button 
                type="link"
                onClick={() => openExternalLink('https://github.com/soddygo/duck_client/releases')}
              >
                查看完整版本历史
              </Button>
            </div>
          </Card>
        </Col>

        {/* 感谢信息 */}
        <Col span={24}>
          <Card>
            <div style={{ textAlign: 'center', padding: '20px 0' }}>
              <HeartOutlined style={{ fontSize: 32, color: '#ff4d4f', marginBottom: 16 }} />
              <Title level={4}>感谢使用 Duck Client</Title>
              <Paragraph style={{ fontSize: 16, maxWidth: 600, margin: '0 auto' }}>
                Duck Client 是一个开源项目，致力于为开发者提供简单易用的 Docker 服务管理工具。
                如果您觉得这个项目对您有帮助，欢迎给我们 Star ⭐ 或者贡献代码！
              </Paragraph>
              <Space style={{ marginTop: 24 }}>
                <Button 
                  type="primary" 
                  size="large"
                  icon={<GithubOutlined />}
                  onClick={() => openExternalLink('https://github.com/soddygo/duck_client')}
                >
                  Star on GitHub
                </Button>
                <Button 
                  size="large"
                  icon={<LikeOutlined />}
                  onClick={() => openExternalLink('https://duck-client.dev/support')}
                >
                  支持项目
                </Button>
              </Space>
            </div>
          </Card>
        </Col>
      </Row>

      {/* 许可证模态框 */}
      <Modal
        title="MIT 许可证"
        open={licenseModalVisible}
        onCancel={() => setLicenseModalVisible(false)}
        footer={[
          <Button key="close" onClick={() => setLicenseModalVisible(false)}>
            关闭
          </Button>
        ]}
        width={800}
      >
        <pre style={{ 
          backgroundColor: '#f5f5f5', 
          padding: 16, 
          borderRadius: 6,
          fontSize: 12,
          lineHeight: 1.4,
          maxHeight: 400,
          overflow: 'auto'
        }}>
          {licenseText}
        </pre>
      </Modal>
    </div>
  );
};

export default About; 