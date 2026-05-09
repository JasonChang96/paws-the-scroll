// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

// NSPanel type for the macOS overlay window. Configured as a non-activating,
// floating panel so showing/hiding never steals focus from the active app.
#[cfg(target_os = "macos")]
use tauri::Manager;

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(OverlayPanel {
        config: {
            can_become_key_window: false,
            is_floating_panel: true
        }
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .invoke_handler(tauri::generate_handler![greet])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            setup_overlay_panel(app.handle())?;
            #[cfg(not(target_os = "macos"))]
            let _ = app;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(target_os = "macos")]
fn setup_overlay_panel(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::utils::config::BackgroundThrottlingPolicy;
    use tauri_nspanel::{CollectionBehavior, PanelLevel, StyleMask, WebviewWindowExt};

    let overlay = tauri::WebviewWindowBuilder::new(
        app,
        "overlay",
        tauri::WebviewUrl::App("overlay.html".into()),
    )
    .title("Overlay")
    .inner_size(240.0, 80.0)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .focused(false)
    .focusable(false)
    .accept_first_mouse(true)
    .visible(true)
    .visible_on_all_workspaces(true)
    // The overlay is `focusable(false)`, so WebKit treats it as a background
    // window and throttles requestAnimationFrame / timers / CSS animations to
    // ~1Hz. Disable throttling so animations in the overlay run at full rate.
    .background_throttling(BackgroundThrottlingPolicy::Disabled)
    .build()?;

    match overlay.to_panel::<OverlayPanel>() {
        Ok(panel) => {
            // ScreenSaver level sits above fullscreen apps; PanelLevel::Floating
            // sits below them.
            panel.set_level(PanelLevel::ScreenSaver.value());
            panel.set_floating_panel(true);

            let behavior = CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary()
                .ignores_cycle();
            panel.set_collection_behavior(behavior.value());

            // Non-activating panel: clicking the overlay never steals focus
            // from the active app.
            let style = StyleMask::empty().nonactivating_panel();
            panel.set_style_mask(style.value());

            // The window server only re-evaluates collection behavior on certain
            // events. Without this hide/show cycle the overlay can fail to
            // appear on every Space — especially Spaces that existed before the
            // panel was created. order_front_regardless alone is not reliable.
            panel.hide();
            std::thread::sleep(std::time::Duration::from_millis(50));
            panel.show();
            panel.order_front_regardless();
        }
        Err(error) => {
            eprintln!("[NSPanel] Failed to convert overlay to NSPanel: {error:?}");
        }
    }

    Ok(())
}
