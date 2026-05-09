//! Hidden global shortcut that fires the interruption directly. Useful for
//! the hackathon stage demo: hit `Cmd+Ctrl+Opt+P` to summon the cat over
//! whatever the active app is, bypassing grace period and randomized window.

use tauri::{AppHandle, Runtime};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn register<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    let shortcut = Shortcut::new(
        Some(Modifiers::SUPER | Modifiers::CONTROL | Modifiers::ALT),
        Code::KeyP,
    );
    let manager = app.global_shortcut();
    if let Err(error) = manager.on_shortcut(shortcut, |handle, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            if let Err(error) = crate::activity::request_interruption(handle.clone()) {
                log::warn!("[demo-trigger] failed to fire interruption: {error}");
            }
        }
    }) {
        log::warn!("[demo-trigger] failed to register shortcut: {error}");
    } else {
        log::info!("[demo-trigger] registered Cmd+Ctrl+Opt+P");
    }
    Ok(())
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn register<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<()> {
    Ok(())
}
