# Development

## Prerequisites

- Rust 2024 edition (rustc 1.85+)
- No external dependencies — SQLite is bundled via rusqlite

## Build

```bash
cargo build --workspace
```

## Test

```bash
cargo test --workspace
```

## Lint

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

## Release

1. Update version:
   ```bash
   ./bump-version.sh 2026.3.17
   ```

2. Update `CHANGELOG.md` with new version section.

3. Commit, tag, and push:
   ```bash
   git add -A
   git commit -m "release 2026.3.17"
   git tag 2026.3.17
   git push origin main --tags
   ```

4. GitHub Actions will build amd64 + arm64 binaries and create a release.

## Project Structure

| Crate | Purpose |
|-------|---------|
| `rahd-core` | Types: Event, Contact, Calendar, Recurrence, TimeSlot, Conflict |
| `rahd-store` | SQLite persistence (rusqlite, bundled) |
| `rahd-schedule` | Conflict detection, free slots, meeting suggestions |
| `rahd-ai` | NL event parsing, priority scoring |
| `rahd-mcp` | MCP tool definitions for AGNOS |
