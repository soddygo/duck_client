import React, { useState } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, theme } from 'antd';
import {
  DashboardOutlined,
  ControlOutlined,
  UploadOutlined,
  SafetyOutlined,
  SettingOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';

const { Sider, Header } = Layout;

interface AppLayoutProps {
  children: React.ReactNode;
}

const AppLayout: React.FC<AppLayoutProps> = ({ children }) => {
  const [collapsed, setCollapsed] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  const {
    token: { colorBgContainer, borderRadiusLG },
  } = theme.useToken();

  const menuItems = [
    {
      key: '/dashboard',
      icon: <DashboardOutlined />,
      label: '仪表盘',
    },
    {
      key: '/service',
      icon: <ControlOutlined />,
      label: '服务管理',
    },
    {
      key: '/upgrade',
      icon: <UploadOutlined />,
      label: '升级管理',
    },
    {
      key: '/backup',
      icon: <SafetyOutlined />,
      label: '备份恢复',
    },
    {
      key: '/settings',
      icon: <SettingOutlined />,
      label: '系统设置',
    },
    {
      key: '/about',
      icon: <InfoCircleOutlined />,
      label: '关于',
    },
  ];

  const handleMenuClick = (key: string) => {
    navigate(key);
  };

  const getCurrentKey = () => {
    if (location.pathname === '/') {
      return '/dashboard';
    }
    return location.pathname;
  };

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider 
        collapsible 
        collapsed={collapsed} 
        onCollapse={setCollapsed}
        theme="light"
        width={200}
        style={{
          borderRight: '1px solid #f0f0f0',
        }}
      >
        <div 
          style={{ 
            height: 64, 
            display: 'flex', 
            alignItems: 'center', 
            justifyContent: 'center',
            borderBottom: '1px solid #f0f0f0',
            fontSize: collapsed ? 14 : 16,
            fontWeight: 'bold',
            color: '#1677ff',
          }}
        >
          {collapsed ? 'DC' : 'Duck Client'}
        </div>
        <Menu
          theme="light"
          mode="inline"
          selectedKeys={[getCurrentKey()]}
          items={menuItems}
          onClick={({ key }) => handleMenuClick(key)}
          style={{ border: 'none' }}
        />
      </Sider>
      <Layout>
        <Header 
          style={{ 
            padding: '0 24px', 
            background: colorBgContainer,
            borderBottom: '1px solid #f0f0f0',
            display: 'flex',
            alignItems: 'center',
            fontSize: 18,
            fontWeight: 'bold',
          }}
        >
          Docker 服务管理平台
        </Header>
        <div
          style={{
            background: colorBgContainer,
            borderRadius: borderRadiusLG,
            overflow: 'auto',
            flex: 1,
          }}
        >
          {children}
        </div>
      </Layout>
    </Layout>
  );
};

export default AppLayout; 