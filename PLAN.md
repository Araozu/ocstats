# OpenCode Data Plan

## Scope

Collect local OpenCode usage data for a future UI. This project is read-only:
it does not call models, modify OpenCode data, or read credentials from
`auth.json`.

## Source Strategy

1. Read OpenCode's local SQLite database for complete historical data.
2. Optionally add the OpenCode HTTP API and event stream later for live updates.
3. Keep a separate analytics store for incremental imports and computed views.

The database is an internal OpenCode interface. The extractor must validate its
schema and fail clearly when an OpenCode upgrade changes it.

## Data Location

Resolve the XDG data directory as `$XDG_DATA_HOME/opencode`, falling back to
`~/.local/share/opencode`. Use `OPENCODE_DB` when it is set; absolute values
are used directly and relative values are resolved below the data directory.
The normal stable database is `opencode.db`; non-stable channels may use
`opencode-<channel>.db`.

Open SQLite in read-only mode. OpenCode uses WAL mode, so the `-wal` and `-shm`
files must remain beside the database while it is read. Never copy only the
main database file from an active OpenCode instance.

## Canonical Records

- Sessions: project, title, time range, selected model, and OpenCode's
  aggregate cost/token values.
- Assistant messages: provider/model plus message-level cost and token values.
- Completed steps: `part.data` records whose JSON `type` is `step-finish`;
  these are the finest persisted usage records.
- Projects: project identity, name, worktree, and VCS metadata.

Normalize every record with its stable OpenCode ID, source database path, and
timestamp. Preserve parsing issues separately instead of treating malformed or
unknown fields as zero.

## Extraction Rules

Use completed steps as the eventual authoritative usage source. Message-level
values are a fallback only when a message has no usable completed steps.
Session totals are for fast summaries and reconciliation, not an additional
additive input. This prevents double-counting.

Missing cost is unknown, not zero. Normalize timestamps to UTC and model
identity as provider, model, and optional variant.

## Compatibility and Validation

Before extraction, inspect SQLite metadata and require the tables and columns
needed by this version: `project`, `session`, `message`, and `part`. Maintain a
schema signature/version with imported data in the future analytics store. If
validation fails, report the missing items and do not return partial metrics.

## Later Work

1. Add an application-owned incremental analytics database.
2. Reconcile step totals to message and session totals, surfacing differences.
3. Add `opencode serve` REST/SSE ingestion for live updates, then reconcile
   after reconnect because the event stream is not a replay log.
4. Add filtering, period aggregation, and presentation APIs for the UI.
