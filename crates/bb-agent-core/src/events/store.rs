use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Result as SqliteResult, params};

use super::AgentEvent;
use bb_common::enums::{EventCategory, EventSeverity, EventType};

/// SQLite-backed persistent event store.
pub struct EventStore {
    conn: Connection,
}

impl EventStore {
    /// Open or create the SQLite database at the given path.
    pub fn new(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                category TEXT NOT NULL,
                severity TEXT NOT NULL,
                domain TEXT,
                plugin_id TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                timestamp TEXT NOT NULL,
                reported INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_events_reported ON events(reported);
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);",
        )?;
        Ok(Self { conn })
    }

    /// Create an in-memory store (for testing).
    pub fn in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                category TEXT NOT NULL,
                severity TEXT NOT NULL,
                domain TEXT,
                plugin_id TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                timestamp TEXT NOT NULL,
                reported INTEGER NOT NULL DEFAULT 0
            );",
        )?;
        Ok(Self { conn })
    }

    /// Insert an event and return its row ID.
    pub fn insert(&self, event: &AgentEvent) -> Result<i64, rusqlite::Error> {
        let event_type =
            serde_json::to_string(&event.event_type).unwrap_or_else(|_| "\"unknown\"".to_string());
        let category =
            serde_json::to_string(&event.category).unwrap_or_else(|_| "\"unknown\"".to_string());
        let severity =
            serde_json::to_string(&event.severity).unwrap_or_else(|_| "\"unknown\"".to_string());
        let metadata = event.metadata.to_string();
        let timestamp = event.timestamp.to_rfc3339();

        self.conn.execute(
            "INSERT INTO events (event_type, category, severity, domain, plugin_id, metadata, timestamp, reported)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event_type,
                category,
                severity,
                event.domain,
                event.plugin_id,
                metadata,
                timestamp,
                i32::from(event.reported),
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Fetch unreported events, ordered by timestamp, limited to `limit` rows.
    #[allow(clippy::cast_possible_wrap)]
    pub fn unreported(&self, limit: usize) -> Result<Vec<AgentEvent>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_type, category, severity, domain, plugin_id, metadata, timestamp, reported
             FROM events
             WHERE reported = 0
             ORDER BY timestamp ASC
             LIMIT ?1",
        )?;

        let events = stmt
            .query_map(params![limit as i64], |row| {
                let id: i64 = row.get(0)?;
                let event_type_str: String = row.get(1)?;
                let category_str: String = row.get(2)?;
                let severity_str: String = row.get(3)?;
                let domain: Option<String> = row.get(4)?;
                let plugin_id: String = row.get(5)?;
                let metadata_str: String = row.get(6)?;
                let timestamp_str: String = row.get(7)?;
                let reported: i32 = row.get(8)?;

                Ok(AgentEvent {
                    id: Some(id),
                    event_type: serde_json::from_str(&event_type_str)
                        .unwrap_or(EventType::Heartbeat),
                    category: serde_json::from_str(&category_str).unwrap_or(EventCategory::System),
                    severity: serde_json::from_str(&severity_str).unwrap_or(EventSeverity::Info),
                    domain,
                    plugin_id,
                    metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
                    timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                        .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc)),
                    reported: reported != 0,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(events)
    }

    /// Mark the given event IDs as reported.
    pub fn mark_reported(&self, ids: &[i64]) -> Result<(), rusqlite::Error> {
        if ids.is_empty() {
            return Ok(());
        }

        // Use a transaction for batch updates
        let tx = self.conn.unchecked_transaction()?;
        for &id in ids {
            tx.execute("UPDATE events SET reported = 1 WHERE id = ?1", params![id])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Delete reported events older than `days` days. Returns number of deleted rows.
    pub fn prune_older_than(&self, days: u32) -> Result<usize, rusqlite::Error> {
        let cutoff = Utc::now() - chrono::Duration::days(i64::from(days));
        let cutoff_str = cutoff.to_rfc3339();

        let deleted = self.conn.execute(
            "DELETE FROM events WHERE reported = 1 AND timestamp < ?1",
            params![cutoff_str],
        )?;

        Ok(deleted)
    }

    /// Count total events in the store.
    pub fn count(&self) -> Result<i64, rusqlite::Error> {
        self.conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::AgentEvent;

    #[test]
    fn test_insert_and_unreported() {
        let store = EventStore::in_memory().expect("create store");

        let event = AgentEvent::dns_block("bet365.com", "dns.resolver");
        let id = store.insert(&event).expect("insert");
        assert!(id > 0);

        let unreported = store.unreported(100).expect("unreported");
        assert_eq!(unreported.len(), 1);
        assert_eq!(unreported[0].domain, Some("bet365.com".to_string()));
        assert_eq!(unreported[0].plugin_id, "dns.resolver");
    }

    #[test]
    fn test_mark_reported() {
        let store = EventStore::in_memory().expect("create store");

        let id1 = store
            .insert(&AgentEvent::dns_block("a.com", "dns.resolver"))
            .expect("insert");
        let _id2 = store
            .insert(&AgentEvent::dns_block("b.com", "dns.resolver"))
            .expect("insert");

        // Both unreported
        assert_eq!(store.unreported(100).expect("unreported").len(), 2);

        // Mark first as reported
        store.mark_reported(&[id1]).expect("mark");
        let unreported = store.unreported(100).expect("unreported");
        assert_eq!(unreported.len(), 1);
        assert_eq!(unreported[0].domain, Some("b.com".to_string()));
    }

    #[test]
    fn test_unreported_limit() {
        let store = EventStore::in_memory().expect("create store");

        for i in 0..10 {
            store
                .insert(&AgentEvent::dns_block(
                    &format!("site{i}.com"),
                    "dns.resolver",
                ))
                .expect("insert");
        }

        let batch = store.unreported(3).expect("unreported");
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn test_file_persistence() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("events.db");

        // Insert
        {
            let store = EventStore::new(&db_path).expect("create");
            store
                .insert(&AgentEvent::dns_block("test.com", "dns.resolver"))
                .expect("insert");
        }

        // Re-open and verify
        {
            let store = EventStore::new(&db_path).expect("reopen");
            let unreported = store.unreported(100).expect("unreported");
            assert_eq!(unreported.len(), 1);
            assert_eq!(unreported[0].domain, Some("test.com".to_string()));
        }
    }

    #[test]
    fn test_count() {
        let store = EventStore::in_memory().expect("create store");
        assert_eq!(store.count().expect("count"), 0);

        store.insert(&AgentEvent::heartbeat()).expect("insert");
        assert_eq!(store.count().expect("count"), 1);
    }
}
