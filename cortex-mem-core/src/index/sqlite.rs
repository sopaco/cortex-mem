use crate::Result;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// SQLite-based index for metadata and time-series queries
pub struct SQLiteIndex {
    conn: rusqlite::Connection,
}

/// Index entry for a memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub uri: String,
    pub dimension: String,
    pub id: String,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub size: u64,
    pub checksum: String,
}

/// Time-series query options
#[derive(Debug, Clone)]
pub struct TimeSeriesQuery {
    pub dimension: Option<String>,
    pub id: Option<String>,
    pub category: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

impl SQLiteIndex {
    /// Create a new SQLite index
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = rusqlite::Connection::open(db_path)?;
        let index = Self { conn };
        index.initialize()?;
        Ok(index)
    }

    /// Create in-memory index for testing
    pub fn in_memory() -> Result<Self> {
        let conn = rusqlite::Connection::open_in_memory()?;
        let index = Self { conn };
        index.initialize()?;
        Ok(index)
    }

    /// Initialize database schema
    fn initialize(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                uri TEXT PRIMARY KEY,
                dimension TEXT NOT NULL,
                id TEXT NOT NULL,
                category TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                size INTEGER NOT NULL,
                checksum TEXT NOT NULL
            )",
            [],
        )?;

        // Indexes for fast queries
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_dimension ON memories(dimension)",
            [],
        )?;

        self.conn
            .execute("CREATE INDEX IF NOT EXISTS idx_id ON memories(id)", [])?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_category ON memories(category)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_created_at ON memories(created_at)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_updated_at ON memories(updated_at)",
            [],
        )?;

        Ok(())
    }

    /// Insert or update an index entry
    pub fn upsert(&self, entry: &IndexEntry) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO memories
             (uri, dimension, id, category, created_at, updated_at, size, checksum)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                entry.uri,
                entry.dimension,
                entry.id,
                entry.category,
                entry.created_at.to_rfc3339(),
                entry.updated_at.to_rfc3339(),
                entry.size,
                entry.checksum,
            ],
        )?;
        Ok(())
    }

    /// Delete an index entry
    pub fn delete(&self, uri: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM memories WHERE uri = ?1", [uri])?;
        Ok(())
    }

    /// Get entry by URI
    pub fn get(&self, uri: &str) -> Result<Option<IndexEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT uri, dimension, id, category, created_at, updated_at, size, checksum
             FROM memories WHERE uri = ?1",
        )?;

        let entry = stmt
            .query_row([uri], |row| {
                Ok(IndexEntry {
                    uri: row.get(0)?,
                    dimension: row.get(1)?,
                    id: row.get(2)?,
                    category: row.get(3)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Utc),
                    size: row.get(6)?,
                    checksum: row.get(7)?,
                })
            })
            .optional()?;

        Ok(entry)
    }

    /// Query entries by time range and filters
    pub fn query(&self, query: &TimeSeriesQuery) -> Result<Vec<IndexEntry>> {
        let mut sql = String::from("SELECT uri, dimension, id, category, created_at, updated_at, size, checksum FROM memories WHERE 1=1");
        let mut params = Vec::new();

        if let Some(ref dimension) = query.dimension {
            sql.push_str(" AND dimension = ?");
            params.push(dimension.clone());
        }

        if let Some(ref id) = query.id {
            sql.push_str(" AND id = ?");
            params.push(id.clone());
        }

        if let Some(ref category) = query.category {
            sql.push_str(" AND category = ?");
            params.push(category.clone());
        }

        if let Some(ref start_time) = query.start_time {
            sql.push_str(" AND created_at >= ?");
            params.push(start_time.to_rfc3339());
        }

        if let Some(ref end_time) = query.end_time {
            sql.push_str(" AND created_at <= ?");
            params.push(end_time.to_rfc3339());
        }

        sql.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();

        let entries = stmt
            .query_map(&param_refs[..], |row| {
                Ok(IndexEntry {
                    uri: row.get(0)?,
                    dimension: row.get(1)?,
                    id: row.get(2)?,
                    category: row.get(3)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Utc),
                    size: row.get(6)?,
                    checksum: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Count total entries
    pub fn count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get entries by dimension
    pub fn list_by_dimension(&self, dimension: &str) -> Result<Vec<IndexEntry>> {
        self.query(&TimeSeriesQuery {
            dimension: Some(dimension.to_string()),
            id: None,
            category: None,
            start_time: None,
            end_time: None,
            limit: None,
        })
    }

    /// Get entries by agent/user/thread ID
    pub fn list_by_id(&self, dimension: &str, id: &str) -> Result<Vec<IndexEntry>> {
        self.query(&TimeSeriesQuery {
            dimension: Some(dimension.to_string()),
            id: Some(id.to_string()),
            category: None,
            start_time: None,
            end_time: None,
            limit: None,
        })
    }
}
