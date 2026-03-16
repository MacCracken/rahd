# Contributing to Rahd

## Build

```bash
cargo build --workspace
```

## Test

```bash
cargo test --workspace
```

Tests include:
- **Unit tests** in each crate (`rahd-core`, `rahd-store`, `rahd-schedule`, `rahd-ai`, `rahd-mcp`)
- **Integration tests** in `tests/integration.rs` — end-to-end CLI round-trips

## Lint

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

## Architecture

See [docs/architecture.md](docs/architecture.md) for the crate structure and data flow.

```
rahd-core       — types (Event, Contact, Calendar, Recurrence, TimeSlot)
                  + ICS/vCard serialization + recurring event expansion
rahd-store      — SQLite persistence (CRUD operations)
rahd-schedule   — conflict detection, free/busy, meeting suggestions
rahd-ai         — natural language parsing, priority scoring
rahd-mcp        — MCP tool definitions + execution for AGNOS integration
src/main.rs     — CLI binary (clap)
tests/          — integration tests
```

## Adding a new CLI command

1. Add the variant to the `Commands` enum in `src/main.rs`
2. Add the match arm in `main()`
3. Add an integration test in `tests/integration.rs`

## Adding a new MCP tool

1. Add the `ToolDescription` to `tool_definitions()` in `rahd-mcp`
2. Add the execution handler in `execute_tool()`
3. Add a unit test
4. Document in `docs/mcp-tools.md`
