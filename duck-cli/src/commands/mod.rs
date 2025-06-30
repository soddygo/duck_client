mod backup;
mod check_update;
mod status;
mod update;
mod docker_service;
mod ducker;
mod auto_backup;
mod auto_upgrade_deploy;

// Status commands
pub use status::{run_status, run_api_info};

// Update commands  
pub use update::run_upgrade;

// Backup commands
pub use backup::{run_backup, run_list_backups, run_rollback};

// Check update commands
pub use check_update::handle_check_update_command;

// Docker service commands
pub use docker_service::{
    deploy_docker_services, start_docker_services, stop_docker_services,
    restart_docker_services, check_docker_services_status, restart_container,
    load_docker_images, setup_image_tags, show_architecture_info
};

// Ducker commands
pub use ducker::run_ducker;

// Auto backup commands  
pub use auto_backup::{
    run_auto_backup, configure_cron, set_enabled,
    show_status as show_auto_backup_status
};

// Auto upgrade deploy commands
pub use auto_upgrade_deploy::{
    run_auto_upgrade_deploy, schedule_delayed_deploy,
    show_status as show_auto_upgrade_status  
}; 