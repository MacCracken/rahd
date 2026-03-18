# MCP Tools Reference

Rahd exposes 5 MCP tools for AGNOS agent integration via daimon's tool registry.

Tools are available both programmatically (`rahd_mcp::execute_tool`) and over HTTP
via `rahd serve` (see [usage guide](usage.md#daimon-api-server)).

## rahd_events

List or search calendar events.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `from` | string | No | Start date (YYYY-MM-DD) |
| `to` | string | No | End date (YYYY-MM-DD) |
| `search` | string | No | Text search in event titles |

## rahd_add

Add a new calendar event using natural language or structured parameters.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `description` | string | No | Natural language description |
| `title` | string | No | Event title (structured mode) |
| `start` | string | No | Start datetime ISO 8601 |
| `end` | string | No | End datetime ISO 8601 |

## rahd_free

Find free time slots on a given date.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `date` | string | No | Date (YYYY-MM-DD, default: today) |
| `work_start` | integer | No | Start hour (default: 9) |
| `work_end` | integer | No | End hour (default: 17) |

## rahd_conflicts

Detect scheduling conflicts among calendar events.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `from` | string | No | Start date range |
| `to` | string | No | End date range |

## rahd_contacts

List or search contacts.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `search` | string | No | Text search across contact fields |
| `limit` | integer | No | Max results (default: 50) |
