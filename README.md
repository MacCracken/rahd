# Rahd — AI-Native Calendar & Contacts

> Ruznam Ahd (Persian: روزنامه daily record + Arabic: عهد appointment)

[![License](https://img.shields.io/badge/license-GPLv3-blue)](LICENSE)
[![Status](https://img.shields.io/badge/status-development-yellow)]()

**Rahd** is an AI-powered calendar and contacts app for [AGNOS](https://github.com/MacCracken/agnosticos). It provides local-first event management with natural language event creation, smart scheduling, conflict detection, and contacts management.

## Features

- **Natural language events** — "lunch with Sam tomorrow at noon" just works
- **Smart scheduling** — find free slots, suggest meeting times, detect conflicts
- **Contact management** — store contacts with email, phone, organization
- **Local-first storage** — SQLite database, no cloud dependency
- **Conflict detection** — automatic overlap detection across all calendars
- **Priority scoring** — AI-based urgency ranking (sooner, social, recurring)
- **MCP tools** — 5 native tools for agent-driven calendar queries
- **CLI interface** — fast terminal workflow via `rahd` command

## Architecture

```
rahd
├── rahd-core       — Event, Contact, Calendar types, recurrence, time zones
├── rahd-store      — SQLite storage, import/export (ICS/vCard planned)
├── rahd-schedule   — Conflict detection, free/busy, time slot suggestions
├── rahd-ai         — NL event parsing, smart scheduling, priority scoring
└── rahd-mcp        — MCP tool definitions for AGNOS integration
```

### Data Flow

```
User input ("lunch with Sam tomorrow at noon")
    → rahd-ai (NL parsing)
    → rahd-core (Event struct)
    → rahd-store (SQLite persistence)
    → rahd-schedule (conflict check)
    → CLI output
```

## Usage

```bash
# Add event using natural language
rahd add "lunch with Sam tomorrow at noon"
rahd add "meeting with Bob on Friday at 3pm for 1 hour"
rahd add "dentist appointment March 20 at 10am"

# List events
rahd list --today
rahd list --week
rahd list --month

# Show event details
rahd show <event-id>

# Delete event
rahd delete <event-id>

# Contacts
rahd contacts list
rahd contacts add "Sam Wilson" --email sam@example.com --phone 555-1234

# Free time slots
rahd free
rahd free --date 2026-03-20

# Scheduling conflicts
rahd conflicts
```

## AGNOS Integration

Rahd integrates with AGNOS through:

- **daimon API** (port 8090) — event queries, scheduling from agents
- **hoosh API** (port 8088) — LLM-assisted smart scheduling (planned)
- **MCP tools** — `rahd_events`, `rahd_add`, `rahd_free`, `rahd_conflicts`, `rahd_contacts`
- **agnoshi intents** — "schedule a meeting", "show my calendar", "find free time" (planned)

## Build

```bash
cargo build --workspace
cargo test --workspace
```

## License

GPL-3.0
