mod commands;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // On non-macOS, disable native decorations (custom titlebar used instead)
            #[cfg(not(target_os = "macos"))]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_decorations(false);
            }

            // Tray icon
            let show = MenuItem::with_id(app, "show", "Show SkillReg", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("SkillReg")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            window.show().ok();
                            window.set_focus().ok();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::read_config,
            commands::config::write_config,
            commands::auth::login_initiate,
            commands::auth::login_poll,
            commands::auth::login_with_token,
            commands::auth::whoami,
            commands::auth::logout,
            commands::auth::open_url,
            commands::local::scan_local_skills,
            commands::skills::list_skills,
            commands::skills::get_skill,
            commands::skills::search_skills,
            commands::skills::pull_skill,
            commands::skills::push_skill,
            commands::skills::uninstall_skill,
            commands::skills::check_updates,
            commands::env::get_env_vars,
            commands::env::set_env_vars,
            commands::env::delete_env_vars,
            commands::env::list_all_env_vars,
            commands::env::import_env_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
