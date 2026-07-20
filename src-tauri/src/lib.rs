mod commands;

use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime, WindowEvent,
};

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            // On non-macOS, disable native decorations (custom titlebar used instead)
            #[cfg(not(target_os = "macos"))]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_decorations(false);
            }

            let open = MenuItem::with_id(app, "open", "Open SkillReg", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit SkillReg", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &separator, &quit])?;
            let tray_icon = Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(true)
                .icon(tray_icon)
                .icon_as_template(true)
                .tooltip("SkillReg")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main_window(app),
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            if let Ok(config) = commands::config::read_config() {
                let _ = commands::config::apply_launch_at_login(
                    app.handle(),
                    config.launch_at_login_value(),
                );
            }

            commands::auto_update::spawn_auto_update_worker(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::read_config,
            commands::config::write_config,
            commands::config::set_launch_at_login,
            commands::auth::login_initiate,
            commands::auth::login_poll,
            commands::auth::login_with_token,
            commands::auth::whoami,
            commands::auth::logout,
            commands::auth::open_url,
            commands::local::scan_local_skills,
            commands::collaboration::propose_skill_change,
            commands::collaboration::list_skill_proposals,
            commands::collaboration::get_skill_proposal,
            commands::skills::list_skills,
            commands::skills::get_skill,
            commands::skills::search_skills,
            commands::skills::pull_skill,
            commands::skills::get_catalog_policy,
            commands::skills::list_catalog_skills,
            commands::skills::install_catalog_skill,
            commands::skills::push_skill,
            commands::skills::uninstall_skill,
            commands::skills::delete_skill,
            commands::skills::check_updates,
            commands::slash_commands::list_commands,
            commands::slash_commands::get_command,
            commands::slash_commands::pull_command,
            commands::slash_commands::list_local_commands,
            commands::slash_commands::remove_command,
            commands::slash_commands::update_command,
            commands::installed_manifest::list_tracked_installations,
            commands::installed_manifest::set_skill_auto_update,
            commands::auto_update::run_auto_update_now,
            commands::env::get_env_vars,
            commands::env::get_org_env_var,
            commands::env::set_env_vars,
            commands::env::set_org_env_var,
            commands::env::delete_env_vars,
            commands::env::delete_org_env_var,
            commands::env::list_org_env_vars,
            commands::env::list_all_env_vars,
            commands::env::preview_legacy_env_migration,
            commands::env::migrate_legacy_env_vars,
            commands::env::cleanup_legacy_env_vars,
            commands::env::migrate_org_env_file_to_secure_store,
            commands::env::import_env_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
