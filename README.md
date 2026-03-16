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
- **ICS/vCard import/export** — interop with standard calendar and contact formats
- **Recurring events** — daily, weekly, monthly, yearly recurrence expansion
- **Reminders** — per-event reminders with `rahd upcoming` alerts
- **MCP tools** — 5 native tools with execution for agent-driven calendar operations
- **CLI interface** — fast terminal workflow via `rahd` command

## Architecture

```
rahd
├── rahd-core       — Event, Contact, Calendar types, ICS/vCard, recurrence expansion
├── rahd-store      — SQLite storage, CRUD operations
├── rahd-schedule   — Conflict detection, free/busy, time slot suggestions
├── rahd-ai         — NL event parsing, smart scheduling, priority scoring
└── rahd-mcp        — MCP tool definitions + execution for AGNOS integration
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
rahd add "dentist at 10am" --remind 15

# Edit event
rahd edit <event-id> --title "New title" --location "Room 42"

# List events
rahd list --today
rahd list --week
rahd list --month

# Show event details / delete
rahd show <event-id>
rahd delete <event-id>

# Upcoming events with reminder alerts
rahd upcoming
rahd upcoming --within 120

# Contacts
rahd contacts list
rahd contacts add "Sam Wilson" --email sam@example.com --phone 555-1234
rahd contacts delete <contact-id>

# Free time slots
rahd free
rahd free --date 2026-03-20

# Scheduling conflicts
rahd conflicts

# Import/Export
rahd import calendar.ics
rahd import contacts.vcf
rahd export events -o calendar.ics
rahd export contacts -o contacts.vcf
```

See [docs/usage.md](docs/usage.md) for the full usage guide.

## AGNOS Integration

Rahd integrates with AGNOS through:

- **MCP tools** — `rahd_events`, `rahd_add`, `rahd_free`, `rahd_conflicts`, `rahd_contacts` (with execution)
- **daimon API** (port 8090) — event queries, scheduling from agents (planned)
- **hoosh API** (port 8088) — LLM-assisted smart scheduling (planned)
- **agnoshi intents** — "schedule a meeting", "show my calendar", "find free time" (planned)

## Build

```bash
cargo build --workspace
cargo test --workspace
```

## Docs

- [Usage Guide](docs/usage.md)
- [Architecture](docs/architecture.md)
- [MCP Tools Reference](docs/mcp-tools.md)
- [Roadmap](docs/roadmap.md)
- [Contributing](CONTRIBUTING.md)

## License

GPL-3.0
