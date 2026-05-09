mod activity;
mod cat_bases;
mod cat_state;
mod demo_trigger;
mod image_cache;
mod model;
mod openai;
mod overlay;
mod store;
mod task_outcome;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Respect RUST_LOG if set; otherwise default to info-level for our
    // crate so OpenAI calls, overlay positioning, and rembg attempts show
    // up in the dev terminal without the user having to know the env var.
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("paws_the_scroll_lib=info"),
    )
    .format_timestamp_secs()
    .try_init();

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build());

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());
    }

    builder
        .invoke_handler(tauri::generate_handler![
            store::get_user_profile,
            store::save_user_profile,
            store::get_cat,
            store::save_cat,
            store::get_settings,
            store::save_settings,
            store::record_task_event,
            store::list_task_events,
            store::list_aggregates,
            store::factory_reset,
            openai::generate_interruption_task,
            openai::generate_cat_portrait,
            openai::regen_cat_portrait,
            openai::seed_initial_portrait,
            openai::read_portrait_bytes,
            activity::request_interruption,
            activity::current_foreground,
            activity::accessibility_trusted,
            overlay::enter_interruption_mode,
            overlay::exit_interruption_mode,
            task_outcome::apply_task_outcome,
            task_outcome::list_cat_skills,
        ])
        .setup(|app| {
            overlay::setup_primary_overlay(app.handle())?;
            activity::start_watcher(app.handle().clone());
            demo_trigger::register(app.handle())?;
            tray::install(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
