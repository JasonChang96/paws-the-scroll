//! Menu-bar tray icon. Gives the user a way to summon the cat, open the
//! dashboard, or quit without having to find the floating overlay first.

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let summon = MenuItemBuilder::with_id("summon", "Summon the cat").build(app)?;
    let dashboard = MenuItemBuilder::with_id("dashboard", "Open dashboard").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&summon)
        .item(&dashboard)
        .separator()
        .item(&quit)
        .build()?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::Anyhow(anyhow::anyhow!("no default window icon")))?;

    TrayIconBuilder::with_id("paws-tray")
        .icon(icon)
        // Template mode: macOS auto-tints for light/dark menu bar.
        .icon_as_template(true)
        .tooltip("Paws The Scroll")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "summon" => {
                if let Err(error) = crate::activity::request_interruption(app.clone()) {
                    log::warn!("[tray] summon failed: {error}");
                }
            }
            "dashboard" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
