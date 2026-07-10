# Changelog

## 0.6.0 — 2026-07-10

Brand + security + reliability pass. Existing installs update silently via the
signed auto-updater (no SmartScreen prompt — that only affects fresh installs).

### Added / changed
- **Rebranded to the BadgeBadger design system** — Brand Blue + ivory + slate,
  a real light *and* dark theme (was dark-only), the actual badger mascot in a
  blue coin, and the two-tone wordmark. Flat cards, no more generic glass.
- **Calm hero-first layout** — a Ready mode that foregrounds recent activity
  and a Setup mode that foregrounds the steps to a first print, instead of one
  long scroll of cards. Humanized the status line (names your printer, not a
  localhost URL).

### Fixed
- **Bulk prints over ~2 MB no longer fail (HTTP 413).** The 25 MB body limit
  was silently capped at axum's 2 MB default.
- **Cloud print jobs are hardened** — the agent now requires https, refuses
  private/reserved download hosts (SSRF), enforces a streamed 25 MB cap
  (OOM protection), and re-verifies the `%PDF-` signature before printing.
- **`/health` can no longer be flooded to stall the agent** — its printer
  probes moved off the async runtime.
- **Autostart toggle no longer desyncs** between the setup callout and Settings.

### Security / compliance
- Bundled SumatraPDF is now SHA-256-pinned and Authenticode-checked at build.
- Full GPL-3.0 license text is now conveyed with the bundled SumatraPDF.

### Known / next
- Windows installer is not yet Authenticode-signed — fresh installs still show
  SmartScreen until code signing is set up. Existing-install updates are
  unaffected.
- Status still reflects local listener state only (pairing/cloud "truth" model
  is in progress).
