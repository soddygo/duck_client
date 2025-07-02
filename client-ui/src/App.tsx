
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { Layout } from 'antd';
import 'antd/dist/reset.css';
import './App.css';
import AppLayout from './components/Layout/AppLayout';
import Dashboard from './pages/Dashboard';
import ServiceManagement from './pages/ServiceManagement';
import UpgradeManagement from './pages/UpgradeManagement';
import BackupRecovery from './pages/BackupRecovery';
import Settings from './pages/Settings';
import About from './pages/About';

const { Content } = Layout;

function App() {
  return (
    <Router>
      <AppLayout>
        <Content style={{ padding: '24px', minHeight: '280px' }}>
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/dashboard" element={<Dashboard />} />
            <Route path="/service" element={<ServiceManagement />} />
            <Route path="/upgrade" element={<UpgradeManagement />} />
            <Route path="/backup" element={<BackupRecovery />} />
            <Route path="/settings" element={<Settings />} />
            <Route path="/about" element={<About />} />
          </Routes>
        </Content>
      </AppLayout>
    </Router>
  );
}

export default App;
