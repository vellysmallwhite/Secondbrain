// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod crypto;
mod database;

use database::{DiaryDB, DiaryEntry, GraphData};
use std::sync::Mutex;
use tauri::State;

struct AppState {
    db: Mutex<DiaryDB>,
}

#[tauri::command]
fn save_diary(
    state: State<AppState>,
    id: Option<String>,
    title: String,
    content: String,
    tags: Vec<String>,
) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    db.save_diary(id.as_deref(), &title, &content, &tags)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_diary(state: State<AppState>, id: String) -> Result<DiaryEntry, String> {
    let db = state.db.lock().unwrap();
    db.get_diary(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_diaries(state: State<AppState>) -> Result<Vec<DiaryEntry>, String> {
    let db = state.db.lock().unwrap();
    db.list_diaries().map_err(|e| e.to_string())
}

#[tauri::command]
fn search_diaries_by_tag(state: State<AppState>, tag: String) -> Result<Vec<DiaryEntry>, String> {
    let db = state.db.lock().unwrap();
    db.search_diaries_by_tag(&tag).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_graph_data(state: State<AppState>) -> Result<GraphData, String> {
    let db = state.db.lock().unwrap();
    db.get_graph_data().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_diary(state: State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db.delete_diary(&id).map_err(|e| e.to_string())
}

fn main() {
    let db = DiaryDB::new();
    let app_state = AppState {
        db: Mutex::new(db),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            save_diary,
            get_diary,
            list_diaries,
            search_diaries_by_tag,
            get_graph_data,
            delete_diary
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
