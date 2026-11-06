# Changelog

## 0.7.2

- Pairing works on a computer whose name has lower case letters or is
  longer than 15 characters. The pairing link and the agent's status check
  now use the same name, so BadgeBadger no longer asks to pair a computer
  that is already paired.

## 0.7.1

- A test print shows one notice instead of two. The Test print button
  reports its own result, and the print it starts no longer adds a second
  "Sent to the printer" notice on top.

## 0.7.0

Windows pass and a new window. Existing installs get it from the in-app
update banner.

### New window
- **One status card, drawn as the computer's own badge** on a lanyard,
  with the badger as its photo. It says whether badges will print and, if
  not, the one thing to do: pair this computer, plug in a printer, restart
  the agent. A colour band and light show the state, the badger waits in
  grey until things are ready, and the computer's name sits where a badge
  number would go. The tray icon's tooltip says the same thing.
- **It knows whether this computer is paired.** The agent already polls
  BadgeBadger; the card now shows what BadgeBadger said ("Connected to
  BadgeBadger", "Not connected yet", "Can't reach BadgeBadger"). Coming
  back to the window after pairing in the browser checks again right away.
- **Badge printer.** Pick the printer badges go to when BadgeBadger doesn't
  name one, instead of changing the computer's default (usually the office
  paper printer). Card printers are recognised by name, listed first, and
  offered with "Use it for badges". The choice is saved in settings.json;
  /health reports it as the default so the web app shows where badges go.
- **Plain language.** The port, the pairing code, update checks and the
  other details IT asks for moved into "Settings and troubleshooting".
- **Recent prints** shows the last three badges, with a failed one's reason
  in plain words.
- The web app's own fonts (Outfit and DM Sans, bundled under the OFL) and
  a broken Help link fixed: it now opens the troubleshooting guide.

### Changed
- **The Windows download is 12 MB instead of 216 MB.** The installer used to
  carry Microsoft's full WebView2 runtime, which Windows 10 and 11 already
  have. It now carries the small WebView2 setup that runs only on a PC
  that's missing it, which needs an internet connection.
- **Opening the agent while it's running shows the window it already has.**
  A second launch used to start another copy that couldn't get port 9988,
  so it said "Agent stopped" next to the copy that was working.
- **On Windows the agent starts in the tray at sign-in** instead of opening
  its window every morning. Opening it from the Start menu still shows the
  window. Installs from before 0.7.0 switch over on their next launch,
  unless launch on login is off.
- **Signed Windows builds, once Azure Artifact Signing is set up.** CI signs
  the app, the installer, the uninstaller and the MSI, and fails the build if
  a signature doesn't check out. Without the settings it builds unsigned, as
  before.

### Fixed
- Reinstalling turns start-at-sign-in back on. Uninstalling removed the
  sign-in entry but left the agent's note that it had set it up once, so a
  reinstall never re-enabled it. The uninstaller now clears the note too.
  Updates never touched either.
- A printer with a long name no longer adds a sideways scrollbar to the
  window.
- The maximize button is off; the window has a fixed size.

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
