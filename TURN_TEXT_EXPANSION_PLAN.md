# Turn Text Expansion Plan

## Purpose

Add lazy, per-turn assistant text to the session Turns table without increasing the size of the existing session-detail response.

Each completed turn will gain an Actions control. Activating the control requests that turn's text from the backend and expands a detail row when the request succeeds. Activating it again collapses the detail row without discarding the cached response.

The existing User message section will use the same icon-and-label expand/collapse control so both kinds of expandable content have one interaction language.

This document intentionally divides the work into independently reviewable increments. Each increment should leave the repository building and its existing behavior intact.

## Current Architecture

### Source extraction

- `src/lib.rs` opens OpenCode's SQLite database read-only and validates the `project`, `session`, `message`, and `part` tables.
- OpenCode message metadata is parsed by `extract_messages`.
- User message text is already assembled by `message_text`, which selects all `part` rows with `data.type = "text"`, orders them by `time_created, id`, and joins their `text` values with newlines.
- Assistant message metadata is represented by `AssistantMessage`, but assistant text is currently discarded.
- A UI turn is represented by a `CompletedStep`, extracted from a `part` whose type is `step-finish`.
- `CompletedStep.message_id` links the turn to its assistant message.

### Analytics storage

- `src/analytics.rs` imports extracted records into the application-owned analytics SQLite database.
- `assistant_message` stores model identity, usage, parent user-message ID, and timestamps, but no text.
- `completed_step` stores the records shown as turns.
- `user_message` already stores imported text.
- `AnalyticsStore::session_detail` joins a completed step to its assistant message and parent user message, returning `SessionDetail.turns`.
- The analytics database is persisted, so existing installations require a non-destructive schema migration and a clear state for rows imported before assistant text support.

### Backend API

- `src/server.rs` exposes authenticated Axum routes under `/api`.
- Session detail is currently fetched through `GET /api/usage/session?source=...&session_id=...`.
- Protected routes use the existing cookie authentication middleware.
- Store operations are serialized through `Arc<Mutex<AnalyticsStore>>`.

### Frontend

- `frontend/src/lib/components/dashboard/session-turns.svelte` renders the table and inserts a User message row before the first turn associated with each user message.
- `frontend/src/lib/components/dashboard/session-turn-row.svelte` renders the eight current turn columns.
- `frontend/src/lib/components/dashboard/session-user-message-row.svelte` clamps long messages and exposes a text-only Show more/Show less ghost button.
- `frontend/src/lib/api/ocstats.ts` owns API response types and fetch functions.
- `frontend/src/lib/queries/usage.ts` owns TanStack Svelte Query options and query keys.
- There is currently no frontend test runner. Static validation is performed with `pnpm check`, `pnpm lint`, and `pnpm build`.

## Product Decisions

### Definition of turn text

For this feature, a turn's text is the ordered concatenation of OpenCode `text` parts belonging to the assistant message identified by `completed_step.message_id`.

This matches the repository's current turn model: usage belongs to a completed step, while model identity and conversation linkage belong to its assistant message. It also reuses the established user-message text extraction behavior.

Before implementing storage, add a focused fixture proving the expected relationship between an assistant message, its text parts, and its `step-finish` record. If real OpenCode data demonstrates that multiple distinct completed steps can share one assistant message and need different text slices, stop after the extraction checkpoint and revise the model to persist text by completed-step boundary instead of duplicating message-level text. Do not silently guess boundaries from timestamps in the API handler.

### Lazy loading

- Do not add assistant text to `SessionDetail` or `Turn`.
- Fetch text only after the user activates a turn's action.
- Cache successful responses by source, session, and turn ID using TanStack Query.
- Collapsing and reopening a turn should use cached data and should not issue an immediate duplicate request.
- Multiple turns may remain expanded independently.

### Rendering

- Render returned text as Svelte text content, never as raw HTML.
- Preserve line breaks with `whitespace-pre-wrap` and allow long tokens/URLs to wrap with `break-words`.
- Do not render Markdown in the first version. Plain text is safer and exactly reflects the stored assistant output.
- Do not include reasoning parts, tool input, tool output, or other part types. This endpoint returns assistant-visible `text` parts only.
- Do not truncate backend responses. The user explicitly requests the text of the turn. The table detail cell can wrap and grow vertically.

### Action control

- Add a ninth table column named Actions.
- Use the existing shadcn `Button` component, preferably `variant="ghost"` and `size="xs"` for a compact table action.
- Use a Phosphor `CaretDownIcon` or equivalent `*Icon` import, following `frontend/AGENTS.md`.
- Label a closed turn action `Show text` and an open turn action `Hide text`. This is more specific than the ambiguous word Dropdown while preserving the requested dropdown-style icon.
- Rotate the caret when expanded, and expose state with `aria-expanded` and `aria-controls`.
- Give loading, failure, unavailable, empty, and expanded states distinct accessible labels.
- Update the User message control to the same icon-plus-label shape, retaining Show more/Show less wording.

## Proposed API Contract

### Request

```http
GET /api/usage/turn-text?source=<encoded-source>&session_id=<encoded-session-id>&turn_id=<encoded-turn-id>
Cookie: ocstats_session=...
```

All three identifiers are required.

- `source` disambiguates records imported from multiple OpenCode databases.
- `session_id` scopes the lookup to the session currently displayed and prevents a stale/mismatched row from retrieving a turn in another session.
- `turn_id` is the existing stable `completed_step.id` already sent in `SessionDetail.turns`.

### Success response

```json
{
  "turn_id": "part-step-finish-id",
  "message_id": "assistant-message-id",
  "text": "The assistant response text"
}
```

Use `text: string | null` in the serialized contract during migration:

- A non-empty string means imported text is available.
- An empty string means the current extractor processed the assistant message but it had no `text` parts.
- `null` means the analytics row predates assistant text support and needs a new import before text can be served.

This distinction is necessary because existing analytics databases are persisted and adding a nullable column cannot reconstruct historical text without rereading the OpenCode source database.

### Status behavior

- `200 OK`: the source/session/turn exists. The body contains a string, an empty string, or `null` as described above.
- `400 Bad Request`: required query values cannot be deserialized or are absent. Axum's extractor can provide this behavior.
- `401 Unauthorized`: existing authentication middleware behavior.
- `404 Not Found`: no completed step matches all of `source`, `session_id`, and `turn_id`.
- `500 Internal Server Error`: analytics lookup or lock failure, using the existing JSON error shape.

Do not automatically read the live OpenCode database from this GET endpoint. Imports are the existing synchronization boundary; keeping reads in `AnalyticsStore` avoids blocking request handling on source-database access and keeps behavior consistent with the rest of the dashboard.

## Backend Implementation

### Increment 1: Extract assistant text

Files:

- `src/lib.rs`

Changes:

1. Add `text: String` to `AssistantMessage`.
2. Rename `message_text` only if needed for clarity; it already works for either role and can remain as-is to minimize change.
3. In the assistant branch of `extract_messages`, call `message_text(connection, &id)` and retain the result in `AssistantMessage`.
4. Preserve the existing ordering of text parts by `time_created, id` and newline joining.
5. Continue treating missing `text` fields inside a text part as an empty fragment, unless the new fixture demonstrates this should become a parse issue. Avoid broad parser policy changes in this feature.
6. Update all `AssistantMessage` construction sites in tests.

Tests:

1. Build an in-memory OpenCode schema fixture with one assistant message.
2. Insert two ordered text parts and one non-text part.
3. Assert extraction returns the two text values in source order, joined by a newline.
4. Assert tool/reasoning/step parts do not enter the assistant text.
5. Include one `step-finish` part tied to the same message to document the turn-to-message relation.
6. If practical, add a textless assistant fixture and assert its extracted text is `""`.

Review checkpoint:

- Confirm the definition of text from source records before changing persistent storage or API behavior.
- Run `cargo test`.

### Increment 2: Persist assistant text safely

Files:

- `src/analytics.rs`

Changes:

1. Add nullable `text TEXT` to the fresh `assistant_message` table definition.
2. Add a non-destructive `ensure_column(..., "assistant_message", "text", "TEXT")` migration for existing analytics databases.
3. Keep migrated existing rows as `NULL`; this intentionally represents not-yet-imported text.
4. Extend `upsert_message` and the assistant-message branch of `upsert_usage_record` to insert and update text.
5. Prefer passing text explicitly in the assistant-message write path rather than adding meaningless text fields to completed-step writes. If extending the shared `UsageRecord` would make the SQL branching harder to understand, split the assistant upsert into a direct statement instead. Choose the smallest readable change during implementation.
6. Consider incrementing `EXTRACTOR_SCHEMA_VERSION` from 1 to 2 to identify imports produced by the text-aware extractor. The source schema signature does not change. If this version is intended strictly to describe the upstream schema rather than extraction completeness, document that and rely on the nullable column as the migration marker instead.

Tests:

1. Extend `imports_idempotently_and_prefers_steps_over_messages` with assistant text.
2. Import once, change the extracted text, import again, and assert the stored response is updated.
3. Open a database initialized with the old `assistant_message` shape, run `AnalyticsStore::open`, and assert migration adds the column without dropping usage data.
4. Assert an old row has `NULL` text after migration.
5. Assert a freshly imported textless assistant has `Some("")`, preserving the unavailable-versus-empty distinction.

Review checkpoint:

- Inspect the generated SQLite migration path and idempotent upsert behavior.
- Run `cargo test` twice against the migration fixture to establish repeatability.

### Increment 3: Add the store lookup

Files:

- `src/analytics.rs`
- `src/lib.rs` for public re-export if the response type lives in analytics

Types:

```rust
pub struct TurnText {
    pub turn_id: String,
    pub message_id: String,
    pub text: Option<String>,
}
```

Suggested method:

```rust
pub fn turn_text(
    &self,
    source: &str,
    session_id: &str,
    turn_id: &str,
) -> Result<Option<TurnText>, Error>
```

Lookup behavior:

1. Query `completed_step` by all three request identifiers.
2. Join `assistant_message` using both `source` and `message_id`.
3. Return the completed step ID, assistant message ID, and nullable assistant text.
4. Return `None` when the completed step does not exist in the requested source/session.
5. Decide explicitly how to handle a completed step whose assistant message row is missing. The recommended behavior is still `404`, because the API cannot establish a valid text-bearing turn. A left join plus `message_id` with null text would incorrectly look like a migration-only state.

Tests:

1. Existing turn returns the expected IDs and text.
2. Textless imported turn returns `Some("")` inside the result.
3. Pre-feature migrated row returns `None` text inside an existing result.
4. Wrong source returns no result.
5. Wrong session returns no result.
6. Wrong turn ID returns no result.
7. A turn cannot retrieve another session's assistant text.

Review checkpoint:

- Review SQL scoping and null semantics independently of HTTP concerns.
- Run `cargo test`.

### Increment 4: Expose the authenticated route

Files:

- `src/server.rs`

Changes:

1. Add a `TurnTextQuery` deserialization type with `source`, `session_id`, and `turn_id`.
2. Register `GET /api/usage/turn-text` in `protected_api` so existing authentication applies.
3. Add a handler that calls `AnalyticsStore::turn_text` through `with_store`.
4. Serialize a found `TurnText` with `Json`.
5. Map a missing result to `404` with a stable message such as `turn not found`.
6. Keep error details generic on the frontend; do not include source paths in newly authored user-facing failure messages.

Endpoint tests:

1. Authenticated request returns `200` and the exact JSON fields.
2. Missing turn returns `404`.
3. Mismatched session returns `404`.
4. Missing query parameter returns `400`.
5. Unauthenticated request returns `401`.
6. Empty and null text both return `200` with distinct JSON values.

Review checkpoint:

- Exercise the endpoint with a URL-encoded source path, including spaces if supported by the fixture.
- Run `cargo fmt --check`, `cargo clippy --all-targets --all-features`, and `cargo test`.

## Frontend Implementation

### Increment 5: Add the typed client and query definition

Files:

- `frontend/src/lib/api/ocstats.ts`
- `frontend/src/lib/queries/usage.ts`

Types:

```ts
export type TurnText = {
	turn_id: string;
	message_id: string;
	text: string | null;
};
```

Client function:

```ts
export function getTurnText(source: string, sessionId: string, turnId: string) {
	const params = new URLSearchParams({
		source,
		session_id: sessionId,
		turn_id: turnId
	});
	return get<TurnText>(`/usage/turn-text?${params}`);
}
```

Query options:

- Add `usageQueries.turnText(source, sessionId, turnId, enabled)`.
- Use `['turn-text', source, sessionId, turnId]` as the query key.
- Enable it only after the user requests the turn and all identifiers are present.
- Use the normal TanStack Query cache rather than adding a parallel component-level text cache.
- Do not refetch merely because a row is collapsed and reopened. The library default stale behavior may refetch on broader lifecycle events; if that produces duplicate calls during review, set an explicit `staleTime` suitable for immutable imported text and rely on query invalidation after import.

Import invalidation consideration:

The current import flow manually refetches dashboard queries. Once turn text is cached, a new import may update assistant output. Add invalidation/refetch behavior for query keys beginning with `turn-text` after a successful import, or define a finite stale policy. Explicit invalidation is preferred so reopening a previously viewed turn cannot show text from before the latest import.

Review checkpoint:

- Confirm query-key identity includes `source`, not only `turn_id`.
- Run `pnpm check`.

### Increment 6: Add the turn Actions control and detail row

Files:

- `frontend/src/lib/components/dashboard/session-turns.svelte`
- `frontend/src/lib/components/dashboard/session-turn-row.svelte`
- Optionally add one focused component such as `session-turn-text-row.svelte` if keeping loading/error/content markup in `session-turn-row.svelte` becomes difficult to scan.

Table structure:

1. Add an Actions header after Pricing.
2. Add the matching ninth cell to every regular turn row.
3. Change the empty-state `colspan` from 8 to 9.
4. Ensure all expanded/detail rows use `colspan={9}`.
5. Repair the User message row's column structure as part of Increment 7; it currently spans seven cells despite the table having eight columns.

State model:

- Keep whether the user requested/expanded a turn local to that keyed row, or keep a set of expanded turn IDs in `session-turns.svelte` if review shows row state should be controlled centrally.
- Prefer local keyed-row state because each `SessionTurnRow` already has stable identity through `{#each ... (turn.id)}` and independent expansion is desired.
- Create the turn-text query inside the row component using `session.source`, `session.session_id`, and `turn.id`, or pass `source` and `sessionId` as explicit props.
- Do not infer source/session from global URL state in a table-row component.

Interaction sequence:

1. Closed and uncached: action displays a down caret plus Show text.
2. First activation: start the query and expose a loading state on the action. Keep the metrics row in place.
3. Successful response: insert a sibling detail `TableRow` directly after the metrics row and set `aria-expanded=true`.
4. Open activation: remove the detail row while retaining query data in cache.
5. Reopen: show cached text immediately.
6. Failure: expose a compact error detail with a Retry action, or change the action label to Retry and provide an adjacent screen-reader/visible error message. Prefer an error detail row because icon-only error signaling is insufficient.

The product wording says the row expands when data is received. The preferred implementation therefore keeps the detail row closed during the initial request and shows progress in the Actions button. If the request fails, an error detail row may open so the failure and retry control are visible. This exception is preferable to a silent failure.

Detail states:

- Non-empty text: render in a `<p>` or `<pre>`-like block with `whitespace-pre-wrap break-words`, inheriting existing card/table tokens.
- Empty string: show `No text output for this turn.`
- Null text: show `Text is unavailable for this imported turn. Import OpenCode data again.`
- Error: show `Unable to load turn text.` and a Retry button.
- Loading: action label becomes `Loading` and includes a spinner/progress icon or animated caret; disable duplicate activation while the initial request is pending.

Accessibility:

- Assign a stable detail-row content ID derived from a safe local value, not an unescaped raw source path.
- Set `aria-controls` to that ID.
- Set `aria-expanded` from actual visible detail state.
- Keep a visible text label; do not make the control icon-only.
- Mark decorative caret icons `aria-hidden=true`.
- Preserve keyboard activation through the native Button element.
- If the content opens only after an asynchronous request, announce loading and failure using visible text and an appropriate `aria-live="polite"` region where necessary.

Responsive behavior:

- Keep the Actions column visible on mobile because it is the only way to retrieve text.
- Use a compact button and allow the header to remain narrow.
- Verify the added column does not make the critical Model/Input/Output cells unusable. Horizontal table scrolling is acceptable if it follows the existing Table wrapper behavior.
- Expanded text must wrap and must not force the table to the width of a long line.

Visual behavior:

- Use only existing CSS color tokens from the main stylesheet.
- Give the detail row a subtle existing-token background such as `bg-muted/30` only if it improves association with its metrics row.
- Avoid adding a new card inside the table; the row expansion should remain visually part of the turn.
- Rotate the caret with a short transition rather than swapping unrelated icons.

Review checkpoint:

- Manually exercise one, several, empty, unavailable, and failed turn expansions.
- Verify only the selected turn sends a request.
- Verify collapse/reopen is immediate and cached.
- Run `pnpm check`, `pnpm lint`, and `pnpm build`.

### Increment 7: Unify User message expansion

Files:

- `frontend/src/lib/components/dashboard/session-user-message-row.svelte`
- `frontend/src/lib/components/dashboard/session-turns.svelte` if state ownership or props change

Changes:

1. Replace the current text-only Show more/Show less button with the same Button variant, size, caret icon, icon rotation, and label placement used for turn text.
2. Preserve the current three-line collapsed presentation.
3. Preserve the current threshold unless manual review reveals the character-count check does not match visual overflow. Do not add ResizeObserver-based overflow measurement in the first version unless there is a demonstrated need.
4. Keep short user messages unencumbered; only show the expand action when content can be collapsed under the existing rule.
5. Set `aria-expanded` and `aria-controls` consistently.
6. Align the control with the new Actions column where practical: use one content cell spanning the first eight columns and one action cell for the ninth. This gives user and assistant rows the same interaction location.
7. Keep the User message summary cost visible in the content area.
8. Key expanded user-message state by stable message identity rather than text when possible. The current API exposes only the text on each turn, so duplicate user messages can collide in `expandedMessages`. The preferred backend follow-up is to include `user_message_id` in `Turn`; if that is considered too broad for this feature, key by the first turn ID for each displayed message group in `session-turns.svelte`.

The last point is a concrete correctness fix worth including: two identical user prompts in one session should not expand and collapse together.

Review checkpoint:

- Verify long user messages use the same interaction language as turn text.
- Verify two identical user-message strings expand independently.
- Verify all table rows resolve to nine columns.
- Run `pnpm check`, `pnpm lint`, and `pnpm build`.

### Increment 8: Integrate import invalidation and end-to-end states

Files:

- `frontend/src/routes/+page.svelte`
- Query utilities as needed

Changes:

1. After `importData()` succeeds, invalidate cached turn-text queries in addition to refetching session summaries.
2. If the currently selected session is refetched after import, ensure expanded rows do not display stale text from old query data.
3. Decide whether expanded rows remain open through a successful import. Recommended behavior: keep expansion state, invalidate/refetch visible expanded text, and show its loading state; do not unexpectedly collapse user context.
4. Confirm authentication expiration is handled consistently with other dashboard query failures. Do not add route-specific login behavior.

Review checkpoint:

- Load a null/unavailable pre-feature row, run Import, and verify it becomes text or the empty state without a full browser reload.
- Change source text in a fixture, re-import, and verify previously cached text updates.

## Test Strategy

### Rust unit tests

- Assistant text extraction preserves ordering.
- Non-text parts are excluded.
- Textless assistant extraction is represented as an imported empty string.
- Analytics schema migration is non-destructive and idempotent.
- Assistant text upserts update on repeated import.
- Turn lookup is scoped by source, session, and turn ID.
- Missing assistant joins do not leak or fabricate text.
- Null and empty text remain distinguishable.

### Rust HTTP tests

- Authentication is required.
- Required query parameters are enforced.
- Success JSON matches the contract.
- Not-found and mismatched ownership cases return 404.
- URL encoding works for source paths.
- Null and empty text responses both return 200.

### Frontend static checks

Run from `frontend/`:

```sh
pnpm check
pnpm lint
pnpm build
```

### Repository checks

Run from the repository root:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features
cargo test
```

### Manual browser matrix

1. Desktop, one normal text turn.
2. Desktop, several turns expanded simultaneously.
3. Mobile width with Actions still reachable.
4. Very long multiline text.
5. A long unbroken URL/token that must wrap.
6. Text containing Markdown and HTML-like strings, shown literally rather than interpreted.
7. Empty imported assistant text.
8. Null text from an analytics database migrated before re-import.
9. Backend 404 caused by stale session data.
10. Backend 500/network failure followed by Retry.
11. Collapse and reopen after success, confirming no immediate duplicate request.
12. Two identical user messages, confirming independent controls.
13. Import while a turn is expanded, confirming cache invalidation.
14. Keyboard-only expansion and collapse.

## Security And Privacy

- Keep the route inside `protected_api`.
- Scope every lookup by source, session, and turn ID.
- Return only `text` part content. Do not expose raw part JSON, tool arguments, tool results, reasoning, or message metadata not required by the contract.
- Render as escaped text in Svelte. Do not use `{@html}`.
- Do not log response text or include it in error messages.
- Treat source paths as identifiers in the request but do not echo them in the response.
- Do not add assistant text to list/session payloads where it would be fetched without explicit user intent.

## Performance

- The endpoint performs a point lookup using the `completed_step` composite primary key and an indexed/primary-key assistant-message join.
- The request returns one turn only.
- TanStack Query prevents repeated network fetches during ordinary collapse/reopen interactions.
- Keeping text out of `SessionDetail` avoids multiplying initial dashboard payload size.
- Import cost increases because assistant text parts are read and stored. The existing `message_text` implementation performs one part query per message; this feature can initially reuse it for the smallest change. If import profiling demonstrates an N+1 bottleneck, optimize user and assistant text extraction together in a separate change rather than introducing an unreviewed bulk parser here.
- No arbitrary response-size limit is proposed initially. If real outputs cause browser or server pressure, measure first and define explicit truncation/pagination semantics rather than silently clipping text.

## Risks And Mitigations

### Turn/message boundary mismatch

Risk: more than one completed step may point to one assistant message, causing each turn to return the same message-level text.

Mitigation: make the source fixture and relationship validation the first checkpoint. If this occurs in supported data, persist text by step boundary with an explicit extraction algorithm before exposing the route.

### Existing analytics rows have no text

Risk: migration creates the column but cannot reconstruct data.

Mitigation: keep the column nullable, return `text: null`, display an Import instruction, and invalidate turn queries after import.

### Duplicate user-message strings

Risk: current expansion state is keyed by message text, so identical prompts toggle together.

Mitigation: key UI groups by stable ID or the first turn ID, not content.

### Table column mismatch

Risk: adding Actions can produce malformed row spans; the current User message row already spans seven cells in an eight-column table.

Mitigation: audit every header, regular row, user row, detail row, and empty row against the nine-column structure.

### Stale cached content after import

Risk: TanStack Query may continue showing old assistant text.

Mitigation: invalidate the `turn-text` query family after every successful import.

### Large or hostile-looking text

Risk: huge lines can break layout, and HTML-like output could become unsafe if rendered as markup.

Mitigation: wrap content, preserve whitespace, render only escaped text, and never use raw HTML.

## Acceptance Criteria

### Backend

- Assistant text parts are imported into the analytics database.
- Existing analytics databases migrate without data loss.
- Existing rows not yet re-imported are distinguishable from imported textless turns.
- An authenticated endpoint returns one turn's text by source/session/turn identity.
- The endpoint returns 404 for mismatched or unknown identities.
- Assistant text is not added to the session-detail payload.
- Extraction, storage, lookup, and route tests pass.

### Frontend

- The Turns table has an Actions column.
- Every completed turn has an icon-plus-label Show text/Hide text action.
- Activating an uncached turn requests only that turn's text.
- The detail row appears directly under its metrics row after successful loading.
- Loading, error/retry, empty, unavailable, expanded, and collapsed states are understandable.
- Multiple turns can be expanded independently.
- Collapse/reopen uses cached data until import invalidates it.
- Assistant text is rendered literally with preserved line breaks and safe wrapping.
- Long User message rows use the same caret-plus-label paradigm.
- Identical user-message strings expand independently.
- Desktop, mobile, keyboard, and screen-reader semantics remain usable.
- `pnpm check`, `pnpm lint`, and `pnpm build` pass.

### Whole repository

- `cargo fmt --check`, `cargo clippy --all-targets --all-features`, and `cargo test` pass.
- No new color values are introduced; existing CSS tokens are used.
- No raw OpenCode data is modified.

## Suggested Pull Request Sequence

1. **Extraction and persistence:** add assistant text extraction, migration, upsert behavior, and Rust tests.
2. **Lookup and endpoint:** add `TurnText`, scoped store lookup, authenticated route, and HTTP tests.
3. **Typed frontend data layer:** add API types/functions, query options, and import invalidation.
4. **Turn expansion UI:** add Actions, lazy loading, detail rows, errors, empty states, accessibility, and responsive behavior.
5. **User message consistency:** convert the existing control and fix stable grouping/column spans.
6. **Final verification:** run all automated checks and complete the manual browser matrix.

Each pull request can be reviewed and merged independently after its checkpoint, except the endpoint should not be considered user-visible until the extraction/persistence increment is deployed and a fresh import has run.

## Deferred Work

- Markdown rendering and syntax highlighting.
- Displaying reasoning, tool calls, or tool outputs.
- Copy-to-clipboard controls.
- Full-text search over turn content.
- Prefetch-on-hover.
- Virtualization for very large turn tables.
- Streaming turn text.
- Bulk extraction optimization unless import profiling demonstrates a real regression.
