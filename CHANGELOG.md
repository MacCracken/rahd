# Changelog

All notable changes to Rahd will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
