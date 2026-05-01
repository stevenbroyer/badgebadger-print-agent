import { useCallback, useEffect, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { ZapIcon } from "./icons";

// Auto-update state machine. The backend (tauri-plugin-updater) handles
// HTTP fetches + signature verification; this hook just sequences UI
// states and exposes triggers.
export type UpdaterState =
  | { status: "idle" }
  | { status: "checking" }
  | { status: "up-to-date"; checkedAt: number }
  | { status: "available"; update: UpdateMeta }
  | { status: "downloading"; update: UpdateMeta; downloaded: number; total: number | null }
  | { status: "error"; message: string };

// Pulled out of `Update` for the hot path so we don't keep a live
// handle to the plugin object in render-time state. The plugin stores
// the actual handle internally; we only need version + notes for UI.
type UpdateMeta = {
  version: string;
  currentVersion: string;
  date: string | null;
  body: string | null;
  handle: Update;
};

const SIX_HOURS_MS = 6 * 60 * 60 * 1000;

export function useUpdater() {
  const [state, setState] = useState<UpdaterState>({ status: "idle" });

  const runCheck = useCallback(async () => {
    setState({ status: "checking" });
    try {
      const update = await check();
      if (!update) {
        setState({ status: "up-to-date", checkedAt: Date.now() });
        return;
      }
      setState({
        status: "available",
        update: {
          version: update.version,
          currentVersion: update.currentVersion,
          date: update.date ?? null,
          body: update.body ?? null,
          handle: update,
        },
      });
    } catch (e) {
      setState({ status: "error", message: String(e) });
    }
  }, []);

  // Check on mount + every 6 h.
  useEffect(() => {
    void runCheck();
    const id = setInterval(() => void runCheck(), SIX_HOURS_MS);
    return () => clearInterval(id);
  }, [runCheck]);

  const install = useCallback(async () => {
    if (state.status !== "available") return;
    const meta = state.update;
    setState({ status: "downloading", update: meta, downloaded: 0, total: null });
    try {
      await meta.handle.downloadAndInstall((event) => {
        // Tauri 2 emits Started / Progress / Finished events.
        if (event.event === "Started") {
          setState({
            status: "downloading",
            update: meta,
            downloaded: 0,
            total: event.data.contentLength ?? null,
          });
        } else if (event.event === "Progress") {
          setState((prev) =>
            prev.status === "downloading"
              ? {
                  ...prev,
                  downloaded: prev.downloaded + event.data.chunkLength,
                }
              : prev,
          );
        }
      });
      // On Windows the NSIS installer relaunches us; relaunch() is a
      // safety net for platforms / cases where it doesn't.
      await relaunch();
    } catch (e) {
      setState({ status: "error", message: String(e) });
    }
  }, [state]);

  return { state, runCheck, install };
}

type Props = {
  state: UpdaterState;
  onInstall: () => Promise<void> | void;
};

export function UpdateBanner({ state, onInstall }: Props) {
  // Render only when there's something to communicate. "checking",
  // "idle", and "up-to-date" stay silent — the SettingsCard surface
  // covers the manual-trigger feedback for those cases.
  if (state.status === "available") {
    return (
      <section className="update-banner update-banner--available" role="status">
        <div className="update-banner__icon" aria-hidden>
          <ZapIcon size={16} />
        </div>
        <div className="update-banner__copy">
          <p className="update-banner__title">
            Update available · v{state.update.version}
          </p>
          <p className="update-banner__body">
            {state.update.body?.trim()
              ? truncate(state.update.body, 140)
              : "A newer agent is ready. Restart to apply — should take a few seconds."}
          </p>
        </div>
        <div className="update-banner__actions">
          <button
            type="button"
            className="btn btn--primary"
            onClick={() => void onInstall()}
          >
            Restart now
          </button>
        </div>
      </section>
    );
  }

  if (state.status === "downloading") {
    const pct =
      state.total && state.total > 0
        ? Math.min(100, Math.round((state.downloaded / state.total) * 100))
        : null;
    return (
      <section className="update-banner update-banner--available" role="status">
        <div className="update-banner__icon" aria-hidden>
          <ZapIcon size={16} />
        </div>
        <div className="update-banner__copy">
          <p className="update-banner__title">
            Installing v{state.update.version}…
          </p>
          <p className="update-banner__body">
            {pct !== null
              ? `Downloaded ${pct}%. The agent will restart automatically.`
              : "Downloading… The agent will restart automatically."}
          </p>
        </div>
      </section>
    );
  }

  // Errors are intentionally not rendered here. The first time the
  // agent runs against a fresh repo (before any release has been
  // published) the manifest URL 404s — we don't want operators to
  // see an alarming red banner for an infrastructure state they
  // can't act on. Errors stay visible in SettingsCard's "Updates"
  // row text (and in tracing logs) so IT / developers can debug.

  return null;
}

function truncate(s: string, max: number): string {
  if (s.length <= max) return s;
  return s.slice(0, max - 1).trimEnd() + "…";
}
