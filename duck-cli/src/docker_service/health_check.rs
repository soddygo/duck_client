use crate::docker_service::error::{DockerServiceError, DockerServiceResult};
use client_core::container::DockerManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// 容器状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerStatus {
    /// 运行中
    Running,
    /// 已停止
    Stopped,
    /// 正在启动
    Starting,
    /// 不健康
    Unhealthy,
    /// 未知状态
    Unknown,
}

impl ContainerStatus {
    /// 从字符串解析容器状态
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "running" | "up" => ContainerStatus::Running,
            "exited" | "stopped" | "down" => ContainerStatus::Stopped,
            "starting" | "restarting" => ContainerStatus::Starting,
            "unhealthy" => ContainerStatus::Unhealthy,
            _ => ContainerStatus::Unknown,
        }
    }

    /// 获取状态的显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            ContainerStatus::Running => "运行中",
            ContainerStatus::Stopped => "已停止",
            ContainerStatus::Starting => "启动中",
            ContainerStatus::Unhealthy => "不健康",
            ContainerStatus::Unknown => "未知",
        }
    }

    /// 判断状态是否健康
    pub fn is_healthy(&self) -> bool {
        matches!(self, ContainerStatus::Running)
    }

    /// 判断状态是否为过渡状态（需要继续等待）
    pub fn is_transitioning(&self) -> bool {
        matches!(self, ContainerStatus::Starting)
    }
}

/// 容器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    /// 容器名称
    pub name: String,
    /// 容器状态
    pub status: ContainerStatus,
    /// 镜像名称
    pub image: String,
    /// 端口映射
    pub ports: Vec<String>,
    /// 启动时间
    pub uptime: Option<String>,
    /// 健康检查状态
    pub health: Option<String>,
}

/// 服务整体状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    /// 所有服务都在运行
    AllRunning,
    /// 部分服务在运行
    PartiallyRunning,
    /// 所有服务都已停止
    AllStopped,
    /// 服务正在启动中
    Starting,
    /// 服务状态未知
    Unknown,
}

impl ServiceStatus {
    /// 获取状态的显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            ServiceStatus::AllRunning => "全部运行",
            ServiceStatus::PartiallyRunning => "部分运行",
            ServiceStatus::AllStopped => "全部停止",
            ServiceStatus::Starting => "启动中",
            ServiceStatus::Unknown => "未知",
        }
    }

    /// 判断状态是否健康
    pub fn is_healthy(&self) -> bool {
        matches!(self, ServiceStatus::AllRunning)
    }
}

/// 健康检查报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// 整体服务状态
    pub overall_status: ServiceStatus,
    /// 容器详细信息
    pub containers: Vec<ContainerInfo>,
    /// 运行中的容器数量
    pub running_count: usize,
    /// 总容器数量
    pub total_count: usize,
    /// 检查时间
    pub check_time: chrono::DateTime<chrono::Utc>,
    /// 错误信息
    pub errors: Vec<String>,
}

impl HealthReport {
    /// 创建新的健康检查报告
    pub fn new() -> Self {
        Self {
            overall_status: ServiceStatus::Unknown,
            containers: Vec::new(),
            running_count: 0,
            total_count: 0,
            check_time: chrono::Utc::now(),
            errors: Vec::new(),
        }
    }

    /// 添加容器信息
    pub fn add_container(&mut self, container: ContainerInfo) {
        if container.status.is_healthy() {
            self.running_count += 1;
        }
        self.total_count += 1;
        self.containers.push(container);
    }

    /// 添加错误信息
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// 完成报告并计算整体状态
    pub fn finalize(&mut self) {
        self.overall_status = if self.total_count == 0 {
            ServiceStatus::Unknown
        } else if self.running_count == self.total_count {
            ServiceStatus::AllRunning
        } else if self.running_count == 0 {
            ServiceStatus::AllStopped
        } else {
            // 检查是否有正在启动的容器
            let has_starting = self.containers.iter().any(|c| c.status.is_transitioning());
            if has_starting {
                ServiceStatus::Starting
            } else {
                ServiceStatus::PartiallyRunning
            }
        };
    }

    /// 获取失败的容器列表
    pub fn get_failed_containers(&self) -> Vec<&ContainerInfo> {
        self.containers
            .iter()
            .filter(|c| !c.status.is_healthy() && !c.status.is_transitioning())
            .collect()
    }

    /// 获取正在启动的容器列表
    pub fn get_starting_containers(&self) -> Vec<&ContainerInfo> {
        self.containers
            .iter()
            .filter(|c| c.status.is_transitioning())
            .collect()
    }
}

/// 健康检查器
pub struct HealthChecker {
    docker_manager: DockerManager,
}

impl HealthChecker {
    /// 创建新的健康检查器
    pub fn new(docker_manager: DockerManager) -> Self {
        Self { docker_manager }
    }

    /// 执行健康检查
    pub async fn check_health(&self) -> DockerServiceResult<HealthReport> {
        let mut report = HealthReport::new();

        // 获取服务状态
        match self.docker_manager.get_services_status().await {
            Ok(services) => {
                for service in services {
                    let container = ContainerInfo {
                        name: service.name,
                        status: ContainerStatus::from_str(&format!("{:?}", service.status)),
                        image: service.image,
                        ports: service.ports,
                        uptime: None,
                        health: None,
                    };
                    report.add_container(container);
                }
            }
            Err(e) => {
                let error_msg = format!("获取服务状态失败: {}", e);
                error!("{}", error_msg);
                report.add_error(error_msg);
            }
        }

        report.finalize();
        Ok(report)
    }

    /// 等待服务启动完成
    pub async fn wait_for_services_ready(
        &self,
        timeout: Duration,
        check_interval: Duration,
    ) -> DockerServiceResult<HealthReport> {
        let start_time = Instant::now();
        let mut last_report = None;

        info!("等待服务启动完成，超时时间: {:?}", timeout);

        loop {
            let elapsed = start_time.elapsed();
            if elapsed >= timeout {
                let final_report = last_report.unwrap_or_else(|| {
                    let mut report = HealthReport::new();
                    report.add_error("等待超时".to_string());
                    report.finalize();
                    report
                });

                return Err(DockerServiceError::Timeout {
                    operation: "等待服务启动".to_string(),
                    timeout_seconds: timeout.as_secs(),
                });
            }

            // 执行健康检查
            let report = self.check_health().await?;

            // 显示进度
            self.log_progress(&report, elapsed);

            // 检查是否所有服务都已就绪
            match report.overall_status {
                ServiceStatus::AllRunning => {
                    info!("所有服务已成功启动! 用时: {:?}", elapsed);
                    return Ok(report);
                }
                ServiceStatus::AllStopped => {
                    warn!("所有服务都已停止");
                    return Err(DockerServiceError::ServiceManagement(
                        "所有服务都已停止".to_string(),
                    ));
                }
                _ => {
                    // 继续等待
                }
            }

            last_report = Some(report);
            tokio::time::sleep(check_interval).await;
        }
    }

    /// 记录进度日志
    fn log_progress(&self, report: &HealthReport, elapsed: Duration) {
        let running_containers = report
            .containers
            .iter()
            .filter(|c| c.status.is_healthy())
            .map(|c| &c.name)
            .collect::<Vec<_>>();

        let starting_containers = report
            .get_starting_containers()
            .iter()
            .map(|c| &c.name)
            .collect::<Vec<_>>();

        if !running_containers.is_empty() {
            info!("已启动服务: {:?}", running_containers);
        }

        if !starting_containers.is_empty() {
            info!("启动中服务: {:?}", starting_containers);
        }

        info!(
            "进度: {}/{} 服务就绪, 用时: {:?}",
            report.running_count, report.total_count, elapsed
        );
    }

    /// 检查特定容器的状态
    pub async fn check_container_status(
        &self,
        container_name: &str,
    ) -> DockerServiceResult<ContainerInfo> {
        let report = self.check_health().await?;

        for container in report.containers {
            if container.name == container_name {
                return Ok(container);
            }
        }

        Err(DockerServiceError::ServiceManagement(format!(
            "未找到容器: {}",
            container_name
        )))
    }

    /// 获取服务状态摘要
    pub async fn get_status_summary(&self) -> DockerServiceResult<String> {
        let report = self.check_health().await?;

        let mut summary = format!(
            "服务状态: {} ({}/{})",
            report.overall_status.display_name(),
            report.running_count,
            report.total_count
        );

        if !report.errors.is_empty() {
            summary.push_str(&format!("\n错误: {}", report.errors.join(", ")));
        }

        let failed_containers = report.get_failed_containers();
        if !failed_containers.is_empty() {
            let failed_names: Vec<&str> =
                failed_containers.iter().map(|c| c.name.as_str()).collect();
            summary.push_str(&format!("\n失败容器: {:?}", failed_names));
        }

        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_status_from_str() {
        assert_eq!(
            ContainerStatus::from_str("running"),
            ContainerStatus::Running
        );
        assert_eq!(ContainerStatus::from_str("UP"), ContainerStatus::Running);
        assert_eq!(
            ContainerStatus::from_str("exited"),
            ContainerStatus::Stopped
        );
        assert_eq!(
            ContainerStatus::from_str("starting"),
            ContainerStatus::Starting
        );
        assert_eq!(
            ContainerStatus::from_str("unknown"),
            ContainerStatus::Unknown
        );
    }

    #[test]
    fn test_health_report() {
        let mut report = HealthReport::new();

        report.add_container(ContainerInfo {
            name: "service1".to_string(),
            status: ContainerStatus::Running,
            image: "test:latest".to_string(),
            ports: vec!["8080:8080".to_string()],
            uptime: None,
            health: None,
        });

        report.add_container(ContainerInfo {
            name: "service2".to_string(),
            status: ContainerStatus::Starting,
            image: "test2:latest".to_string(),
            ports: vec![],
            uptime: None,
            health: None,
        });

        report.finalize();

        assert_eq!(report.overall_status, ServiceStatus::Starting);
        assert_eq!(report.running_count, 1);
        assert_eq!(report.total_count, 2);
    }
}
