use super::error::{DockerServiceError, DockerServiceResult};
use serde_yaml::Value;

use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use tracing::{error, info, warn};

/// 端口映射信息
#[derive(Debug, Clone)]
pub struct PortMapping {
    /// 主机端口
    pub host_port: u16,
    /// 容器端口
    pub container_port: u16,
    /// 协议类型 (tcp/udp)
    pub protocol: String,
    /// 服务名称
    pub service_name: String,
}

/// 端口冲突检查结果
#[derive(Debug)]
pub struct PortConflictReport {
    /// 有冲突的端口
    pub conflicted_ports: Vec<PortConflict>,
    /// 检查的端口总数
    pub total_checked: usize,
    /// 是否有冲突
    pub has_conflicts: bool,
}

/// 端口冲突详情
#[derive(Debug)]
pub struct PortConflict {
    /// 端口号
    pub port: u16,
    /// 服务名称
    pub service_name: String,
    /// 端口映射信息
    pub mapping: String,
}

/// 端口管理器 - 负责检测和管理端口冲突
#[derive(Debug, Clone)]
pub struct PortManager {
    /// 保留端口列表
    reserved_ports: Vec<u16>,
}

impl PortManager {
    /// 创建新的端口管理器
    pub fn new() -> Self {
        Self {
            reserved_ports: Vec::new(),
        }
    }

    /// 检查端口是否可用（实际检测系统端口占用）
    pub fn is_port_available(&self, port: u16) -> bool {
        // 检查是否在保留端口列表中
        if self.reserved_ports.contains(&port) {
            return false;
        }

        // 先检查 0.0.0.0（所有接口），这是最严格的检查
        // 如果能绑定 0.0.0.0，说明端口确实可用
        match TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))) {
            Ok(listener) => {
                // 显式drop以立即释放端口
                drop(listener);
                true
            }
            Err(_) => {
                // 如果 0.0.0.0 绑定失败，再尝试 127.0.0.1
                // 这可以检测是否只是权限问题（某些系统上普通用户无法绑定 0.0.0.0）
                match TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))) {
                    Ok(listener) => {
                        drop(listener);
                        // 能绑定本地回环但不能绑定所有接口，可能是权限限制
                        // 这种情况下我们认为端口可用（但可能需要提醒用户）
                        warn!("端口 {} 只能绑定到 127.0.0.1，可能存在权限限制", port);
                        true
                    }
                    Err(_) => {
                        // 连本地回环都绑定不了，端口确实被占用
                        false
                    }
                }
            }
        }
    }

    /// 获取可用端口
    #[allow(dead_code)]
    pub fn get_available_port(&self, preferred_port: u16) -> DockerServiceResult<u16> {
        if self.is_port_available(preferred_port) {
            Ok(preferred_port)
        } else {
            // 简单的端口递增策略
            for port in (preferred_port + 1)..=(preferred_port + 100) {
                if self.is_port_available(port) {
                    return Ok(port);
                }
            }
            Err(DockerServiceError::Configuration(format!(
                "无法找到从 {preferred_port} 开始的可用端口"
            )))
        }
    }

    /// 保留端口
    #[allow(dead_code)]
    pub fn reserve_port(&mut self, port: u16) {
        if !self.reserved_ports.contains(&port) {
            self.reserved_ports.push(port);
        }
    }

    /// 从docker-compose.yml文件中解析端口映射
    pub async fn parse_compose_ports(
        &self,
        compose_file_path: &Path,
    ) -> DockerServiceResult<Vec<PortMapping>> {
        let content = std::fs::read_to_string(compose_file_path).map_err(|e| {
            DockerServiceError::Configuration(format!(
                "无法读取docker-compose文件 {}: {}",
                compose_file_path.display(),
                e
            ))
        })?;

        let yaml: Value = serde_yaml::from_str(&content).map_err(|e| {
            DockerServiceError::Configuration(format!("解析docker-compose文件失败: {e}"))
        })?;

        let mut port_mappings = Vec::new();

        if let Some(services) = yaml.get("services").and_then(|s| s.as_mapping()) {
            for (service_name, service_config) in services {
                let service_name = service_name.as_str().unwrap_or("unknown").to_string();

                if let Some(ports) = service_config.get("ports").and_then(|p| p.as_sequence()) {
                    for port_def in ports {
                        if let Some(port_mapping) =
                            self.parse_port_definition(port_def, &service_name)?
                        {
                            port_mappings.push(port_mapping);
                        }
                    }
                }
            }
        }

        Ok(port_mappings)
    }

    /// 解析单个端口定义
    fn parse_port_definition(
        &self,
        port_def: &Value,
        service_name: &str,
    ) -> DockerServiceResult<Option<PortMapping>> {
        match port_def {
            Value::String(port_str) => {
                // 格式: "8080:80" 或 "127.0.0.1:8080:80" 或 "8080:80/tcp"
                let port_str = port_str.trim();

                // 提取协议
                let (port_part, protocol) = if port_str.contains('/') {
                    let parts: Vec<&str> = port_str.split('/').collect();
                    (parts[0], parts.get(1).unwrap_or(&"tcp").to_string())
                } else {
                    (port_str, "tcp".to_string())
                };

                // 解析端口映射
                let ports: Vec<&str> = port_part.split(':').collect();
                match ports.len() {
                    2 => {
                        // "8080:80"
                        let host_port = ports[0].parse::<u16>().map_err(|_| {
                            DockerServiceError::Configuration(format!(
                                "无效的主机端口: {}",
                                ports[0]
                            ))
                        })?;
                        let container_port = ports[1].parse::<u16>().map_err(|_| {
                            DockerServiceError::Configuration(format!(
                                "无效的容器端口: {}",
                                ports[1]
                            ))
                        })?;

                        Ok(Some(PortMapping {
                            host_port,
                            container_port,
                            protocol,
                            service_name: service_name.to_string(),
                        }))
                    }
                    3 => {
                        // "127.0.0.1:8080:80"
                        let host_port = ports[1].parse::<u16>().map_err(|_| {
                            DockerServiceError::Configuration(format!(
                                "无效的主机端口: {}",
                                ports[1]
                            ))
                        })?;
                        let container_port = ports[2].parse::<u16>().map_err(|_| {
                            DockerServiceError::Configuration(format!(
                                "无效的容器端口: {}",
                                ports[2]
                            ))
                        })?;

                        Ok(Some(PortMapping {
                            host_port,
                            container_port,
                            protocol,
                            service_name: service_name.to_string(),
                        }))
                    }
                    _ => {
                        warn!("无法解析端口定义: {}", port_str);
                        Ok(None)
                    }
                }
            }
            Value::Number(port_num) => {
                // 仅容器端口，没有主机端口映射
                if let Some(port) = port_num.as_u64() {
                    if port <= 65535 {
                        // 这种情况下没有主机端口映射，不需要检查冲突
                        Ok(None)
                    } else {
                        Err(DockerServiceError::Configuration(format!(
                            "端口号超出范围: {port}"
                        )))
                    }
                } else {
                    Ok(None)
                }
            }
            _ => {
                warn!("未知的端口定义格式: {:?}", port_def);
                Ok(None)
            }
        }
    }

    /// 检查docker-compose.yml中定义的端口是否有冲突
    pub async fn check_compose_port_conflicts(
        &self,
        compose_file_path: &Path,
    ) -> DockerServiceResult<PortConflictReport> {
        info!(
            "开始检查docker-compose文件的端口冲突: {}",
            compose_file_path.display()
        );

        let port_mappings = self.parse_compose_ports(compose_file_path).await?;
        let mut conflicted_ports = Vec::new();
        let total_checked = port_mappings.len();

        for mapping in &port_mappings {
            if !self.is_port_available(mapping.host_port) {
                warn!(
                    "发现端口冲突: 端口 {} 已被占用 (服务: {})",
                    mapping.host_port, mapping.service_name
                );

                conflicted_ports.push(PortConflict {
                    port: mapping.host_port,
                    service_name: mapping.service_name.clone(),
                    mapping: format!(
                        "{}:{}/{}",
                        mapping.host_port, mapping.container_port, mapping.protocol
                    ),
                });
            } else {
                info!(
                    "端口 {} 可用 (服务: {})",
                    mapping.host_port, mapping.service_name
                );
            }
        }

        let has_conflicts = !conflicted_ports.is_empty();

        if has_conflicts {
            error!(
                "发现 {} 个端口冲突，共检查 {} 个端口",
                conflicted_ports.len(),
                total_checked
            );
        } else {
            info!(
                "端口检查完成，没有发现冲突，共检查 {} 个端口",
                total_checked
            );
        }

        Ok(PortConflictReport {
            conflicted_ports,
            total_checked,
            has_conflicts,
        })
    }

    /// 显示端口冲突报告
    pub fn print_conflict_report(&self, report: &PortConflictReport) {
        if report.has_conflicts {
            warn!("⚠️  发现端口冲突!");
            warn!("总计检查: {} 个端口映射", report.total_checked);
            warn!("冲突数量: {} 个", report.conflicted_ports.len());

            warn!("冲突详情:");
            for conflict in &report.conflicted_ports {
                warn!("  🔴 端口 {} 已被占用", conflict.port);
                warn!("     服务: {}", conflict.service_name);
                warn!("     映射: {}", conflict.mapping);
            }

            info!("💡 解决建议:");
            info!("  1. 停止占用端口的其他进程");
            info!("  2. 修改docker-compose.yml中的端口映射");
            info!("  3. 使用以下命令查看端口占用情况:");

            for conflict in &report.conflicted_ports {
                info!("     lsof -i :{}", conflict.port);
            }
        } else {
            info!("✅ 端口检查通过，没有发现冲突");
            info!("总计检查: {} 个端口映射", report.total_checked);
        }
    }
}

impl Default for PortManager {
    fn default() -> Self {
        Self::new()
    }
}
