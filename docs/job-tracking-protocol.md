# Print Job Tracking Protocol — agent ↔ web app

**Status**: design contract. Implements split:
- **Agent side** (Rust): persistent SQLite job log, Windows spooler polling, GET /jobs endpoint
- **Web side** (Next.js): per-card bulk fan-out, live progress UI, "retry failed" recovery

Both sides agree on this document before either codes against it. The single biggest failure mode in v1 — operator
runs "Print 100 cards", a jam at card 47, no clean way to know which 53 didn't print — is what this protocol is
designed to make automatic.

## Goals

1. **Per-card visibility**. Every printed card is a separate job with a stable identifier. Failure of card 47 is
   isolated — the operator (or the web UI) can re-fire just the failed cards with one click.
2. **Survives restarts**. Agent persists job state to disk. Web polls `GET /jobs?since=...` to reconcile after a
   page refresh, agent restart, or operator coming back tomorrow.
3. **Detects printer-side failures**. Agent polls the Windows spooler (`GetJob`) until terminal state — catches
   jams, ribbon-empty, paper-out, "user deleted from spooler".
4. **No webhook plumbing**. Web pulls; agent doesn't push. Every component already exists in v1 (HTTP listener, web
   poller). One fewer moving part than a push pipeline.

## Architecture decisions

| Decision | Rationale |
| --- | --- |
| **Per-card POSTs** for bulk, not multi-page | Recovery is trivial — failed cards are first-class jobs. Aligns with v0.2's pdfium-render path where per-card is the natural unit. Spooler overhead for 100 separate jobs is negligible. |
| **Web-generated `job_id`** (UUID v4) | Web owns the audit log + correlation. Agent stores opaque IDs without parsing. |
| **No agent-side dedup** | Each POST = new job, even if the same `job_id` is re-sent. Agent log captures all attempts. Web is responsible for not re-POSTing an in-flight job. |
| **Polling, not webhooks** | Web's existing `useAgent()` hook already polls every 5s. Adds `GET /jobs` to the same tick. No retries, no idempotency keys, no push-state plumbing. |
| **Query-string metadata** | Keeps body-as-raw-PDF contract from v1 unchanged. Metadata fields are query params on POST. Headers reserved for content-type + IDM-bypass. |

## POST /print (extended)

```
POST http://127.0.0.1:9988/print?<query>
Headers:
  Content-Type: application/pdf
Body: <PDF bytes>
```

### Query parameters

| Param | Required | Notes |
| --- | --- | --- |
| `printer` | no | Target printer queue name. Defaults to OS default if absent. Existing v1 behaviour. |
| `job_name` | no | Human label shown in the OS spooler UI. Existing v1 behaviour. |
| `job_id` | **new** | Web-generated UUID v4 (opaque). Stable across retries (web sends a NEW UUID per retry attempt). Agent uses this as the primary key in its job log. Required when web wants tracking; absent for one-off prints from curl / agent's own test-print button. |
| `run_id` | new | Web-generated UUID v4. Groups all per-card POSTs from one bulk run. Used by the web's progress UI to show "X / Y printed". Optional. |
| `label` | new | Short human-readable identifier shown in the agent's activity feed and `GET /jobs` response. e.g. `"Steven Broyer (#104616)"`. Optional. Max 200 chars. |
| `parent_job_id` | new | If this job is a retry of a previously-failed `job_id`, this points at the original. Lets analytics see "this run had 3 jams that were eventually all retried successfully". Optional. |

### Response (unchanged from v1, with one additional field)

```json
{
  "ok": true,
  "printer": "Fargo HDP5000",
  "job_name": "Steven Broyer (#104616)",
  "job_id": "9d6fb72f-6c09-4c84-b229-8f3958716d8e",
  "spooler_job_id": 137
}
```

| Field | Notes |
| --- | --- |
| `spooler_job_id` | **new** Windows print spooler job number returned by `AddJob`/`StartDocPrinter`. Lets the web correlate with Windows spooler UI for manual cancel/restart if needed. Null on macOS/Linux for now. |

The 200 response means **the job entered the spooler queue**, NOT that it physically printed. The agent's job-state
machine continues asynchronously after the response — the web learns about the actual printer-side outcome via
`GET /jobs`.

## GET /jobs

```
GET http://127.0.0.1:9988/jobs?since=<iso-8601>&run_id=<uuid>&limit=<int>
```

### Query parameters

| Param | Required | Notes |
| --- | --- | --- |
| `since` | no | ISO 8601 timestamp. Returns only jobs whose `updated_at >= since`. Web uses the previous response's max `updated_at` as the next request's `since` for incremental polling. |
| `run_id` | no | Filter to jobs in a specific bulk run. Web's progress UI uses this to scope "X / Y printed". |
| `limit` | no | Max rows. Default 100, hard-cap 500. |

### Response

```json
{
  "jobs": [
    {
      "job_id": "9d6fb72f-6c09-4c84-b229-8f3958716d8e",
      "run_id": "f1234567-...",
      "parent_job_id": null,
      "label": "Steven Broyer (#104616)",
      "printer": "Fargo HDP5000",
      "spooler_job_id": 137,
      "state": "printed",
      "error": null,
      "submitted_at": "2026-04-30T18:00:00.123Z",
      "started_at":   "2026-04-30T18:00:00.456Z",
      "completed_at": "2026-04-30T18:00:05.789Z",
      "updated_at":   "2026-04-30T18:00:05.789Z",
      "pages_printed": 1,
      "attempts": 1
    },
    ...
  ],
  "next_since": "2026-04-30T18:00:05.789Z"
}
```

`next_since` is the max `updated_at` across the returned jobs — web uses it as the `since` parameter on the next
poll. If the response is empty, web reuses its previous `since` until the next poll cycle.

## Job state machine

```
       POST /print
            │
            ▼
       ┌─────────┐
       │ queued  │   agent received the POST, hasn't yet dispatched to the spooler
       └────┬────┘
            │ agent calls SumatraPDF / lp / WritePrinter, gets spooler_job_id
            ▼
       ┌──────────┐
       │submitted │   in OS spooler queue, awaiting printer
       └────┬─────┘
            │ spooler reports JOB_STATUS_PRINTING (Windows) or equivalent
            ▼
       ┌──────────┐
       │printing  │   physically pulling paper / film
       └────┬─────┘
            │ spooler reports JOB_STATUS_PRINTED + status=DONE
            ▼
       ┌─────────┐
       │ printed │   terminal — success
       └─────────┘

   any non-terminal state can transition to:
       ┌─────────┐
       │ failed  │   error: spooler reported JOB_STATUS_ERROR / paper-out / etc.
       └─────────┘  `error` field has a human-readable string.
       ┌──────────┐
       │cancelled │   user deleted job from spooler UI, or POST /jobs/{id}/cancel
       └──────────┘
```

`state` values are exactly one of: `queued`, `submitted`, `printing`, `printed`, `failed`, `cancelled`.

## Recovery flow — the canonical example

**Operator clicks "Print 100 cards":**

1. Web generates `run_id = uuid()`. For each employee:
   - generates `job_id = uuid()`
   - POSTs `/print?job_id=…&run_id=…&label=…&printer=…` with the per-employee PDF
2. Web stores all `(job_id, employee_id)` mappings client-side (or in its own audit log).
3. Web starts polling `GET /jobs?since=…&run_id=…` every 2 seconds.
4. UI renders:
   ```
   Bulk print · 47 / 100 cards printed
   ▰▰▰▰▰▰▰▰▰▱▱▱▱▱▱▱▱▱▱▱▱▱▱
   ✓ Steven Broyer
   ✓ Alex Rivera
   ⚠ Jordan Lee · jammed (job 47)
   ⏳ Sam Kim · printing
   ▢ 52 cards queued
   ```
5. Agent reports `state: failed, error: "Paper jam at card eject"` for `job_id` 47.
6. Operator clears jam. Spooler may automatically retry, in which case agent updates state to `printing` →
   `printed`. Or the spooler holds the queue.
7. If automatic retry didn't happen, web shows "1 card failed — retry?". Operator clicks. Web POSTs a **new
   `job_id`** with `parent_job_id = <the failed one>` and the same employee's PDF. Agent treats it as a fresh job
   that knows it's a retry.
8. Web's audit log + agent's SQLite both show the full chain — original failed → retry succeeded.

## SQLite schema (agent side)

```sql
CREATE TABLE IF NOT EXISTS jobs (
  job_id          TEXT PRIMARY KEY,            -- UUID from web (or agent-generated for ad-hoc prints)
  run_id          TEXT,                        -- bulk run ID, null for one-offs
  parent_job_id   TEXT REFERENCES jobs(job_id),
  label           TEXT,
  printer         TEXT NOT NULL,
  spooler_job_id  INTEGER,                     -- Windows spooler job number
  state           TEXT NOT NULL,               -- queued|submitted|printing|printed|failed|cancelled
  error           TEXT,
  submitted_at    TEXT NOT NULL,               -- ISO 8601
  started_at      TEXT,
  completed_at    TEXT,
  updated_at      TEXT NOT NULL,
  pages_printed   INTEGER NOT NULL DEFAULT 0,
  attempts        INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_jobs_run_id ON jobs(run_id);
CREATE INDEX IF NOT EXISTS idx_jobs_updated_at ON jobs(updated_at);
CREATE INDEX IF NOT EXISTS idx_jobs_state ON jobs(state);
```

`updated_at` is updated on every state transition. Index serves the `?since=` query without scanning the whole
table.

DB file lives at `%LOCALAPPDATA%\BadgeBadger\Agent\jobs.db` (Windows) or `~/Library/Application Support/BadgeBadger/Agent/jobs.db` (macOS).

## Open questions

1. **Cancel endpoint**? `DELETE /jobs/{job_id}` would let the web UI cancel an in-flight job. Useful, low effort,
   skip until needed.
2. **Pruning**? Job log grows forever. Add `DELETE FROM jobs WHERE updated_at < ?` task that runs daily on agent
   startup, defaulting to 30-day retention. Bonus: `agent.conf` setting for venues that want longer.
3. **Active spooler poll cadence**? Polling `GetJob` every 500ms while a job is non-terminal is fine for one job;
   100 concurrent jobs = 200 syscalls/sec. Probably acceptable, but if needed we batch-poll once per second across
   all in-flight jobs.

## What each side ships

### Agent side (Rust)

- New `db.rs` module — SQLite via `rusqlite` (~130KB binary cost). Init on startup, migrations, prepared statements.
- New `tracker.rs` — Windows spooler poll loop spawned per `submitted` job. Updates jobs table on transitions.
- `http_server.rs` — `POST /print` accepts new query params, writes a `queued` row, dispatches, writes
  `submitted`. Response includes `job_id` + `spooler_job_id`.
- New route `GET /jobs` — query the SQLite table, return JSON.
- New route `DELETE /jobs/{job_id}` — for cancel UX (deferred).

### Web side (Next.js)

- Update `lib/agent/client.ts`:
  - `printViaAgent({pdf, printer, jobName, jobId, runId, label, parentJobId})` — pass tracking params.
  - New `pollAgentJobs({since, runId})` — paged GET /jobs polling helper.
  - New `useAgentJobs(runId, pollMs)` hook — returns the live job array.
- Update `app/(app)/employees/employees-list.tsx`:
  - Generate `run_id` + per-employee `job_id`s up front.
  - Replace single multi-page POST with per-card POSTs (concurrency 4 — keeps the spooler moving without
    overwhelming).
  - Render bulk-print progress dialog (component below).
- New `components/print/bulk-print-progress.tsx`:
  - Live count, per-card status, retry-failed button, cancel button.
  - Polls `useAgentJobs(runId, 2000)`.
- Update `/settings/printers/agents` page:
  - Activity card lists last 50 jobs from `GET /jobs?limit=50` (replaces the in-memory ring buffer that's lost on
    reload).

## Versioning

This protocol is **v1**. Agent's `/health` payload gains a `protocol: "1"` field — web reads it on first probe and
disables tracking features (per-card POST, GET /jobs, retry UI) when the agent reports an older protocol or
none. Both sides ship the v1 protocol in lockstep; mismatches degrade gracefully to v0 (one POST per print, no
tracking), which is what we have today.
