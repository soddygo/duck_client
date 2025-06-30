use super::types::{DockerManager, ServiceConfig};
use crate::{DuckError, Result};
use std::collections::HashSet;

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
            .map_err(|e| DuckError::Docker(format!("读取compose文件失败: {e}")))?;

        // 尝试解析YAML
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| DuckError::Docker(format!("解析compose文件失败: {e}")))?;

        // 导航到services部分
        let services = yaml
            .get("services")
            .ok_or_else(|| DuckError::Docker("compose文件中没有services部分".to_string()))?;

        let service = services
            .get(service_name)
            .ok_or_else(|| DuckError::Docker(format!("找不到服务: {service_name}")))?;

        let restart = service
            .get("restart")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(ServiceConfig { restart })
    }

    /// 获取 docker-compose.yml 中定义的所有服务名称
    pub async fn get_compose_service_names(&self) -> Result<HashSet<String>> {
        use std::fs;

        let content = fs::read_to_string(&self.compose_file)
            .map_err(|e| DuckError::Docker(format!("读取compose文件失败: {e}")))?;

        // 解析YAML
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| DuckError::Docker(format!("解析compose文件失败: {e}")))?;

        // 导航到services部分
        let services = yaml
            .get("services")
            .ok_or_else(|| DuckError::Docker("compose文件中没有services部分".to_string()))?;

        let mut service_names = HashSet::new();
        
        if let Some(services_map) = services.as_mapping() {
            for (key, _) in services_map {
                if let Some(service_name) = key.as_str() {
                    service_names.insert(service_name.to_string());
                }
            }
        }

        Ok(service_names)
    }

    /// 获取 docker-compose 项目名称
    /// 优先级：1. 环境变量 COMPOSE_PROJECT_NAME 2. compose文件所在目录名称
    pub fn get_compose_project_name(&self) -> String {
        // 首先检查环境变量
        if let Ok(project_name) = std::env::var("COMPOSE_PROJECT_NAME") {
            return project_name;
        }

        // 使用compose文件所在目录名称作为项目名称
        if let Some(parent_dir) = self.compose_file.parent() {
            if let Some(dir_name) = parent_dir.file_name() {
                if let Some(name_str) = dir_name.to_str() {
                    return name_str.to_string();
                }
            }
        }

        // 默认项目名称
        "docker".to_string()
    }

    /// 生成 docker-compose 容器名称模式
    /// Docker Compose 生成的容器名称格式：{项目名}_{服务名}_{实例号}
    pub fn generate_compose_container_patterns(&self, service_name: &str) -> Vec<String> {
        let project_name = self.get_compose_project_name();
        
        vec![
            // 标准格式：项目名_服务名_实例号
            format!("{project_name}_{service_name}_1"),
            format!("{project_name}-{service_name}-1"),
            // 无实例号格式
            format!("{project_name}_{service_name}"),
            format!("{project_name}-{service_name}"),
            // 直接服务名匹配
            service_name.to_string(),
        ]
    }
}
