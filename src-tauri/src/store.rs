// `upsert_aggregate` and `today_local` are used by M3 (activity tracking) but
// not yet wired in — keep them compiling without dead-code spam at M0.
#![allow(dead_code)]

use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{de::DeserializeOwned, Serialize};
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_store::{Store, StoreExt};

pub const CAT_UPDATED_EVENT: &str = "cat-updated";

use crate::model::{ActivityAggregate, Cat, Settings, TaskEvent, UserProfile};

const STORE_FILE: &str = "paws-the-scroll.json";

const KEY_USER_PROFILE: &str = "user_profile";
const KEY_CAT: &str = "cat";
const KEY_SETTINGS: &str = "settings";
const KEY_TASK_EVENTS: &str = "task_events";
const KEY_AGGREGATES: &str = "activity_aggregates";

fn load_store<R: Runtime>(app: &AppHandle<R>) -> Result<Arc<Store<R>>> {
    app.store(STORE_FILE)
        .context("failed to open paws-the-scroll store")
}

fn read_typed<R: Runtime, T: DeserializeOwned>(app: &AppHandle<R>, key: &str) -> Result<Option<T>> {
    let store = load_store(app)?;
    let Some(value) = store.get(key) else {
        return Ok(None);
    };
    let typed = serde_json::from_value::<T>(value)
        .with_context(|| format!("failed to deserialize store key {key}"))?;
    Ok(Some(typed))
}

fn write_typed<R: Runtime, T: Serialize>(app: &AppHandle<R>, key: &str, value: &T) -> Result<()> {
    let store = load_store(app)?;
    let json = serde_json::to_value(value)
        .with_context(|| format!("failed to serialize store key {key}"))?;
    store.set(key, json);
    store
        .save()
        .with_context(|| format!("failed to persist store after writing {key}"))?;
    Ok(())
}

pub fn read_user_profile<R: Runtime>(app: &AppHandle<R>) -> Result<Option<UserProfile>> {
    read_typed(app, KEY_USER_PROFILE)
}

pub fn write_user_profile<R: Runtime>(app: &AppHandle<R>, profile: &UserProfile) -> Result<()> {
    write_typed(app, KEY_USER_PROFILE, profile)
}

pub fn read_cat<R: Runtime>(app: &AppHandle<R>) -> Result<Option<Cat>> {
    read_typed(app, KEY_CAT)
}

pub fn write_cat<R: Runtime>(app: &AppHandle<R>, cat: &Cat) -> Result<()> {
    write_typed(app, KEY_CAT, cat)?;
    if let Err(error) = app.emit(CAT_UPDATED_EVENT, cat) {
        log::warn!("[store] failed to emit cat-updated: {error}");
    }
    Ok(())
}

pub fn read_settings<R: Runtime>(app: &AppHandle<R>) -> Result<Settings> {
    Ok(read_typed::<R, Settings>(app, KEY_SETTINGS)?.unwrap_or_default())
}

pub fn write_settings<R: Runtime>(app: &AppHandle<R>, settings: &Settings) -> Result<()> {
    write_typed(app, KEY_SETTINGS, settings)
}

pub fn append_task_event<R: Runtime>(app: &AppHandle<R>, event: TaskEvent) -> Result<()> {
    let mut events: Vec<TaskEvent> = read_typed(app, KEY_TASK_EVENTS)?.unwrap_or_default();
    events.push(event);
    if events.len() > 5_000 {
        let excess = events.len() - 5_000;
        events.drain(0..excess);
    }
    write_typed(app, KEY_TASK_EVENTS, &events)
}

pub fn read_task_events<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<TaskEvent>> {
    Ok(read_typed(app, KEY_TASK_EVENTS)?.unwrap_or_default())
}

fn today_local() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

pub fn upsert_aggregate<R: Runtime>(
    app: &AppHandle<R>,
    update: impl FnOnce(&mut ActivityAggregate),
) -> Result<ActivityAggregate> {
    let mut aggregates: Vec<ActivityAggregate> =
        read_typed(app, KEY_AGGREGATES)?.unwrap_or_default();
    let date = today_local();
    let entry = if let Some(existing) = aggregates.iter_mut().find(|a| a.date == date) {
        existing
    } else {
        aggregates.push(ActivityAggregate {
            date: date.clone(),
            ..ActivityAggregate::default()
        });
        aggregates
            .last_mut()
            .expect("just pushed an aggregate; this should never fail")
    };
    update(entry);
    let snapshot = entry.clone();
    write_typed(app, KEY_AGGREGATES, &aggregates)?;
    Ok(snapshot)
}

pub fn read_aggregates<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<ActivityAggregate>> {
    Ok(read_typed(app, KEY_AGGREGATES)?.unwrap_or_default())
}

#[tauri::command]
pub fn get_user_profile(app: AppHandle) -> Result<Option<UserProfile>, String> {
    read_user_profile(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_user_profile(app: AppHandle, profile: UserProfile) -> Result<(), String> {
    write_user_profile(&app, &profile).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_cat(app: AppHandle) -> Result<Option<Cat>, String> {
    read_cat(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_cat(app: AppHandle, cat: Cat) -> Result<(), String> {
    write_cat(&app, &cat).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<Settings, String> {
    read_settings(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    write_settings(&app, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn record_task_event(app: AppHandle, event: TaskEvent) -> Result<(), String> {
    append_task_event(&app, event).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_task_events(app: AppHandle) -> Result<Vec<TaskEvent>, String> {
    read_task_events(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_aggregates(app: AppHandle) -> Result<Vec<ActivityAggregate>, String> {
    read_aggregates(&app).map_err(|e| e.to_string())
}
