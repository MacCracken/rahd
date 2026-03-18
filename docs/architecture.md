# Architecture

## Crate Structure

```
rahd/
├── Cargo.toml              # Workspace root + binary
├── src/main.rs             # CLI entry point (clap derive)
├── crates/
│   ├── rahd-core/          # Core types
│   │   └── src/lib.rs      # Event, Contact, Calendar, Recurrence, TimeSlot, Conflict, EventFilter
│   ├── rahd-store/         # Persistence layer
│   │   └── src/lib.rs      # EventStore (SQLite via rusqlite), CRUD operations
│   ├── rahd-schedule/      # Scheduling engine
│   │   └── src/lib.rs      # Scheduler: conflict detection, free slots, meeting suggestions
│   ├── rahd-ai/            # AI/NL processing
│   │   └── src/lib.rs      # NlEventParser, ParsedEvent, PriorityScorer
│   ├── rahd-mcp/           # AGNOS integration
│   │   └── src/lib.rs      # 5 MCP tool definitions + execution
│   ├── rahd-server/        # daimon API server
│   │   └── src/lib.rs      # axum HTTP server on port 8090
│   └── rahd-gui/           # Desktop GUI
│       └── src/
│           ├── lib.rs      # run() entry point (eframe)
│           ├── app.rs      # RahdApp state + eframe::App
│           ├── theme.rs    # AGNOS dark theme palette
│           └── views/      # day, week, month, sidebar, toolbar
```

## Data Flow

```
User CLI input
    │
    ▼
NlEventParser (rahd-ai)
    │  Parses "lunch with Sam tomorrow at noon"
    │  into ParsedEvent { title, date, time, attendees, ... }
    │
    ▼
Event (rahd-core)
    │  Structured event with UUID, DateTime, attendees
    │
    ▼
EventStore (rahd-store)
    │  SQLite: INSERT/SELECT/UPDATE/DELETE
    │  Events stored as JSON blobs with indexed timestamps
    │
    ▼
Scheduler (rahd-schedule)
    │  Conflict detection, free slot analysis
    │
    ▼
CLI output / MCP response / HTTP JSON (rahd-server)
```

## Storage

- SQLite database at `~/.local/share/rahd/rahd.db`
- Events stored as JSON in a `data` column with indexed `start_ts`/`end_ts` columns
- Contacts stored as JSON in a `data` column
- Schema migrations run on every open (CREATE TABLE IF NOT EXISTS)

## Dependencies

- **rahd-core**: serde, chrono, uuid, thiserror (no runtime deps)
- **rahd-store**: rahd-core + rusqlite (bundled SQLite)
- **rahd-schedule**: rahd-core + chrono
- **rahd-ai**: rahd-core + chrono (no external AI deps yet — local parsing only)
- **rahd-mcp**: rahd-core, rahd-store, rahd-schedule, rahd-ai + serde, serde_json
- **rahd-server**: rahd-mcp, rahd-store + axum, tokio
- **rahd-gui**: rahd-core, rahd-store, rahd-schedule + egui, eframe, chrono
