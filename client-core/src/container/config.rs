use super::types::{DockerManager, ServiceConfig};
use crate::{DuckError, Result};

impl DockerManager {
    /// 检查服务是否是一次性任务（解析compose文件和名称模式判断）
    pub async fn is_oneshot_service(&self, service_name: &str) -> Result<bool> {
        // 首先尝试从 docker-compose.yml 文件解析 restart 策略
        if let Ok(service_config) = self.parse_service_config(service_name).await {
            if let Some(restart_policy) = service_config.restart {
                // restart: "no" 表示不自动重启，通常是一次性任务
                if restart_policy == "no" || restart_policy == "false" {
                    return Ok(true);
                }
                // restart: "always" 或 "unless-stopped" 表示应该一直运行
                if restart_policy == "always"
                    || restart_policy == "unless-stopped"
                    || restart_policy == "on-failure"
                {
                    return Ok(false);
                }
            }
        }

        // 回退到基于名称模式的判断
        let oneshot_patterns = [
            "init",
            "setup",
            "migration",
            "migrate",
            "seed",
            "bootstrap",
            "minio-init",
            "db-init",
            "setup-",
            "-init",
            "-setup",
        ];

        let service_name_lower = service_name.to_lowercase();
        for pattern in &oneshot_patterns {
            if service_name_lower.contains(pattern) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 解析docker-compose.yml文件中的服务配置
    pub async fn parse_service_config(&self, service_name: &str) -> Result<ServiceConfig> {
        use std::fs;

        let content = fs::read_to_string(&self.compose_file)
            .map_err(|e| DuckError::Docker(format!("读取compose文件失败: {}", e)))?;

        // 尝试解析YAML
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| DuckError::Docker(format!("解析compose文件失败: {}", e)))?;

        // 导航到services部分
        let services = yaml
            .get("services")
            .ok_or_else(|| DuckError::Docker("compose文件中没有services部分".to_string()))?;

        let service = services
            .get(service_name)
            .ok_or_else(|| DuckError::Docker(format!("找不到服务: {}", service_name)))?;

        let restart = service
            .get("restart")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(ServiceConfig { restart })
    }
}
