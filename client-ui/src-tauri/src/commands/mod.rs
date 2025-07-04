// 导出所有模块
pub mod types;
pub mod version;
pub mod upgrade;
pub mod services;
pub mod system;
pub mod init;
pub mod directory;
pub mod ui;
pub mod logs;
pub mod tasks;

// 重新导出类型，以便在lib.rs中使用
pub use types::AppGlobalState; 