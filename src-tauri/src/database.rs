use crate::crypto::Crypto;
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, Result as SqliteResult, ToSql};
use serde::{Deserialize, Serialize};
use std::{fs, sync::Arc};
use uuid::Uuid;

type DbPool = Pool<SqliteConnectionManager>;

#[derive(Debug, Serialize, Deserialize)]
pub struct DiaryEntry {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub properties: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub struct DiaryDB {
    pool: DbPool,
    crypto: Arc<Crypto>,
}

impl DiaryDB {
    pub fn new() -> Self {
        let db_path = Self::get_db_path();
        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::new(manager).expect("Failed to create database pool");
        
        let crypto = Arc::new(Crypto::new());
        
        let db = Self {
            pool,
            crypto,
        };
        
        db.initialize_db().expect("Failed to initialize database");
        db
    }
    
    fn get_db_path() -> String {
        let proj_dirs = ProjectDirs::from("com", "secondbrian", "diary")
            .expect("Failed to get project directories");
        let data_dir = proj_dirs.data_dir();
        fs::create_dir_all(data_dir).expect("Failed to create data directory");
        data_dir.join("diary.db").to_str().unwrap().to_string()
    }
    
    fn initialize_db(&self) -> SqliteResult<()> {
        let conn = self.pool.get().expect("Failed to get database connection");
        
        // Create diary entries table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS diary_entries (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;
        
        // Create tags table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            )",
            [],
        )?;
        
        // Create relationship table between diary entries and tags
        conn.execute(
            "CREATE TABLE IF NOT EXISTS diary_tags (
                diary_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                PRIMARY KEY (diary_id, tag_id),
                FOREIGN KEY (diary_id) REFERENCES diary_entries (id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags (id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        Ok(())
    }
    
    pub fn save_diary(&self, title: &str, content: &str, tags: &[String]) -> SqliteResult<String> {
        let conn = self.pool.get().expect("Failed to get database connection");
        let encrypted_content = self.crypto.encrypt(content);
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        
        conn.execute(
            "INSERT INTO diary_entries (id, title, content, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, title, encrypted_content, now_str, now_str],
        )?;
        
        // Process tags
        for tag_name in tags {
            let tag_id = self.get_or_create_tag(&conn, tag_name)?;
            
            // Create relationship
            conn.execute(
                "INSERT OR IGNORE INTO diary_tags (diary_id, tag_id) VALUES (?1, ?2)",
                params![id, tag_id],
            )?;
        }
        
        Ok(id)
    }
    
    fn get_or_create_tag(&self, conn: &Connection, tag_name: &str) -> SqliteResult<String> {
        // Try to find existing tag
        let mut stmt = conn.prepare("SELECT id FROM tags WHERE name = ?1")?;
        let mut rows = stmt.query(params![tag_name])?;
        
        if let Some(row) = rows.next()? {
            return Ok(row.get(0)?);
        }
        
        // Create new tag if not found
        let tag_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO tags (id, name) VALUES (?1, ?2)",
            params![tag_id, tag_name],
        )?;
        
        Ok(tag_id)
    }
    
    pub fn get_diary(&self, id: &str) -> SqliteResult<DiaryEntry> {
        let conn = self.pool.get().expect("Failed to get database connection");
        
        let mut stmt = conn.prepare(
            "SELECT id, title, content, created_at, updated_at FROM diary_entries WHERE id = ?1"
        )?;
        
        let mut rows = stmt.query(params![id])?;
        
        if let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let encrypted_content: String = row.get(2)?;
            let created_at: String = row.get(3)?;
            let updated_at: String = row.get(4)?;
            
            let content = self.crypto.decrypt(&encrypted_content);
            let created_at = DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            
            // Get tags for this diary entry
            let tags = self.get_tags_for_diary(&id)?;
            
            Ok(DiaryEntry {
                id,
                title,
                content,
                created_at,
                updated_at,
                tags,
            })
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
        }
    }
    
    fn get_tags_for_diary(&self, diary_id: &str) -> SqliteResult<Vec<String>> {
        let conn = self.pool.get().expect("Failed to get database connection");
        
        let mut stmt = conn.prepare(
            "SELECT t.name FROM tags t
             JOIN diary_tags dt ON t.id = dt.tag_id
             WHERE dt.diary_id = ?1"
        )?;
        
        let tag_iter = stmt.query_map(params![diary_id], |row| {
            let name: String = row.get(0)?;
            Ok(name)
        })?;
        
        let mut tags = Vec::new();
        for tag_result in tag_iter {
            tags.push(tag_result?);
        }
        
        Ok(tags)
    }
    
    pub fn list_diaries(&self) -> SqliteResult<Vec<DiaryEntry>> {
        let conn = self.pool.get().expect("Failed to get database connection");
        
        let mut stmt = conn.prepare(
            "SELECT id, title, content, created_at, updated_at FROM diary_entries ORDER BY created_at DESC"
        )?;
        
        let diary_iter = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let encrypted_content: String = row.get(2)?;
            let created_at: String = row.get(3)?;
            let updated_at: String = row.get(4)?;
            
            let content = self.crypto.decrypt(&encrypted_content);
            let created_at = DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            
            Ok((id, title, content, created_at, updated_at))
        })?;
        
        let mut diaries = Vec::new();
        for diary_result in diary_iter {
            let (id, title, content, created_at, updated_at) = diary_result?;
            let tags = self.get_tags_for_diary(&id)?;
            
            diaries.push(DiaryEntry {
                id,
                title,
                content,
                created_at,
                updated_at,
                tags,
            });
        }
        
        Ok(diaries)
    }
    
    pub fn search_diaries_by_tag(&self, tag_name: &str) -> SqliteResult<Vec<DiaryEntry>> {
        let conn = self.pool.get().expect("Failed to get database connection");
        
        let mut stmt = conn.prepare(
            "SELECT e.id, e.title, e.content, e.created_at, e.updated_at
             FROM diary_entries e
             JOIN diary_tags dt ON e.id = dt.diary_id
             JOIN tags t ON dt.tag_id = t.id
             WHERE t.name = ?1
             ORDER BY e.created_at DESC"
        )?;
        
        let diary_iter = stmt.query_map(params![tag_name], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let encrypted_content: String = row.get(2)?;
            let created_at: String = row.get(3)?;
            let updated_at: String = row.get(4)?;
            
            let content = self.crypto.decrypt(&encrypted_content);
            let created_at = DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            
            Ok((id, title, content, created_at, updated_at))
        })?;
        
        let mut diaries = Vec::new();
        for diary_result in diary_iter {
            let (id, title, content, created_at, updated_at) = diary_result?;
            let tags = self.get_tags_for_diary(&id)?;
            
            diaries.push(DiaryEntry {
                id,
                title,
                content,
                created_at,
                updated_at,
                tags,
            });
        }
        
        Ok(diaries)
    }
    
    pub fn get_graph_data(&self) -> SqliteResult<GraphData> {
        let conn = self.pool.get().expect("Failed to get database connection");
        
        // Get all diary entries as nodes
        let mut diary_stmt = conn.prepare(
            "SELECT id, title, created_at FROM diary_entries"
        )?;
        
        let diary_iter = diary_stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let created_at: String = row.get(2)?;
            
            Ok((id, title, created_at))
        })?;
        
        let mut nodes = Vec::new();
        for diary_result in diary_iter {
            let (id, title, created_at) = diary_result?;
            
            let properties = serde_json::json!({
                "title": title,
                "created_at": created_at,
            });
            
            nodes.push(GraphNode {
                id: id.clone(),
                label: title,
                node_type: "diary".to_string(),
                properties,
            });
        }
        
        // Get all tags as nodes
        let mut tag_stmt = conn.prepare("SELECT id, name FROM tags")?;
        
        let tag_iter = tag_stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            
            Ok((id, name))
        })?;
        
        for tag_result in tag_iter {
            let (id, name) = tag_result?;
            
            let properties = serde_json::json!({
                "name": name,
            });
            
            nodes.push(GraphNode {
                id: id.clone(),
                label: name,
                node_type: "tag".to_string(),
                properties,
            });
        }
        
        // Get all relationships as edges
        let mut edge_stmt = conn.prepare(
            "SELECT dt.diary_id, dt.tag_id, t.name
             FROM diary_tags dt
             JOIN tags t ON dt.tag_id = t.id"
        )?;
        
        let edge_iter = edge_stmt.query_map([], |row| {
            let diary_id: String = row.get(0)?;
            let tag_id: String = row.get(1)?;
            let tag_name: String = row.get(2)?;
            
            Ok((diary_id, tag_id, tag_name))
        })?;
        
        let mut edges = Vec::new();
        for edge_result in edge_iter {
            let (diary_id, tag_id, tag_name) = edge_result?;
            
            edges.push(GraphEdge {
                id: format!("{}-{}", diary_id, tag_id),
                source: diary_id,
                target: tag_id,
                label: format!("tagged_as_{}", tag_name),
            });
        }
        
        Ok(GraphData { nodes, edges })
    }
} 