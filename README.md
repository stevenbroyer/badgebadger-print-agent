# BadgeBadger Print Agent

Native Windows + macOS app that runs in the operator's system tray and submits
PDFs to a local printer on demand. Replaces the popup-based browser print path
with a truly silent flow: the web app sends a PDF to the agent over a local
HTTP endpoint (v1) or a paired WebSocket (v2), and the agent shells out to
the OS print spooler.

**Status**: v1 scaffold — local HTTP listener only, no pairing yet, no
WebSocket. Designed so a curl POST of a PDF to `http://localhost:9988/print`
prints the PDF on the default Windows printer. The web app is unchanged
during this phase; we'll wire pairing + WebSocket in v2 once Casino Pier is
live on the existing browser-print flow.

## Stack

- **Tauri 2** — small native binary (~5MB), Rust backend, web frontend.
- **React + Vite + TypeScript** — frontend for the agent's tray window.
- **axum** — local HTTP listener.
- **windows-rs + SumatraPDF (runtime dep)** — see Windows Printing below.

## Windows printing uses bundled SumatraPDF

The agent dispatches PDFs to a named printer queue by shelling out to
**SumatraPDF** (free, GPLv3, ~6 MB). The MSI ships SumatraPDF.exe as
a Tauri resource so single-MSI installs work without any second
download — `find_sumatra()` probes the install dir first, then falls
back to PATH / Program Files / `%LOCALAPPDATA%\SumatraPDF\` for
operators who installed SumatraPDF separately.

Before building the MSI, fetch the bundled binary:

```sh
# macOS / Linux build host
./scripts/fetch-sumatra.sh
# Windows build host
pwsh ./scripts/fetch-sumatra.ps1
```

The script downloads SumatraPDF.exe into `src-tauri/resources/`
(gitignored). The license + source pointer live in
`src-tauri/resources/SumatraPDF-{LICENSE,NOTICE}.txt` so the MSI
satisfies the GPLv3 redistribution requirements as "mere
aggregation" — the two programs run as separate processes.

**Why not just use Edge / `printto`?** Microsoft Edge is the default
PDF handler on stock Win10/11 and **does not implement the `printto`
shell verb**. Calling `ShellExecute("printto", file, printerName)`
returns success but silently does nothing on stock installs. Adobe
Reader / Foxit DO implement `printto` if installed, but neither has a
documented "always available" CLI for printing to a specific queue
the way SumatraPDF does. SumatraPDF wins on reliability + size +
consistent CLI.

**v0.2 plan**: replace the SumatraPDF shell-out with in-process PDF
rendering via the [`pdfium-render`](https://github.com/ajrcarey/pdfium-render)
crate (BSD-3) — render each PDF page to a bitmap and submit via the
Win32 `WritePrinter` API. Removes the SumatraPDF install requirement,
adds ~8 MB to the agent binary, and gives us full control over scale,
bleed, and color management. Worth it for the multi-tenant product;
not worth blocking Casino Pier on.

## Prerequisites

- Rust toolchain — install via [rustup](https://rustup.rs/).
- Node.js 20+ and pnpm.
- Windows: the [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
  (any recent VS version works, just need the C++ workload).
- macOS: Xcode Command Line Tools (`xcode-select --install`).

## Develop

```sh
cd agent
pnpm install
pnpm tauri dev
```

Opens a dev window with hot reload. The HTTP listener binds to
`http://localhost:9988` automatically.

## Build a Windows installer

From a Windows machine (Tauri can cross-compile from macOS but signing /
notarisation is platform-specific so the supported path is build-on-target):

```sh
pnpm tauri build
```

Produces an unsigned `.msi` in `src-tauri/target/release/bundle/msi/`. For
production we'll add code signing (DigiCert EV cert) so SmartScreen
trusts the installer.

## Security model (v1)

- **Loopback only**: the listener binds `127.0.0.1:9988`. Nothing on
  the LAN can reach it.
- **CORS allowlist**: requests must come from one of the BadgeBadger
  origins (`https://ids.postudios.io`, `https://app.badgebadger.com`,
  `http://localhost:3000`). Override via `BADGEBADGER_AGENT_ORIGINS`.
  v0 used wide-open `Any` — a tab on any visited site could drive the
  printer.
- **Per-install bearer token**: random 256-bit hex, generated on
  first run and persisted at the OS app-data dir
  (`%LOCALAPPDATA%\com.badgebadger.printagent\token` /
  `~/Library/Application Support/com.badgebadger.printagent/token`,
  mode 0600 on unix). The agent UI shows it under "Pair with
  BadgeBadger"; the operator pastes it into the web app once.
  `/print` requires `Authorization: Bearer <token>`; `/health` stays
  unauthenticated so the web app can probe presence.
- **Origin pinning**: `/print` rejects requests with no `Origin`
  header or one outside the allowlist (defence in depth — closes
  curl-from-malware drive-by where the browser's own CORS check
  doesn't fire).
- **Rate limit**: token-bucket on `/print`, 60/min steady, burst 20.

## Test print without the web app

The new auth makes raw curl harder; this is the v1 incantation:

```sh
TOKEN=$(cat "$HOME/Library/Application Support/com.badgebadger.printagent/token")
curl -X POST http://localhost:9988/print \
  --data-binary "@badge.pdf" \
  -H "Content-Type: application/pdf" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Origin: http://localhost:3000"
```

Override the printer with `?printer=<queue-name>`:

```sh
curl -X POST 'http://localhost:9988/print?printer=Fargo%20HDP5000' \
  --data-binary "@badge.pdf" \
  -H "Content-Type: application/pdf" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Origin: http://localhost:3000"
```

## Roadmap

| Phase | Scope |
| --- | --- |
| **v1 (this repo)** | Local HTTP listener, system tray, Windows print integration. No pairing, no cloud. Operator can test print via curl. |
| v2 | Pair via 6-digit code with `https://app.badgebadger.com/api/agent/pair`. Persistent WebSocket from agent → server. Server routes print jobs over WS to the right tenant's agent. |
| v3 | Code signing (DigiCert / Apple Developer ID), auto-update via Tauri's built-in updater, install page on the marketing site. |
| v4 | Multiple printer profiles per agent, encoding (magstripe, RFID) integration, queue status reporting back to the web app. |
