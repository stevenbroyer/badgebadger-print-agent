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

## Windows printing requires SumatraPDF (v1)

The agent dispatches PDFs to a named printer queue by shelling out to
**SumatraPDF**. Operators have to install it once per workstation:
<https://www.sumatrapdfreader.org/download-free-pdf-viewer> (free,
GPLv3, ~6 MB single MSI). The agent UI surfaces this as a setup
checklist item — green when SumatraPDF is detected on PATH or in any
of the standard install dirs (`C:\Program Files\SumatraPDF\`, `C:\Program Files (x86)\SumatraPDF\`, `%LOCALAPPDATA%\SumatraPDF\`).

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

## Test print without the web app

While the agent is running:

```sh
curl -X POST http://localhost:9988/print --data-binary "@badge.pdf" -H "Content-Type: application/pdf"
```

The agent saves the PDF to a temp file and shells out to the OS default
PDF handler with the `printto` verb pointed at the OS default printer.
Override the printer with `?printer=<queue-name>`:

```sh
curl -X POST 'http://localhost:9988/print?printer=Fargo%20HDP5000' --data-binary "@badge.pdf" -H "Content-Type: application/pdf"
```

## Roadmap

| Phase | Scope |
| --- | --- |
| **v1 (this repo)** | Local HTTP listener, system tray, Windows print integration. No pairing, no cloud. Operator can test print via curl. |
| v2 | Pair via 6-digit code with `https://app.badgebadger.com/api/agent/pair`. Persistent WebSocket from agent → server. Server routes print jobs over WS to the right tenant's agent. |
| v3 | Code signing (DigiCert / Apple Developer ID), auto-update via Tauri's built-in updater, install page on the marketing site. |
| v4 | Multiple printer profiles per agent, encoding (magstripe, RFID) integration, queue status reporting back to the web app. |
