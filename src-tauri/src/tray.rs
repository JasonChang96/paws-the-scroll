//! Menu-bar tray icon. Gives the user a way to summon the cat, find the
//! floating overlay if it wandered off-screen, open the dashboard, or
//! quit — without having to chase the panel around the desktop.

use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

/// Dedicated tray icon: just the cat illustration on a transparent
/// background, rendered at 44px (22pt @2x for retina menu bars). NOT the
/// rounded-squircle dock icon — menu-bar conventions want the silhouette
/// itself sitting on the bar.
const TRAY_ICON_PNG: &[u8] = include_bytes!("../icons/tray.png");

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let summon = MenuItemBuilder::with_id("summon", "Summon the cat").build(app)?;
    let find = MenuItemBuilder::with_id("find", "Find the cat").build(app)?;
    let dashboard = MenuItemBuilder::with_id("dashboard", "Open dashboard").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&summon)
        .item(&find)
        .item(&dashboard)
        .separator()
        .item(&quit)
        .build()?;

    let icon = Image::from_bytes(TRAY_ICON_PNG)?;

    TrayIconBuilder::with_id("paws-tray")
        // Template mode: the icon is a black silhouette + alpha channel,
        // and macOS auto-tints it to match the menu bar (white on dark,
        // black on light) just like every other native menu-bar item.
        .icon_as_template(true)
        .icon(icon)
        .tooltip("Paws The Scroll")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "summon" => {
                if let Err(error) = crate::activity::request_interruption(app.clone()) {
                    log::warn!("[tray] summon failed: {error}");
                }
            }
            "find" => {
                if let Err(error) = crate::overlay::find_companion(app) {
                    log::warn!("[tray] find-companion failed: {error}");
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
