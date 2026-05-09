//! `NSPanel` overlay window setup for macOS. The overlay needs to:
//! - Float above fullscreen apps (`PanelLevel::ScreenSaver`).
//! - Appear on every Space (`can_join_all_spaces` + `full_screen_auxiliary`).
//! - Never steal focus from the active app (`nonactivating_panel`,
//!   `focusable(false)`).
//! - Run animations at full rate even when unfocused
//!   (`background_throttling: Disabled`).
//!
//! Also exposes interruption-mode commands that resize the primary overlay
//! to fill the primary monitor and spawn one secondary panel per
//! non-primary monitor so the cat appears on every screen.

#![allow(dead_code)]

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(OverlayPanel {
        config: {
            can_become_key_window: false,
            is_floating_panel: true
        }
    })
}

pub const PRIMARY_OVERLAY_LABEL: &str = "overlay";
pub const SECONDARY_OVERLAY_PREFIX: &str = "overlay-monitor-";
/// Panel size leaves room around the visible circle for shadow/padding.
/// The actual cat circle inside the panel is ~64px (matches dock icons),
/// the extra 16px is padding so the round frame doesn't get clipped at
/// the panel's square edges.
pub const COMPANION_WIDTH: f64 = 80.0;
pub const COMPANION_HEIGHT: f64 = 80.0;
/// Edge insets from the bottom-right corner. `NSPanel` sits above the dock
/// visually (`ScreenSaver` level), so we pin tight to the corner instead of
/// trying to clear the dock height.
const COMPANION_RIGHT_INSET_PX: f64 = 20.0;
const COMPANION_BOTTOM_INSET_PX: f64 = 20.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayMode {
    Companion,
    Interruption,
}

pub const OVERLAY_MODE_EVENT: &str = "overlay-mode-changed";

#[derive(Default)]
pub struct OverlayState {
    companion_position: Mutex<Option<tauri::PhysicalPosition<i32>>>,
}

#[cfg(target_os = "macos")]
pub fn setup_primary_overlay(app: &AppHandle) -> tauri::Result<()> {
    log::info!(
        "[overlay] setup_primary_overlay starting; size={COMPANION_WIDTH}x{COMPANION_HEIGHT}"
    );
    create_overlay_window(
        app,
        PRIMARY_OVERLAY_LABEL,
        COMPANION_WIDTH,
        COMPANION_HEIGHT,
    )?;
    position_overlay_bottom_right(app, PRIMARY_OVERLAY_LABEL)?;
    if let Some(window) = app.get_webview_window(PRIMARY_OVERLAY_LABEL) {
        match (window.outer_position(), window.outer_size()) {
            (Ok(pos), Ok(size)) => {
                log::info!(
                    "[overlay] companion ready: physical pos=({}, {}) size=({}x{})",
                    pos.x,
                    pos.y,
                    size.width,
                    size.height
                );
            }
            _ => log::warn!("[overlay] couldn't read final pos/size after positioning"),
        }
    } else {
        log::warn!("[overlay] companion window missing after setup — to_panel probably failed");
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn setup_primary_overlay(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn create_overlay_window(
    app: &AppHandle,
    label: &str,
    width: f64,
    height: f64,
) -> tauri::Result<()> {
    use tauri::utils::config::BackgroundThrottlingPolicy;
    use tauri_nspanel::{CollectionBehavior, PanelLevel, StyleMask, WebviewWindowExt};

    if app.get_webview_window(label).is_some() {
        return Ok(());
    }

    let overlay =
        tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App("overlay.html".into()))
            .title("Paws The Scroll")
            .inner_size(width, height)
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
            .background_throttling(BackgroundThrottlingPolicy::Disabled)
            .build()?;

    match overlay.to_panel::<OverlayPanel>() {
        Ok(panel) => {
            panel.set_level(PanelLevel::ScreenSaver.value());
            panel.set_floating_panel(true);

            let behavior = CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary()
                .ignores_cycle();
            panel.set_collection_behavior(behavior.value());

            let style = StyleMask::empty().nonactivating_panel();
            panel.set_style_mask(style.value());

            panel.hide();
            std::thread::sleep(std::time::Duration::from_millis(50));
            panel.show();
            panel.order_front_regardless();
        }
        Err(error) => {
            log::warn!("[NSPanel] failed to convert {label} to NSPanel: {error:?}");
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn position_overlay_bottom_right(app: &AppHandle, label: &str) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(label) else {
        log::warn!("[overlay] position_overlay_bottom_right: window {label} missing");
        return Ok(());
    };
    let Some(monitor) = window.primary_monitor()? else {
        log::warn!("[overlay] position_overlay_bottom_right: no primary monitor");
        return Ok(());
    };
    let scale = monitor.scale_factor();
    let monitor_size = monitor.size();
    let monitor_pos = monitor.position();
    let logical_w = (f64::from(monitor_size.width) / scale).round();
    let logical_h = (f64::from(monitor_size.height) / scale).round();
    let x =
        (f64::from(monitor_pos.x) / scale + logical_w - COMPANION_WIDTH - COMPANION_RIGHT_INSET_PX)
            .round();
    let y = (f64::from(monitor_pos.y) / scale + logical_h
        - COMPANION_HEIGHT
        - COMPANION_BOTTOM_INSET_PX)
        .round();
    log::info!(
        "[overlay] positioning companion: monitor=({}x{} @scale {scale}) pos=({}, {}) -> logical xy=({x}, {y})",
        monitor_size.width, monitor_size.height, monitor_pos.x, monitor_pos.y
    );
    window.set_position(tauri::LogicalPosition::new(x, y))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn remember_companion_position(app: &AppHandle, window: &tauri::WebviewWindow) {
    let Some(state) = app.try_state::<OverlayState>() else {
        return;
    };
    let Ok(position) = window.outer_position() else {
        log::warn!("[overlay] couldn't read companion position before interruption");
        return;
    };
    let save_result = state.companion_position.lock();
    match save_result {
        Ok(mut saved_position) => {
            *saved_position = Some(position);
        }
        Err(error) => {
            log::warn!("[overlay] couldn't lock companion position state: {error}");
        }
    }
}

#[cfg(target_os = "macos")]
fn restore_companion_position(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
) -> tauri::Result<bool> {
    let Some(state) = app.try_state::<OverlayState>() else {
        return Ok(false);
    };
    let saved_position = state.companion_position.lock();
    let position = match saved_position {
        Ok(saved_position) => *saved_position,
        Err(error) => {
            log::warn!("[overlay] couldn't lock companion position state: {error}");
            None
        }
    };
    let Some(position) = position else {
        return Ok(false);
    };
    window.set_position(position)?;
    Ok(true)
}

/// Tray menu helper: drag the companion back to a known-visible spot. If the
/// panel got stuck off-screen or the user lost it, this puts it center-screen
/// at a slightly bigger size so they can find it, then snaps it back.
#[cfg(target_os = "macos")]
pub fn find_companion(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(PRIMARY_OVERLAY_LABEL) else {
        log::warn!("[overlay] find_companion: window missing");
        return Ok(());
    };
    let Some(monitor) = window.primary_monitor()? else {
        return Ok(());
    };
    let scale = monitor.scale_factor();
    let size = monitor.size();
    let pos = monitor.position();
    let logical_w = f64::from(size.width) / scale;
    let logical_h = f64::from(size.height) / scale;
    let center_x = f64::from(pos.x) / scale + (logical_w - COMPANION_WIDTH) / 2.0;
    let center_y = f64::from(pos.y) / scale + (logical_h - COMPANION_HEIGHT) / 2.0;
    log::info!("[overlay] find_companion: snapping to center ({center_x}, {center_y})");
    window.set_position(tauri::LogicalPosition::new(center_x, center_y))?;
    let _ = window.show();
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn find_companion(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

#[tauri::command]
pub fn enter_interruption_mode(app: AppHandle) -> Result<(), String> {
    set_interruption_mode(&app, true).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn exit_interruption_mode(app: AppHandle) -> Result<(), String> {
    set_interruption_mode(&app, false).map_err(|e| e.to_string())
}

fn set_interruption_mode(app: &AppHandle, interruption: bool) -> tauri::Result<()> {
    if interruption {
        expand_to_fullscreen(app)?;
        spawn_secondary_overlays(app)?;
    } else {
        despawn_secondary_overlays(app)?;
        shrink_to_companion(app)?;
    }
    let mode = if interruption {
        OverlayMode::Interruption
    } else {
        OverlayMode::Companion
    };
    if let Err(error) = app.emit(OVERLAY_MODE_EVENT, mode) {
        log::warn!("[overlay] failed to emit overlay mode event: {error}");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn expand_to_fullscreen(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(PRIMARY_OVERLAY_LABEL) else {
        return Ok(());
    };
    remember_companion_position(app, &window);
    if let Some(monitor) = window.primary_monitor()? {
        let scale = monitor.scale_factor();
        let size = monitor.size();
        let pos = monitor.position();
        let w = f64::from(size.width) / scale;
        let h = f64::from(size.height) / scale;
        let x = f64::from(pos.x) / scale;
        let y = f64::from(pos.y) / scale;
        window.set_size(tauri::LogicalSize::new(w, h))?;
        window.set_position(tauri::LogicalPosition::new(x, y))?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn shrink_to_companion(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(PRIMARY_OVERLAY_LABEL) else {
        return Ok(());
    };
    window.set_size(tauri::LogicalSize::new(COMPANION_WIDTH, COMPANION_HEIGHT))?;
    if !restore_companion_position(app, &window)? {
        position_overlay_bottom_right(app, PRIMARY_OVERLAY_LABEL)?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn spawn_secondary_overlays(app: &AppHandle) -> tauri::Result<()> {
    let Some(primary_window) = app.get_webview_window(PRIMARY_OVERLAY_LABEL) else {
        return Ok(());
    };
    let primary_monitor_name = primary_window
        .primary_monitor()?
        .and_then(|m| m.name().cloned());
    let monitors = primary_window.available_monitors()?;
    for (index, monitor) in monitors.into_iter().enumerate() {
        let monitor_name = monitor.name().cloned();
        if monitor_name == primary_monitor_name {
            continue;
        }
        let label = format!("{SECONDARY_OVERLAY_PREFIX}{index}");
        let scale = monitor.scale_factor();
        let size = monitor.size();
        let pos = monitor.position();
        let w = f64::from(size.width) / scale;
        let h = f64::from(size.height) / scale;
        let x = f64::from(pos.x) / scale;
        let y = f64::from(pos.y) / scale;
        create_overlay_window(app, &label, w, h)?;
        if let Some(window) = app.get_webview_window(&label) {
            window.set_size(tauri::LogicalSize::new(w, h))?;
            window.set_position(tauri::LogicalPosition::new(x, y))?;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn despawn_secondary_overlays(app: &AppHandle) -> tauri::Result<()> {
    let labels: Vec<String> = app
        .webview_windows()
        .into_keys()
        .filter(|label| label.starts_with(SECONDARY_OVERLAY_PREFIX))
        .collect();
    for label in labels {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
        }
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn expand_to_fullscreen<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<()> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn shrink_to_companion<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<()> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn spawn_secondary_overlays<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<()> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn despawn_secondary_overlays<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<()> {
    Ok(())
}
