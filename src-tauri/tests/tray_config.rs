#[test]
fn tauri_config_does_not_create_unhandled_default_tray_icon() {
    let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let config = std::fs::read_to_string(config_path).expect("tauri.conf.json should be readable");
    let value: serde_json::Value =
        serde_json::from_str(&config).expect("tauri.conf.json should be valid JSON");

    assert!(
        value
            .get("app")
            .and_then(|app| app.get("trayIcon"))
            .is_none(),
        "tray icon is created in Rust so it can own menu and click handlers"
    );
}
