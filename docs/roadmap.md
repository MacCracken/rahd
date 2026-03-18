# Roadmap

## Phase 1 — Core Foundation (Complete)

- [x] Event CRUD (create, read, update, delete)
- [x] Contact management
- [x] Natural language event parsing (keyword-based)
- [x] Conflict detection
- [x] Free/busy time slot analysis
- [x] Meeting time suggestions
- [x] Priority scoring
- [x] SQLite local storage (JSON blobs + indexed timestamps)
- [x] CLI with clap subcommands (8 commands)
- [x] MCP tool definitions (5 tools)
- [x] CI/CD pipeline (check, test, clippy, fmt + multi-arch release)
- [x] Unit tests (49 tests across all crates)

## MVP — Usable Daily Driver (Complete)

The goal: a calendar you can actually rely on day-to-day from the terminal,
with ICS interop so it's not an island.

- [x] **Event update CLI** — `rahd edit <id> --title/--start/--end/--location`
- [x] **ICS import/export** — import `.ics` files, export events as ICS
- [x] **vCard import/export** — import/export contacts
- [x] **Recurring event expansion** — generate concrete instances from recurrence rules
- [x] **Reminder support** — store reminders on events, `rahd upcoming` for due-soon alerts
- [x] **MCP tool execution** — wire MCP tool defs to actual store/schedule operations (daimon-ready)
- [x] **Integration tests** — 8 end-to-end CLI tests (add → list → show → delete, edit, import/export)
- [x] **Documentation** — usage guide, CONTRIBUTING.md, inline rustdoc on public API

## Post-v1 — Sync, GUI & Intelligence

### Sync & Interop
- [ ] CalDAV client for bidirectional sync with external calendars
- [ ] Multiple calendar support with color coding
- [ ] Google Calendar / Outlook import bridges

### Desktop GUI
- [ ] egui desktop app with day/week/month views
- [ ] Reminder notifications via desktop notifications (notify-rust)
- [ ] Drag-and-drop event rescheduling

### AI Intelligence
- [ ] Smart scheduling via hoosh (LLM-assisted time suggestions)
- [ ] Meeting prep summaries (gather context from contacts + previous meetings)
- [ ] Contact enrichment from email signatures and context
- [ ] Travel time estimation between events with locations
- [ ] Habit tracking (detect patterns in recurring events)

### AGNOS Integration
- [x] daimon API server (port 8090) — HTTP wrapper for MCP tools
- [ ] hoosh API integration (port 8088) — LLM-powered scheduling
- [ ] agnoshi intents: "schedule a meeting", "when am I free", "show my week"
- [ ] Marketplace recipe and AGNOS app registration
