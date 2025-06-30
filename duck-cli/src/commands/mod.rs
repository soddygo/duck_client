mod auto_backup;
mod auto_upgrade_deploy;
mod backup;
mod check_update;
mod docker_service;
mod ducker;
mod status;
mod update;

// Status commands
pub use status::{run_api_info, run_status};

// Update commands
pub use update::run_upgrade;

// Backup commands
pub use backup::{run_backup, run_list_backups, run_rollback};

// Check update commands
pub use check_update::handle_check_update_command;

// Docker service commands
pub use docker_service::{
    check_docker_services_status, deploy_docker_services, extract_docker_service,
    list_docker_images_with_ducker, load_docker_images, restart_container, restart_docker_services,
    setup_image_tags, show_architecture_info, start_docker_services, stop_docker_services,
};

// Ducker commands
pub use ducker::run_ducker;

// Auto backup commands
pub use auto_backup::{
    configure_cron, run_auto_backup, set_enabled, show_status as show_auto_backup_status,
};

// Auto upgrade deploy commands
pub use auto_upgrade_deploy::{
    run_auto_upgrade_deploy, schedule_delayed_deploy, show_status as show_auto_upgrade_status,
};
