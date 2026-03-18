# Changelog

All notable changes to Rahd will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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

### Changed

- All crate versions bumped to 2026.3.18

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
