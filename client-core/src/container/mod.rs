// 模块声明
mod command;
mod config;
mod image;
mod manager;
mod service;
mod types;

// 重新导出公共API
pub use types::{DockerManager, ServiceConfig, ServiceInfo, ServiceStatus};

// 导入测试模块
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn create_dummy_compose_file(dir: &Path) -> PathBuf {
        let compose_file = dir.join("docker-compose.yml");
        std::fs::write(
            &compose_file,
            r#"
version: '3.8'
services:
  test-service:
    image: nginx:alpine
    ports:
      - "8080:80"
"#,
        )
        .unwrap();
        compose_file
    }

    #[test]
    fn test_docker_manager_creation() {
        let dir = tempdir().unwrap();
        let compose_file = create_dummy_compose_file(dir.path());

        let manager = DockerManager::new(&compose_file).unwrap();
        assert_eq!(manager.get_compose_file(), compose_file);
    }

    #[test]
    fn test_docker_manager_with_nonexistent_file() {
        // 允许创建DockerManager实例，即使文件不存在
        let result = DockerManager::new("/nonexistent/docker-compose.yml");
        assert!(result.is_ok());

        // 但是compose_file_exists应该返回false
        let manager = result.unwrap();
        assert!(!manager.compose_file_exists());
    }

    #[test]
    fn test_service_info_parsing() {
        let manager = DockerManager {
            compose_file: PathBuf::from("test"),
        };

        let json_output = r#"{"Name":"test_service_1","State":"running","Image":"nginx:alpine","Ports":"0.0.0.0:8080->80/tcp"}
{"Name":"test_db_1","State":"exited","Image":"postgres:13","Ports":""}"#;

        let services = manager.parse_service_info(json_output).unwrap();
        assert_eq!(services.len(), 2);
        assert_eq!(services[0].name, "test_service_1");
        assert_eq!(services[0].status, ServiceStatus::Running);
        assert_eq!(services[1].status, ServiceStatus::Stopped);
    }
}
