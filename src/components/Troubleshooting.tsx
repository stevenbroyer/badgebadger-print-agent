import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAutostart } from "../useAutostart";
import { CheckIcon, CopyIcon } from "./icons";
import type { UpdaterState } from "./UpdateBanner";
import type { AgentStatus } from "../types";

// Everything a front desk rarely needs, folded away: starting at sign-in,
// updates, the manual pairing code, and the details IT asks for on a
// support call. Nothing in here is needed for an everyday print.

type Props = {
  status: AgentStatus | null;
  updaterState: UpdaterState;
  onCheckForUpdates: () => void;
  onRecheck: () => void;
};

function updateText(state: UpdaterState): string {
  switch (state.status) {
    case "checking":
      return "Checking for updates…";
    case "up-to-date":
      return "Up to date.";
    case "available":
      return `Version ${state.update.version} is ready to install.`;
    case "downloading":
      return `Installing version ${state.update.version}…`;
    case "error":
      return "Couldn't check for updates just now.";
    default:
      return "";
  }
}

const CONNECTION_TEXT = {
  checking: "checking",
  connected: "connected",
  notPaired: "not paired",
  offline: "unreachable",
} as const;

export function Troubleshooting({ status, updaterState, onCheckForUpdates, onRecheck }: Props) {
  const { autostart, enable, disable } = useAutostart();
  const [pending, setPending] = useState(false);
  const [token, setToken] = useState<string | null>(null);
  const [revealed, setRevealed] = useState(false);
  const [copied, setCopied] = useState(false);
  const [open, setOpen] = useState(false);

  // The pairing code is only fetched once someone opens this section.
  useEffect(() => {
    if (!open || token || !status?.listening) return;
    let cancelled = false;
    invoke<string>("get_pairing_token")
      .then((t) => {
        if (!cancelled) setToken(t);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [open, token, status?.listening]);

  async function toggleAutostart() {
    if (autostart === null) return;
    setPending(true);
    try {
      await (autostart ? disable() : enable());
    } finally {
      setPending(false);
    }
  }

  function copyToken() {
    if (!token) return;
    void navigator.clipboard.writeText(token).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    });
  }

  return (
    <details className="drawer" onToggle={(e) => setOpen((e.target as HTMLDetailsElement).open)}>
      <summary className="drawer__summary">Settings and troubleshooting</summary>

      <div className="drawer__body">
        <div className="row">
          <div className="row__copy">
            <p className="row__label" id="autostart-label">
              Start when I sign in
            </p>
            <p className="row__help">So badges print after a restart without anyone opening the agent.</p>
          </div>
          <label className="switch">
            <input
              type="checkbox"
              role="switch"
              aria-labelledby="autostart-label"
              checked={!!autostart}
              disabled={autostart === null || pending}
              onChange={() => void toggleAutostart()}
            />
            <span className="switch__track">
              <span className="switch__thumb" />
            </span>
          </label>
        </div>

        <div className="row">
          <div className="row__copy">
            <p className="row__label">Version {__APP_VERSION__}</p>
            <p className="row__help">{updateText(updaterState)}</p>
          </div>
          <button
            type="button"
            className="btn btn--secondary"
            onClick={onCheckForUpdates}
            disabled={updaterState.status === "checking" || updaterState.status === "downloading"}
          >
            Check for updates
          </button>
        </div>

        <div className="row row--stack">
          <div className="row__copy">
            <p className="row__label">Pairing code</p>
            <p className="row__help">
              Only needed to print from a browser on another computer. Paste it in BadgeBadger under Settings →
              Printers → Print agents.
            </p>
          </div>
          <div className="code">
            <code className="code__value">
              {token === null ? "…" : revealed ? token : "•".repeat(24)}
            </code>
            <button type="button" className="btn btn--link" onClick={() => setRevealed((v) => !v)} disabled={!token}>
              {revealed ? "Hide" : "Show"}
            </button>
            <button type="button" className="btn btn--link" onClick={copyToken} disabled={!token}>
              {copied ? <CheckIcon size={12} /> : <CopyIcon size={12} />}
              {copied ? "Copied" : "Copy"}
            </button>
          </div>
        </div>

        <div className="row row--stack">
          <div className="row__copy">
            <p className="row__label">Details for IT</p>
          </div>
          <dl className="facts">
            <dt>Computer</dt>
            <dd>{status?.computerName ?? "…"}</dd>
            <dt>Local port</dt>
            <dd>
              {status ? `127.0.0.1:${status.listenerPort || 9988}, ${status.listening ? "listening" : "not listening"}` : "…"}
            </dd>
            <dt>BadgeBadger</dt>
            <dd>{status ? CONNECTION_TEXT[status.connection] : "…"}</dd>
            <dt>Default printer</dt>
            <dd>{status?.osDefaultPrinter ?? "none"}</dd>
            {status && !status.helperInstalled ? (
              <>
                <dt>PDF helper</dt>
                <dd>missing (SumatraPDF)</dd>
              </>
            ) : null}
          </dl>
          <button type="button" className="btn btn--link" onClick={onRecheck}>
            Check the connection again
          </button>
        </div>
      </div>
    </details>
  );
}
