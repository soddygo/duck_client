use super::types::DockerManager;
use crate::Result;
use std::path::Path;

impl DockerManager {
    /// 创建新的 Docker 管理器
    pub fn new<P: AsRef<Path>>(compose_file: P) -> Result<Self> {
        let compose_file = compose_file.as_ref().to_path_buf();

        // 不再在初始化时检查文件存在性，而是在实际执行命令时检查
        // 这样允许在首次使用时创建DockerManager实例
        Ok(Self { compose_file })
    }

    /// 检查 Docker Compose 文件是否存在
    pub fn compose_file_exists(&self) -> bool {
        self.compose_file.exists()
    }

    /// 获取 Docker Compose 文件路径
    pub fn get_compose_file(&self) -> &Path {
        &self.compose_file
    }

    /// 获取 Docker Compose 工作目录
    pub fn get_working_directory(&self) -> Option<&Path> {
        self.compose_file.parent()
    }
}
