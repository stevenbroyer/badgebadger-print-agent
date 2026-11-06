import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openExternal } from "@tauri-apps/plugin-shell";
import { relaunch } from "@tauri-apps/plugin-process";
import { StationCard, type StationAction, type StationTone } from "./components/StationCard";
import { BadgePrinter } from "./components/BadgePrinter";
import { RecentPrints } from "./components/RecentPrints";
import { Troubleshooting } from "./components/Troubleshooting";
import { Toaster, useToast } from "./components/Toaster";
import { UpdateBanner, useUpdater } from "./components/UpdateBanner";
import { useAutostart } from "./useAutostart";
import type { AgentStatus, PrintEvent, ActivityItem } from "./types";

const WEB_APP = "https://hq.badgebadger.app";
const INSTALL_GUIDE = "https://badgebadger.app/docs/install-print-agent";
const HELP = "https://badgebadger.app/docs/troubleshooting";

type Station = {
  tone: StationTone;
  headline: string;
  body: string;
  action?: StationAction | undefined;
};

export default function App() {
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [activity, setActivity] = useState<ActivityItem[]>([]);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [pairing, setPairing] = useState(false);
  const toast = useToast();
  const updater = useUpdater();
  const { autostart, enable: enableAutostart } = useAutostart();

  const refresh = useCallback(async () => {
    try {
      setStatus(await invoke<AgentStatus>("get_status"));
    } catch {
      // Status fetch shouldn't fail in normal operation; if it does,
      // leave the previous status visible rather than flicker.
    }
  }, []);

  // Poll status. Cheap (just a Tauri command), and 1s is fast enough that
  // a printer plugged in, or a pairing finished in the browser, shows up
  // within a moment.
  useEffect(() => {
    void refresh();
    const interval = setInterval(() => void refresh(), 1000);
    return () => clearInterval(interval);
  }, [refresh]);

  // Coming back to the window, usually from pairing in the browser: ask
  // BadgeBadger again now instead of after the poll loop's backoff.
  const connection = status?.connection ?? null;
  useEffect(() => {
    function onFocus() {
      if (connection !== "connected") void invoke("recheck_connection");
    }
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [connection]);

  // Say so once when a pairing started here goes through.
  const wasConnected = useRef<boolean | null>(null);
  useEffect(() => {
    if (connection === null || connection === "checking") return;
    const now = connection === "connected";
    if (wasConnected.current === false && now) {
      setPairing(false);
      toast.show({ tone: "success", title: "Connected to BadgeBadger", body: "Badges can print here now." });
    }
    wasConnected.current = now;
  }, [connection, toast]);

  // Live activity feed: Rust emits a `print` event whenever a job is
  // dispatched (success or failure) to the local printer.
  useEffect(() => {
    const unlistenPromise = listen<PrintEvent>("print", (event) => {
      const e = event.payload;
      const item: ActivityItem = {
        id: `${e.startedAt}-${Math.random().toString(36).slice(2, 8)}`,
        startedAt: e.startedAt,
        printer: e.printer,
        jobName: e.jobName ?? null,
        employeeName: e.employeeName ?? null,
        templateName: e.templateName ?? null,
        ok: e.ok,
        error: e.error ?? null,
      };
      setActivity((prev) => [item, ...prev].slice(0, 25));
      if (e.ok) {
        toast.show({
          tone: "success",
          title: e.employeeName ? `Printed ${e.employeeName}` : "Sent to the printer",
          body: [e.employeeName ? e.templateName : e.jobName, e.printer].filter(Boolean).join(" · "),
        });
      } else {
        toast.show({ tone: "error", title: "That badge didn't print", body: e.error ?? "Unknown error" });
      }
    });
    return () => {
      void unlistenPromise.then((fn) => fn());
    };
  }, [toast]);

  async function handleTestPrint() {
    setTesting(true);
    try {
      await invoke<string>("test_print", { printerName: null });
      toast.show({
        tone: "success",
        title: "Test card sent",
        ...(status?.defaultPrinter ? { body: status.defaultPrinter } : {}),
      });
    } catch (e) {
      toast.show({ tone: "error", title: "The test card didn't print", body: String(e) });
    } finally {
      setTesting(false);
    }
  }

  async function handleChoosePrinter(printer: string | null) {
    setSaving(true);
    try {
      await invoke("set_badge_printer", { printerName: printer });
      await refresh();
      toast.show({
        tone: "success",
        title: "Badge printer saved",
        body: printer ? `Badges print to ${printer}.` : "Badges print to this computer's default printer.",
      });
    } catch (e) {
      toast.show({ tone: "error", title: "Couldn't save that", body: String(e) });
    } finally {
      setSaving(false);
    }
  }

  async function handleConnect() {
    try {
      await openExternal(await invoke<string>("get_pair_url"));
      setPairing(true);
    } catch (e) {
      toast.show({ tone: "error", title: "Couldn't open your browser", body: String(e) });
    }
  }

  function openWebApp() {
    void openExternal(WEB_APP).catch(() => undefined);
  }

  // The relaunch inherits --hidden from a sign-in start; ask for the window.
  async function restartShowingWindow() {
    await invoke("show_window_on_next_start").catch(() => undefined);
    await relaunch();
  }

  const printsToday = useMemo(() => {
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    return activity.filter((a) => a.ok && new Date(a.startedAt).getTime() >= start.getTime()).length;
  }, [activity]);

  // One state at a time, most fundamental first: a problem that stops
  // printing outranks one that only stops some of it.
  const station: Station = (() => {
    if (!status) return { tone: "neutral", headline: "Starting…", body: "One moment." };
    if (!status.listening) {
      return {
        tone: "problem",
        headline: "Not running",
        body: "Another program may be using the agent's connection. Restarting the agent usually fixes it.",
        action: { label: "Restart the agent", onClick: () => void restartShowingWindow(), primary: true },
      };
    }
    if (!status.helperInstalled) {
      return {
        tone: "problem",
        headline: "Part of the agent is missing",
        body: "Its printing helper didn't install. Download the agent again and reinstall it.",
        action: {
          label: "Download the agent",
          onClick: () => void openExternal(INSTALL_GUIDE).catch(() => undefined),
          primary: true,
        },
      };
    }
    if (status.connection === "notPaired") {
      return pairing
        ? {
            tone: "attention",
            headline: "Finish in your browser",
            body: "Sign in to BadgeBadger and confirm this computer. This window updates by itself.",
            action: { label: "Open the page again", onClick: () => void handleConnect() },
          }
        : {
            tone: "attention",
            headline: "Not connected yet",
            body: "Pair it with your BadgeBadger account so badges can print here.",
            // The docs and the web app's confirm dialog both say "Pair this computer".
            action: { label: "Pair this computer", onClick: () => void handleConnect(), primary: true },
          };
    }
    if (status.printers.length === 0) {
      return {
        tone: "attention",
        headline: "No printer found",
        body: "Plug in your badge printer and install its driver. It shows up here by itself.",
      };
    }
    if (status.connection === "offline") {
      return {
        tone: "attention",
        headline: "Can't reach BadgeBadger",
        body: "Check this computer's internet connection. The agent keeps trying on its own.",
        action: { label: "Try again", onClick: () => void invoke("recheck_connection") },
      };
    }
    if (!status.defaultPrinter) {
      return {
        tone: "attention",
        headline: "Pick a badge printer",
        body: "Choose where badges print, under Badge printer below.",
      };
    }
    if (status.connection === "checking") {
      return { tone: "neutral", headline: "Checking the connection…", body: "Making sure BadgeBadger can reach this computer." };
    }
    return { tone: "ready", headline: "Ready to print", body: `Badges print to ${status.defaultPrinter}.` };
  })();

  return (
    <main className="shell">
      <header className="topbar">
        <span className="coin" aria-hidden>
          <img src="/mascot.svg" alt="" />
        </span>
        <p className="wordmark">
          <span className="wordmark__badge">Badge</span>Badger <span className="wordmark__product">Print Agent</span>
        </p>
        <button type="button" className="btn btn--link topbar__open" onClick={openWebApp}>
          Open BadgeBadger <span aria-hidden>↗</span>
        </button>
      </header>

      <UpdateBanner state={updater.state} onInstall={updater.install} />

      <StationCard
        tone={station.tone}
        headline={station.headline}
        body={station.body}
        action={station.action}
        computerName={status?.computerName ?? null}
        connection={status?.listening ? status.connection : null}
        printsToday={printsToday}
      />

      {autostart === false ? (
        <div className="notice" role="note">
          <p>The agent won&rsquo;t start by itself after a restart.</p>
          <button type="button" className="btn btn--link" onClick={() => void enableAutostart().catch(() => undefined)}>
            Turn on
          </button>
        </div>
      ) : null}

      {status?.listening ? (
        <BadgePrinter
          printers={status.printers}
          badgePrinter={status.badgePrinter}
          osDefaultPrinter={status.osDefaultPrinter}
          saving={saving}
          testing={testing}
          onChoose={(p) => void handleChoosePrinter(p)}
          onTestPrint={() => void handleTestPrint()}
        />
      ) : null}

      <RecentPrints items={activity} />

      <Troubleshooting
        status={status}
        updaterState={updater.state}
        onCheckForUpdates={() => void updater.runCheck()}
        onRecheck={() => void invoke("recheck_connection")}
      />

      <footer className="footer">
        <span>Closing this window keeps the agent running in the tray.</span>
        <button type="button" className="btn btn--link" onClick={() => void openExternal(HELP).catch(() => undefined)}>
          Help
        </button>
      </footer>

      <Toaster />
    </main>
  );
}
