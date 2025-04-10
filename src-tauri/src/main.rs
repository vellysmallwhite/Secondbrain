// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod crypto;
mod database;

use database::{DiaryDB, DiaryEntry, GraphData, Relationship};
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

#[tauri::command]
fn add_relationship(
    state: State<AppState>,
    parent_id: Option<String>,
    child_id: Option<String>,
    relationship_type: Option<String>,
) -> Result<String, String> {
    // Add debug logging
    println!("Debug: add_relationship called with parameters:");
    println!("  - parent_id: '{:?}'", parent_id);
    println!("  - child_id: '{:?}'", child_id);
    println!("  - relationship_type: '{:?}'", relationship_type);
    
    // Check if all parameters are None, which suggests an accidental or unintended call
    if parent_id.is_none() && child_id.is_none() && relationship_type.is_none() {
        println!("Debug: Empty relationship call detected and rejected");
        return Err("Empty relationship parameters - operation aborted".to_string());
    }
    
    // Validate required parameters
    let final_parent_id = parent_id.ok_or_else(|| "Parent ID is required".to_string())?;
    let final_child_id = child_id.ok_or_else(|| "Child ID is required".to_string())?;
    let final_relationship_type = relationship_type.unwrap_or_else(|| "depends_on".to_string());
    
    // Validate parameters
    if final_parent_id.is_empty() {
        println!("Debug: parent_id is empty!");
        return Err("Parent ID is required".to_string());
    }
    if final_child_id.is_empty() {
        println!("Debug: child_id is empty!");
        return Err("Child ID is required".to_string());
    }
    
    let db = state.db.lock().unwrap();
    db.add_relationship(&final_parent_id, &final_child_id, &final_relationship_type)
        .map_err(|e| {
            println!("Debug: Error in add_relationship: {}", e);
            e.to_string()
        })
}

#[tauri::command]
fn delete_relationship(state: State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db.delete_relationship(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_relationships(state: State<AppState>, diary_id: String) -> Result<Vec<Relationship>, String> {
    let db = state.db.lock().unwrap();
    db.get_relationships(&diary_id).map_err(|e| e.to_string())
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
            delete_diary,
            add_relationship,
            delete_relationship,
            get_relationships
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
