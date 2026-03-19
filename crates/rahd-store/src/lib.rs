//! Rahd Store — SQLite-backed storage for events and contacts

use anyhow::Result;

use rusqlite::{Connection, params};
use uuid::Uuid;

use rahd_core::{Contact, Event, EventFilter};

/// Local event and contact store backed by SQLite.
pub struct EventStore {
    conn: Connection,
}

impl EventStore {
    /// Open or create a store at the given path.
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.run_migrations()?;
        Ok(store)
    }

    /// Create an in-memory store (for testing).
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.run_migrations()?;
        Ok(store)
    }

    fn run_migrations(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode=WAL;
            PRAGMA busy_timeout=5000;
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                start_ts TEXT NOT NULL,
                end_ts TEXT NOT NULL,
                calendar_id TEXT NOT NULL DEFAULT 'default'
            );
            CREATE TABLE IF NOT EXISTS contacts (
                id TEXT PRIMARY KEY,
                data TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS calendars (
                id TEXT PRIMARY KEY,
                data TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    /// Add an event to the store.
    pub fn add_event(&self, event: &Event) -> Result<()> {
        let data = serde_json::to_string(event)?;
        self.conn.execute(
            "INSERT INTO events (id, data, start_ts, end_ts, calendar_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                event.id.to_string(),
                data,
                event.start.to_rfc3339(),
                event.end.to_rfc3339(),
                event.calendar_id,
            ],
        )?;
        Ok(())
    }

    /// Get an event by ID.
    pub fn get_event(&self, id: Uuid) -> Result<Option<Event>> {
        let mut stmt = self.conn.prepare("SELECT data FROM events WHERE id = ?1")?;
        let mut rows = stmt.query(params![id.to_string()])?;
        match rows.next()? {
            Some(row) => {
                let data: String = row.get(0)?;
                let event: Event = serde_json::from_str(&data)?;
                Ok(Some(event))
            }
            None => Ok(None),
        }
    }

    /// List events matching a filter.
    pub fn list_events(&self, filter: &EventFilter) -> Result<Vec<Event>> {
        let mut sql = "SELECT data FROM events WHERE 1=1".to_string();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

        if let Some(from) = &filter.from {
            sql.push_str(" AND end_ts >= ?");
            params_vec.push(Box::new(from.to_rfc3339()));
        }
        if let Some(to) = &filter.to {
            sql.push_str(" AND start_ts <= ?");
            params_vec.push(Box::new(to.to_rfc3339()));
        }
        if let Some(cal) = &filter.calendar_id {
            sql.push_str(" AND calendar_id = ?");
            params_vec.push(Box::new(cal.clone()));
        }

        sql.push_str(" ORDER BY start_ts ASC");

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let data: String = row.get(0)?;
            Ok(data)
        })?;

        let mut events = Vec::new();
        for row in rows {
            let data = row?;
            let event: Event = serde_json::from_str(&data)?;
            if let Some(search) = &filter.search {
                let lower = search.to_lowercase();
                let title_match = event.title.to_lowercase().contains(&lower);
                let desc_match = event
                    .description
                    .as_ref()
                    .is_some_and(|d| d.to_lowercase().contains(&lower));
                if !title_match && !desc_match {
                    continue;
                }
            }
            events.push(event);
        }
        Ok(events)
    }

    /// Update an existing event. Returns true if it existed.
    pub fn update_event(&self, event: &Event) -> Result<bool> {
        let data = serde_json::to_string(event)?;
        let rows = self.conn.execute(
            "UPDATE events SET data = ?1, start_ts = ?2, end_ts = ?3, calendar_id = ?4 WHERE id = ?5",
            params![
                data,
                event.start.to_rfc3339(),
                event.end.to_rfc3339(),
                event.calendar_id,
                event.id.to_string(),
            ],
        )?;
        Ok(rows > 0)
    }

    /// Delete an event. Returns true if it existed.
    pub fn delete_event(&self, id: Uuid) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM events WHERE id = ?1", params![id.to_string()])?;
        Ok(rows > 0)
    }

    /// Add a contact.
    pub fn add_contact(&self, contact: &Contact) -> Result<()> {
        let data = serde_json::to_string(contact)?;
        self.conn.execute(
            "INSERT INTO contacts (id, data) VALUES (?1, ?2)",
            params![contact.id.to_string(), data],
        )?;
        Ok(())
    }

    /// List all contacts.
    pub fn list_contacts(&self) -> Result<Vec<Contact>> {
        let mut stmt = self.conn.prepare("SELECT data FROM contacts ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            let data: String = row.get(0)?;
            Ok(data)
        })?;
        let mut contacts = Vec::new();
        for row in rows {
            let data = row?;
            contacts.push(serde_json::from_str(&data)?);
        }
        Ok(contacts)
    }

    /// Delete a contact. Returns true if it existed.
    pub fn delete_contact(&self, id: Uuid) -> Result<bool> {
        let rows = self.conn.execute(
            "DELETE FROM contacts WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(rows > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rahd_core::EventFilter;

    fn make_event(title: &str, start_hour: u32, end_hour: u32) -> Event {
        let now = Utc::now();
        Event {
            id: Uuid::new_v4(),
            title: title.to_string(),
            description: None,
            start: Utc.with_ymd_and_hms(2026, 3, 16, start_hour, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 3, 16, end_hour, 0, 0).unwrap(),
            location: None,
            attendees: vec![],
            recurrence: None,
            reminders: vec![],
            calendar_id: "default".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn make_contact(name: &str) -> Contact {
        Contact {
            id: Uuid::new_v4(),
            name: name.to_string(),
            email: Some(format!("{name}@example.com")),
            phone: None,
            organization: None,
            notes: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn create_in_memory() {
        let store = EventStore::new_in_memory().unwrap();
        let events = store.list_events(&EventFilter::default()).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn add_and_get_event() {
        let store = EventStore::new_in_memory().unwrap();
        let event = make_event("Test", 10, 11);
        store.add_event(&event).unwrap();
        let found = store.get_event(event.id).unwrap().unwrap();
        assert_eq!(found.title, "Test");
    }

    #[test]
    fn list_events_all() {
        let store = EventStore::new_in_memory().unwrap();
        store.add_event(&make_event("A", 9, 10)).unwrap();
        store.add_event(&make_event("B", 11, 12)).unwrap();
        let events = store.list_events(&EventFilter::default()).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn list_events_with_time_filter() {
        let store = EventStore::new_in_memory().unwrap();
        store.add_event(&make_event("Morning", 8, 9)).unwrap();
        store.add_event(&make_event("Afternoon", 14, 15)).unwrap();
        let filter = EventFilter {
            from: Some(Utc.with_ymd_and_hms(2026, 3, 16, 12, 0, 0).unwrap()),
            ..Default::default()
        };
        let events = store.list_events(&filter).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Afternoon");
    }

    #[test]
    fn update_event() {
        let store = EventStore::new_in_memory().unwrap();
        let mut event = make_event("Original", 10, 11);
        store.add_event(&event).unwrap();
        event.title = "Updated".to_string();
        store.update_event(&event).unwrap();
        let found = store.get_event(event.id).unwrap().unwrap();
        assert_eq!(found.title, "Updated");
    }

    #[test]
    fn delete_event() {
        let store = EventStore::new_in_memory().unwrap();
        let event = make_event("Delete Me", 10, 11);
        store.add_event(&event).unwrap();
        assert!(store.delete_event(event.id).unwrap());
        assert!(store.get_event(event.id).unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_event() {
        let store = EventStore::new_in_memory().unwrap();
        assert!(!store.delete_event(Uuid::new_v4()).unwrap());
    }

    #[test]
    fn add_and_list_contacts() {
        let store = EventStore::new_in_memory().unwrap();
        store.add_contact(&make_contact("Alice")).unwrap();
        store.add_contact(&make_contact("Bob")).unwrap();
        let contacts = store.list_contacts().unwrap();
        assert_eq!(contacts.len(), 2);
    }

    #[test]
    fn delete_contact() {
        let store = EventStore::new_in_memory().unwrap();
        let contact = make_contact("Charlie");
        store.add_contact(&contact).unwrap();
        assert!(store.delete_contact(contact.id).unwrap());
        let contacts = store.list_contacts().unwrap();
        assert!(contacts.is_empty());
    }

    #[test]
    fn update_nonexistent_event_returns_false() {
        let store = EventStore::new_in_memory().unwrap();
        let event = make_event("Ghost", 10, 11);
        assert!(!store.update_event(&event).unwrap());
    }

    #[test]
    fn update_existing_event_returns_true() {
        let store = EventStore::new_in_memory().unwrap();
        let mut event = make_event("Original", 10, 11);
        store.add_event(&event).unwrap();
        event.title = "Updated".to_string();
        assert!(store.update_event(&event).unwrap());
    }

    #[test]
    fn list_events_search_description() {
        let store = EventStore::new_in_memory().unwrap();
        let mut event = make_event("Meeting", 9, 10);
        event.description = Some("Discuss the project roadmap".to_string());
        store.add_event(&event).unwrap();
        store.add_event(&make_event("Lunch", 12, 13)).unwrap();
        let filter = EventFilter {
            search: Some("roadmap".to_string()),
            ..Default::default()
        };
        let events = store.list_events(&filter).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Meeting");
    }

    #[test]
    fn list_events_with_search_filter() {
        let store = EventStore::new_in_memory().unwrap();
        store.add_event(&make_event("Team Standup", 9, 10)).unwrap();
        store.add_event(&make_event("Lunch", 12, 13)).unwrap();
        let filter = EventFilter {
            search: Some("standup".to_string()),
            ..Default::default()
        };
        let events = store.list_events(&filter).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Team Standup");
    }
}
