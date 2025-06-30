use crate::docker_service::error::{DockerServiceError, DockerServiceResult};
use client_core::container::DockerManager;
use serde::{Deserialize, Serialize};

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
    #[allow(clippy::should_implement_trait)]
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

impl Default for HealthReport {
    fn default() -> Self {
        Self::new()
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

    /// 执行健康检查 - 使用 ducker 库
    pub async fn check_health(&self) -> DockerServiceResult<HealthReport> {
        let mut report = HealthReport::new();

        // 获取服务状态
        match self.docker_manager.get_services_status().await {
            Ok(services) => {
                info!("健康检查: 获取到 {} 个服务", services.len());
                for service in services {
                    let status = match service.status {
                        client_core::container::ServiceStatus::Running => ContainerStatus::Running,
                        client_core::container::ServiceStatus::Stopped => ContainerStatus::Stopped,
                        client_core::container::ServiceStatus::Unknown => ContainerStatus::Unknown,
                    };

                    let container = ContainerInfo {
                        name: service.name.clone(),
                        status,
                        image: service.image.clone(),
                        ports: service.ports.clone(),
                        uptime: None,
                        health: None,
                    };

                    report.add_container(container);
                }
            }
            Err(e) => {
                let error_msg = format!("ducker 获取服务状态失败: {e}");
                error!("{}", error_msg);
                report.add_error(error_msg);
            }
        }

        report.finalize();
        info!(
            "健康检查完成: {}/{} 容器运行正常",
            report.running_count, report.total_count
        );
        Ok(report)
    }

    /// 等待服务启动完成 - 智能等待策略
    pub async fn wait_for_services_ready(
        &self,
        timeout: Duration,
        check_interval: Duration,
    ) -> DockerServiceResult<HealthReport> {
        let start_time = Instant::now();
        let mut last_report = None;
        let mut first_check = true;

        info!("⏳ 开始检查服务启动状态，超时时间: {}秒", timeout.as_secs());

        loop {
            let elapsed = start_time.elapsed();
            if elapsed >= timeout {
                // 超时处理
                let final_report = last_report.unwrap_or_else(|| {
                    let mut report = HealthReport::new();
                    report.add_error("等待超时".to_string());
                    report.finalize();
                    report
                });

                // 清除最后的进度显示
                print!("\r");
                error!("⏰ 健康检查超时! 用时: {}秒", elapsed.as_secs());
                self.print_final_status(&final_report, false);

                return Err(DockerServiceError::Timeout {
                    operation: "等待服务启动".to_string(),
                    timeout_seconds: timeout.as_secs(),
                });
            }

            // 执行健康检查
            let report = self.check_health().await?;

            // 显示实时进度（使用 print! 刷新）
            self.print_progress(&report, elapsed, first_check);
            first_check = false;

            // 检查是否所有服务都已就绪
            match report.overall_status {
                ServiceStatus::AllRunning => {
                    // 所有服务都成功启动，立即返回
                    print!("\r");
                    info!("🎉 所有服务已成功启动! 用时: {}秒", elapsed.as_secs());
                    self.print_final_status(&report, true);
                    return Ok(report);
                }
                ServiceStatus::AllStopped => {
                    print!("\r");
                    warn!("❌ 所有服务都已停止");
                    self.print_final_status(&report, false);
                    return Err(DockerServiceError::ServiceManagement(
                        "所有服务都已停止".to_string(),
                    ));
                }
                ServiceStatus::PartiallyRunning | ServiceStatus::Starting => {
                    // 有服务正在启动或部分运行，继续等待
                    last_report = Some(report);
                }
                ServiceStatus::Unknown => {
                    // 状态未知，继续等待
                    last_report = Some(report);
                }
            }

            tokio::time::sleep(check_interval).await;
        }
    }

    /// 实时进度显示 - 使用print!刷新，避免过多日志
    fn print_progress(&self, report: &HealthReport, elapsed: Duration, is_first: bool) {
        let running_count = report.running_count;
        let total_count = report.total_count;
        let elapsed_secs = elapsed.as_secs();

        // 构建运行中的服务列表
        let running_services: Vec<&str> = report
            .containers
            .iter()
            .filter(|c| c.status.is_healthy())
            .map(|c| c.name.as_str())
            .collect();

        // 构建启动中的服务列表
        let starting_services: Vec<&str> = report
            .get_starting_containers()
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        // 构建失败的服务列表
        let failed_services: Vec<&str> = report
            .get_failed_containers()
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        // 构建状态信息
        let mut status_parts = vec![];

        if !running_services.is_empty() {
            status_parts.push(format!("✅ 运行中: {}", running_services.len()));
        }

        if !starting_services.is_empty() {
            status_parts.push(format!("⏳ 启动中: {}", starting_services.len()));
        }

        if !failed_services.is_empty() {
            status_parts.push(format!("❌ 失败: {}", failed_services.len()));
        }

        let status_text = if status_parts.is_empty() {
            "检查中...".to_string()
        } else {
            status_parts.join(" | ")
        };

        // 使用 \r 回到行首，覆盖之前的进度
        if is_first {
            println!(); // 第一次输出前加个换行
        }

        print!("\r🔍 [{running_count}/{total_count}] {status_text} | 用时: {elapsed_secs}秒");

        // 强制刷新输出
        use std::io::{self, Write};
        io::stdout().flush().unwrap_or(());
    }

    /// 打印最终状态信息
    fn print_final_status(&self, report: &HealthReport, success: bool) {
        println!(); // 换行，确保最终状态在新的一行显示

        if success {
            info!("=== ✅ 服务启动成功 ===");
        } else {
            error!("=== ❌ 服务启动失败 ===");
        }

        info!("总计: {}/{} 服务", report.running_count, report.total_count);

        // 显示运行中的服务
        let running_services: Vec<&str> = report
            .containers
            .iter()
            .filter(|c| c.status.is_healthy())
            .map(|c| c.name.as_str())
            .collect();

        if !running_services.is_empty() {
            info!("✅ 运行中的服务: {:?}", running_services);
        }

        // 显示失败的服务
        let failed_services: Vec<&str> = report
            .get_failed_containers()
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        if !failed_services.is_empty() {
            warn!("❌ 失败的服务: {:?}", failed_services);
        }

        // 显示启动中的服务
        let starting_services: Vec<&str> = report
            .get_starting_containers()
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        if !starting_services.is_empty() {
            warn!("⏳ 仍在启动的服务: {:?}", starting_services);
        }
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
            "未找到容器: {container_name}"
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
            summary.push_str(&format!("\n失败容器: {failed_names:?}"));
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
