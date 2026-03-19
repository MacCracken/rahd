# Changelog

All notable changes to Rahd will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [2026.3.18-1] - 2026-03-18

### Added

- **rahd-gui**: Desktop reminder notifications via `notify-rust`
  - Fires desktop notifications when event reminders are due
  - Checks upcoming 24h of events every 30 seconds, independent of current view
  - Tracks fired reminders to avoid duplicates
- **rahd-gui**: Drag-and-drop event rescheduling in day and week views
  - Drag events to a new time slot to reschedule
  - Visual drop-target highlighting during drag
  - Preserves event duration on reschedule
- **rahd-server**: hoosh API client for LLM-powered scheduling (port 8088)
  - `HooshClient` with health check and scheduling intent API
  - `GET /hoosh/health` — check hoosh connectivity
  - `POST /hoosh/intent` — send natural language scheduling intents for LLM processing
  - Auto-populates event context for LLM reasoning
  - Auto-executes `create_event` actions from LLM responses
  - `IntentAction` enum for known agnoshi actions (create, reschedule, show, find free, cancel)
- **rahd-server**: Agnoshi intent resolver — deterministic local routing for common scheduling queries
  - `POST /intents/resolve` — resolve intents locally without LLM
  - Handles "schedule a meeting", "when am I free", "show my week/day/calendar" patterns
  - `hoosh_intent` now tries local resolution first, falls back to hoosh LLM for ambiguous queries
  - 7 intent resolver unit tests + 4 server endpoint tests
- **22 new tests**: intent resolution, ICS line folding, description search, vCard escaping,
  past-event scoring, invalid work hours, update_event bool, monthly recurrence day=0

### Fixed

- **rahd-gui**: Mini-calendar day click was non-functional (missing `Sense::click()` on label)
- **rahd-gui**: Drag-and-drop reschedule targeted the drag origin instead of the drop target
- **rahd-gui**: Multi-hour events were clamped to single hour-slot height; now uses `.clamp()` for proper sizing
- **rahd-gui**: Drag state leaked when pointer released outside the view; now cleaned up at end of frame
- **rahd-server**: Mutex `.unwrap()` on all endpoints replaced with proper error returns (prevented DoS via panic)
- **rahd-server**: `hoosh_intent` silently swallowed `create_event` tool errors; now surfaces them in response
- **rahd-server**: `list_tools` used `.unwrap()` on serialization; now returns 500 on failure
- **rahd-server**: Unknown tool returned HTTP 200; now returns 500 with error
- **rahd-store**: `update_event` now returns `bool` indicating whether the event existed
- **rahd-store**: `list_events` search now checks both title and description (was title-only)
- **rahd-store**: Added SQLite `PRAGMA journal_mode=WAL` and `busy_timeout=5000` for concurrent access safety
- **rahd-core**: `advance_to_next_month` panicked on `Recurrence::Monthly { day: 0 }`; now clamps to 1
- **rahd-core**: ICS import now handles RFC 5545 line folding (CRLF + whitespace continuation)
- **rahd-core**: vCard export now escapes special characters in FN, EMAIL, TEL, ORG fields
- **rahd-schedule**: `find_free_slots` panicked on `work_start >= 24`; now clamps and validates
- **rahd-ai**: `PriorityScorer` gave past events maximum urgency score; now returns 0
- **rahd-mcp**: `execute_free` loaded all events from store; now filters by target date
- **CLI**: `--week` started from today instead of Monday; `--month` used +30 days instead of calendar month boundaries
- **CLI**: `rahd edit` now reports "Event not found" when updating a nonexistent event

### Changed

- All crate versions bumped to 2026.3.18-1
- **rahd-server**: `hoosh::health()` simplified from `Result<bool>` to `bool`
- **rahd-server**: hoosh client uses configured `reqwest::Client` with connect timeout and pool limits
- **rahd-mcp**: `rahd_events` search description updated to mention description field

## [2026.3.18] - 2026-03-18

### Added

- **rahd-gui**: Desktop GUI with day/week/month calendar views (egui/eframe, wgpu backend)
  - Left sidebar with view switcher and mini calendar navigation
  - Day view with hourly timeline and event blocks
  - Week view with 7-column hourly grid
  - Month view with day cells showing event summaries
  - AGNOS dark theme (shared palette with jalwa, abaco, shruti, nazar)
  - Current time/day highlighting across all views
- **CLI**: `--gui` flag to launch the desktop GUI
- **rahd-server**: daimon API server on port 8090 (axum-based HTTP wrapper for MCP tools)
  - `GET /health` — health check
  - `GET /tools` — list all 5 MCP tool definitions
  - `POST /tools/{tool_name}` — execute any MCP tool with JSON parameters
- **CLI**: `rahd serve` subcommand to start the daimon API server (`--addr` for custom bind address)
- **8 server tests** covering all endpoints (health, tool listing, tool execution for all 5 tools, unknown tool handling)

### Changed

- All crate versions bumped to 2026.3.18
- **CI**: Aligned workflows with AGNOS project standards (nazar, abaco)
  - Consolidated check/lint into single job with fmt + clippy + check
  - Added security audit job (cargo-audit)
  - Added rust-cache (Swatinem/rust-cache@v2) for faster builds
  - Added concurrency group to cancel stale runs
  - CI now callable via `workflow_call` for release gating
- **Release**: Aligned with AGNOS release pattern
  - CI gate runs before build
  - Tar.gz archives with sha256 checksums (was raw binaries)
  - Simplified artifact collection with merge-multiple

## [2026.3.16] - 2026-03-16

### Added — Initial Release

- **rahd-core**: Calendar event, contact, recurrence, reminder, time slot, and conflict types
- **rahd-store**: SQLite-backed local storage with CRUD for events and contacts, filtering
- **rahd-schedule**: Conflict detection, free/busy analysis, meeting time suggestions
- **rahd-ai**: Natural language event parsing ("lunch with Sam tomorrow at noon"), priority scoring
- **rahd-mcp**: 5 MCP tool definitions (events, add, free, conflicts, contacts)
- **CLI**: `add`, `list`, `show`, `delete`, `contacts`, `free`, `conflicts` subcommands
- **CI/CD**: GitHub Actions for check, test, clippy, fmt, release (amd64 + arm64)
- **45+ tests** across all crates, 0 warnings
