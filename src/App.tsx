import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { StatusHero } from "./components/StatusHero";
import { ChecklistCard } from "./components/ChecklistCard";
import { PairingCard } from "./components/PairingCard";
import { PrinterCard } from "./components/PrinterCard";
import { ActivityCard } from "./components/ActivityCard";
import { SettingsCard } from "./components/SettingsCard";
import { Toaster, useToast } from "./components/Toaster";
import { UpdateBanner, useUpdater } from "./components/UpdateBanner";
import type { AgentStatus, PrintEvent, ActivityItem } from "./types";

export default function App() {
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [activity, setActivity] = useState<ActivityItem[]>([]);
  const [testing, setTesting] = useState(false);
  const [selectedPrinter, setSelectedPrinter] = useState<string | null>(null);
  const toast = useToast();
  const updater = useUpdater();

  // Poll status. Cheap (just a Tauri command, no IPC heavy lifting),
  // and 1s is fast enough that printer add/remove from the OS is
  // reflected in the UI within a moment without burning CPU.
  useEffect(() => {
    let cancelled = false;
    async function refresh() {
      try {
        const next = await invoke<AgentStatus>("get_status");
        if (!cancelled) setStatus(next);
      } catch {
        // Status fetch shouldn't fail in normal operation; if it
        // does, leave the previous status visible rather than
        // flicker to "unknown".
      }
    }
    void refresh();
    const interval = setInterval(() => void refresh(), 1000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  // Live activity feed: Rust emits a `print` event whenever a job
  // is dispatched (success or failure) to the local printer. We
  // append to an in-memory ring buffer and surface a toast.
  useEffect(() => {
    const unlistenPromise = listen<PrintEvent>("print", (event) => {
      const e = event.payload;
      const item: ActivityItem = {
        id: `${e.startedAt}-${Math.random().toString(36).slice(2, 8)}`,
        startedAt: e.startedAt,
        printer: e.printer,
        jobName: e.jobName ?? null,
        ok: e.ok,
        error: e.error ?? null,
      };
      setActivity((prev) => [item, ...prev].slice(0, 25));
      if (e.ok) {
        toast.show({
          tone: "success",
          title: "Sent to printer",
          body: `${e.printer}${e.jobName ? ` · ${e.jobName}` : ""}`,
        });
      } else {
        toast.show({
          tone: "error",
          title: "Print failed",
          body: e.error ?? "Unknown error",
        });
      }
    });
    return () => {
      void unlistenPromise.then((fn) => fn());
    };
  }, [toast]);

  async function handleTestPrint() {
    setTesting(true);
    try {
      const message = await invoke<string>("test_print", {
        printerName: selectedPrinter,
      });
      toast.show({
        tone: "success",
        title: "Test print queued",
        body: message,
      });
    } catch (e) {
      toast.show({
        tone: "error",
        title: "Test print failed",
        body: String(e),
      });
    } finally {
      setTesting(false);
    }
  }

  const checklist = useMemo(() => {
    if (!status) return [];
    return [
      {
        id: "listener",
        label: "Local listener running",
        done: status.listening,
        helpText: status.listening
          ? `Accepting print jobs on http://127.0.0.1:${status.listenerPort}.`
          : "The HTTP listener didn't start. Restart the agent and try again.",
      },
      {
        id: "helper",
        label: "PDF helper installed",
        done: status.helperInstalled,
        helpText: status.helperInstalled
          ? "SumatraPDF detected — the agent uses it to dispatch PDFs to your printer."
          : "Install SumatraPDF (https://www.sumatrapdfreader.org/download-free-pdf-viewer) — it's the bridge between PDFs and the Windows print spooler. Stock Windows uses Edge as the PDF handler, but Edge can't print to a specific queue from the command line. SumatraPDF can. Free, 6 MB, install via the official MSI.",
      },
      {
        id: "printer",
        label: "Printer detected",
        done: status.printers.length > 0,
        helpText:
          status.printers.length > 0
            ? `${status.printers.length} printer${
                status.printers.length === 1 ? "" : "s"
              } available.`
            : "No printers found. Plug in or install a printer driver.",
      },
      {
        id: "default",
        label: "Default printer set",
        done: !!status.defaultPrinter,
        helpText: status.defaultPrinter
          ? `${status.defaultPrinter} will receive jobs that don't specify a printer.`
          : "Open System Settings → Printers and right-click your card printer → Set as Default.",
      },
    ];
  }, [status]);

  const allReady = checklist.every((c) => c.done);

  return (
    <main className="shell">
      <header className="header">
        <span className="brand-mark" aria-hidden>
          🦡
        </span>
        <div>
          <h1>BadgeBadger Print Agent</h1>
          <p className="subtitle">
            Quietly forwards badge prints from BadgeBadger to your local
            printer.
          </p>
        </div>
      </header>

      <StatusHero
        ready={allReady}
        listening={status?.listening ?? false}
        defaultPrinter={status?.defaultPrinter ?? null}
        port={status?.listenerPort ?? null}
      />

      <UpdateBanner state={updater.state} onInstall={updater.install} />

      {!allReady && status ? (
        <ChecklistCard items={checklist} />
      ) : null}

      <PairingCard ready={!!status?.listening} />

      <PrinterCard
        printers={status?.printers ?? []}
        defaultPrinter={status?.defaultPrinter ?? null}
        selectedPrinter={selectedPrinter}
        onSelectPrinter={setSelectedPrinter}
        onTestPrint={() => void handleTestPrint()}
        testing={testing}
      />

      <ActivityCard items={activity} />

      <SettingsCard
        port={status?.listenerPort ?? null}
        listening={status?.listening ?? false}
        updaterState={updater.state}
        onCheckForUpdates={() => void updater.runCheck()}
      />

      <footer className="footer">
        <span>BadgeBadger Print Agent · v{__APP_VERSION__}</span>
        <span aria-hidden>·</span>
        <a
          href="https://hq.badgebadger.app/help/print-agent"
          target="_blank"
          rel="noopener noreferrer"
        >
          Help
        </a>
        <span aria-hidden>·</span>
        <span>Closing this window keeps the agent running in the tray.</span>
      </footer>

      <Toaster />
    </main>
  );
}
