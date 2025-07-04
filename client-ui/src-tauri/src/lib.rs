mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(commands::AppGlobalState::default())
        .invoke_handler(tauri::generate_handler![
            // 版本管理命令
            commands::version::get_version_info,
            
            // 升级管理命令
            commands::upgrade::check_upgrade_available,
            commands::upgrade::start_upgrade_download,
            commands::upgrade::simulate_upgrade_progress,
            
            // 服务管理命令
            commands::services::get_services_status,
            commands::services::start_services_monitoring,
            commands::services::start_services,
            commands::services::stop_services,
            commands::services::restart_services,
            
            // 系统检查命令
            commands::system::check_system_requirements,
            commands::system::get_platform,
            commands::system::check_system_storage,
            commands::system::check_storage_space,
            commands::system::open_file_manager,
            
            // 初始化命令
            commands::init::check_initialization_status,
            commands::init::init_client_with_progress,
            commands::init::download_and_deploy_services,
            commands::init::download_package_with_progress,
            
            // 目录管理命令
            commands::directory::get_app_state,
            commands::directory::set_working_directory,
            commands::directory::get_working_directory,
            commands::directory::reset_working_directory,
            commands::directory::open_directory,
            commands::directory::initialize_app_state,
            
            // UI配置命令
            commands::ui::get_ui_config,
            commands::ui::update_ui_config,
            
            // 日志管理命令
            commands::logs::get_activity_logs,
            
            // 任务管理命令
            commands::tasks::get_current_tasks,
            commands::tasks::cancel_task,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
