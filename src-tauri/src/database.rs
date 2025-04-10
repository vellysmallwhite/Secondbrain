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

#[derive(Debug, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub parent_id: String,
    pub child_id: String,
    pub relationship_type: String,
    pub created_at: String,
}

pub struct DiaryDB {
    pool: DbPool,
    crypto: Arc<Crypto>,
}

impl DiaryDB {
    pub fn new() -> Self {
        let db_path = Self::get_db_path();
        let manager = SqliteConnectionManager::file(db_path).with_init(|conn| {
            conn.execute_batch("PRAGMA foreign_keys = ON;")
        });
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
    
    pub fn initialize_db(&self) -> SqliteResult<()> {
        let conn = self.pool.get().expect("Failed to get database connection");
        
        // Enable foreign key constraints
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        
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
        
        // Create relationships table for connecting diary entries
        conn.execute(
            "CREATE TABLE IF NOT EXISTS relationships (
                id TEXT PRIMARY KEY,
                parent_id TEXT NOT NULL,
                child_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (parent_id) REFERENCES diary_entries (id) ON DELETE CASCADE,
                FOREIGN KEY (child_id) REFERENCES diary_entries (id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        Ok(())
    }
    
    pub fn save_diary(&self, id: Option<&str>, title: &str, content: &str, tags: &[String]) -> SqliteResult<String> {
        let conn = self.pool.get().expect("Failed to get database connection");
        let encrypted_content = self.crypto.encrypt(content);
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        
        let diary_id = match id {
            Some(existing_id) => {
                // Update existing diary
                conn.execute(
                    "UPDATE diary_entries SET title = ?1, content = ?2, updated_at = ?3 WHERE id = ?4",
                    params![title, encrypted_content, now_str, existing_id],
                )?;
                
                // Delete existing tag relationships
                conn.execute(
                    "DELETE FROM diary_tags WHERE diary_id = ?1",
                    params![existing_id],
                )?;
                
                existing_id.to_string()
            },
            None => {
                // Create new diary
                let new_id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO diary_entries (id, title, content, created_at, updated_at) 
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![new_id, title, encrypted_content, now_str, now_str],
                )?;
                new_id
            }
        };
        
        // Process tags
        for tag_name in tags {
            let tag_id = self.get_or_create_tag(&conn, tag_name)?;
            
            // Create relationship
            conn.execute(
                "INSERT OR IGNORE INTO diary_tags (diary_id, tag_id) VALUES (?1, ?2)",
                params![diary_id, tag_id],
            )?;
        }
        
        Ok(diary_id)
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
        let mut edges = Vec::new();
        
        // Tag relationships
        let mut tag_edge_stmt = conn.prepare(
            "SELECT dt.diary_id, dt.tag_id, t.name
             FROM diary_tags dt
             JOIN tags t ON dt.tag_id = t.id"
        )?;
        
        let tag_edge_iter = tag_edge_stmt.query_map([], |row| {
            let diary_id: String = row.get(0)?;
            let tag_id: String = row.get(1)?;
            let tag_name: String = row.get(2)?;
            
            Ok((diary_id, tag_id, tag_name))
        })?;
        
        for edge_result in tag_edge_iter {
            let (diary_id, tag_id, tag_name) = edge_result?;
            
            edges.push(GraphEdge {
                id: format!("tag-{}-{}", diary_id, tag_id),
                source: diary_id,
                target: tag_id,
                label: format!("tagged_as_{}", tag_name),
            });
        }
        
        // Diary entry relationships
        let mut rel_edge_stmt = conn.prepare(
            "SELECT id, parent_id, child_id, relationship_type
             FROM relationships"
        )?;
        
        let rel_edge_iter = rel_edge_stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let parent_id: String = row.get(1)?;
            let child_id: String = row.get(2)?;
            let relationship_type: String = row.get(3)?;
            
            Ok((id, parent_id, child_id, relationship_type))
        })?;
        
        for edge_result in rel_edge_iter {
            let (id, parent_id, child_id, relationship_type) = edge_result?;
            
            edges.push(GraphEdge {
                id,
                source: child_id,     // Child is the source of the edge
                target: parent_id,    // Parent is the target
                label: relationship_type,
            });
        }
        
        Ok(GraphData { nodes, edges })
    }

    pub fn delete_diary(&self, id: &str) -> SqliteResult<()> {
        println!("📝 [DELETE_DIARY] Starting deletion for diary ID: {}", id);
        
        // Get a connection from the pool
        let conn = self.pool.get().expect("Failed to get database connection");
        
        // Check foreign keys status
        let foreign_keys_enabled: i32 = conn.query_row(
            "PRAGMA foreign_keys",
            [],
            |row| row.get(0)
        )?;
        println!("📝 [DELETE_DIARY] Foreign keys enabled: {}", foreign_keys_enabled);
        
        // Check for existing relationships
        let rel_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM relationships WHERE parent_id = ?1 OR child_id = ?1",
            params![id],
            |row| row.get(0)
        )?;
        println!("📝 [DELETE_DIARY] Found {} relationships for this diary", rel_count);
        
        // Check for existing tags
        let tags_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM diary_tags WHERE diary_id = ?1",
            params![id],
            |row| row.get(0)
        )?;
        println!("📝 [DELETE_DIARY] Found {} tag connections for this diary", tags_count);
        
        // First, manually delete any relationships
        println!("📝 [DELETE_DIARY] Step 1: Manually deleting relationships");
        let deleted_rels = conn.execute(
            "DELETE FROM relationships WHERE parent_id = ?1 OR child_id = ?1",
            params![id]
        )?;
        println!("📝 [DELETE_DIARY] Deleted {} relationships", deleted_rels);
        
        // Second, manually delete tag connections
        println!("📝 [DELETE_DIARY] Step 2: Manually deleting tag connections");
        let deleted_tags = conn.execute(
            "DELETE FROM diary_tags WHERE diary_id = ?1",
            params![id]
        )?;
        println!("📝 [DELETE_DIARY] Deleted {} tag connections", deleted_tags);
        
        // Finally, delete the diary entry
        println!("📝 [DELETE_DIARY] Step 3: Deleting the diary entry");
        let deleted_diary = conn.execute(
            "DELETE FROM diary_entries WHERE id = ?1",
            params![id]
        )?;
        println!("📝 [DELETE_DIARY] Deleted {} diary entries", deleted_diary);
        
        if deleted_diary == 0 {
            println!("⚠️ [DELETE_DIARY] Warning: No diary entries were deleted!");
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        
        // Verify all relationships were deleted
        let remaining_rels: i32 = conn.query_row(
            "SELECT COUNT(*) FROM relationships WHERE parent_id = ?1 OR child_id = ?1",
            params![id],
            |row| row.get(0)
        )?;
        println!("📝 [DELETE_DIARY] Remaining relationships: {}", remaining_rels);
        
        if remaining_rels > 0 {
            println!("⚠️ [DELETE_DIARY] Warning: Some relationships remained after deletion!");
        }
        
        // Verify all tag connections were deleted
        let remaining_tags: i32 = conn.query_row(
            "SELECT COUNT(*) FROM diary_tags WHERE diary_id = ?1",
            params![id],
            |row| row.get(0)
        )?;
        println!("📝 [DELETE_DIARY] Remaining tag connections: {}", remaining_tags);
        
        if remaining_tags > 0 {
            println!("⚠️ [DELETE_DIARY] Warning: Some tag connections remained after deletion!");
        }
        
        println!("📝 [DELETE_DIARY] Deletion process completed successfully");
        Ok(())
    }

    pub fn add_relationship(&self, id: &str, parent_id: &str, child_id: &str, relationship_type: &str) -> SqliteResult<String> {
        let conn = self.pool.get().expect("Failed to get database connection");
        let now = Utc::now().to_rfc3339();
        
        conn.execute(
            "INSERT INTO relationships (id, parent_id, child_id, relationship_type, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, parent_id, child_id, relationship_type, now],
        )?;
        
        Ok(id.to_string())
    }
    
    pub fn delete_relationship(&self, id: &str) -> SqliteResult<()> {
        let conn = self.pool.get().expect("Failed to get database connection");
        
        conn.execute(
            "DELETE FROM relationships WHERE id = ?1",
            params![id],
        )?;
        
        Ok(())
    }
    
    pub fn get_relationships(&self, diary_id: &str) -> SqliteResult<Vec<Relationship>> {
        let conn = self.pool.get().expect("Failed to get database connection");
        
        let mut stmt = conn.prepare(
            "SELECT id, parent_id, child_id, relationship_type, created_at 
             FROM relationships 
             WHERE parent_id = ?1 OR child_id = ?1"
        )?;
        
        let relationship_iter = stmt.query_map(params![diary_id, diary_id], |row| {
            let id: String = row.get(0)?;
            let parent_id: String = row.get(1)?;
            let child_id: String = row.get(2)?;
            let relationship_type: String = row.get(3)?;
            let created_at_str: String = row.get(4)?;
            
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            
            Ok(Relationship {
                id,
                parent_id,
                child_id,
                relationship_type,
                created_at: created_at.to_rfc3339(),
            })
        })?;
        
        let mut relationships = Vec::new();
        for relationship_result in relationship_iter {
            relationships.push(relationship_result?);
        }
        
        Ok(relationships)
    }
} 