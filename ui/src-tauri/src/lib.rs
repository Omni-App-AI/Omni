mod callbacks;
mod commands;
mod dto;
mod events;
mod marketplace;
mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let state = AppState::initialize(app)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::send_message,
            commands::get_session_messages,
            commands::list_sessions,
            commands::create_session,
            commands::permission_respond,
            commands::permission_revoke,
            commands::kill_switch,
            commands::install_extension,
            commands::list_extensions,
            commands::activate_extension,
            commands::deactivate_extension,
            commands::uninstall_extension,
            commands::toggle_extension_enabled,
            commands::extension_config_get,
            commands::extension_config_set,
            commands::get_audit_log,
            commands::get_guardian_metrics,
            commands::guardian_override,
            commands::update_settings,
            commands::get_settings,
            commands::get_pending_blocks,
            commands::channel_list,
            commands::channel_list_types,
            commands::channel_create_instance,
            commands::channel_remove_instance,
            commands::channel_connect,
            commands::channel_disconnect,
            commands::channel_login,
            commands::channel_send,
            commands::channel_get_api_key,
            commands::binding_add,
            commands::binding_remove,
            commands::binding_list,
            commands::binding_list_for_extension,
            commands::create_extension_instance,
            commands::delete_extension_instance,
            commands::list_extension_instances,
            commands::update_extension_instance,
            commands::activate_extension_instance,
            commands::deactivate_extension_instance,
            commands::toggle_instance_enabled,
            commands::provider_list_types,
            commands::provider_list,
            commands::provider_add,
            commands::provider_update,
            commands::provider_remove,
            commands::provider_set_credential,
            commands::provider_delete_credential,
            commands::provider_test_credential,
            commands::marketplace_search,
            commands::marketplace_get_detail,
            commands::marketplace_get_categories,
            commands::marketplace_install,
            commands::marketplace_check_updates,
            commands::mcp_list_servers,
            commands::mcp_add_server,
            commands::mcp_remove_server,
            commands::mcp_update_server,
            commands::mcp_start_server,
            commands::mcp_stop_server,
            commands::mcp_restart_server,
            commands::mcp_get_server_tools,
            commands::flowchart_list,
            commands::flowchart_get,
            commands::flowchart_save,
            commands::flowchart_delete,
            commands::flowchart_toggle_enabled,
            commands::flowchart_validate,
            commands::flowchart_test,
            commands::env_vars_list,
            commands::env_vars_set,
            commands::env_vars_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
