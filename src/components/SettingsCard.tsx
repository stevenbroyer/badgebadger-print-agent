import { useEffect, useState } from "react";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { CopyIcon, CheckIcon } from "./icons";
import type { UpdaterState } from "./UpdateBanner";

type Props = {
  port: number | null;
  listening: boolean;
  updaterState: UpdaterState;
  onCheckForUpdates: () => void;
};

export function SettingsCard({
  port,
  listening,
  updaterState,
  onCheckForUpdates,
}: Props) {
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [pending, setPending] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let cancelled = false;
    isAutostartEnabled()
      .then((v) => {
        if (!cancelled) setAutostart(v);
      })
      .catch(() => {
        if (!cancelled) setAutostart(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  async function toggleAutostart() {
    if (autostart === null) return;
    setPending(true);
    try {
      if (autostart) {
        await disableAutostart();
        setAutostart(false);
      } else {
        await enableAutostart();
        setAutostart(true);
      }
    } finally {
      setPending(false);
    }
  }

  function copyEndpoint() {
    if (!port) return;
    void navigator.clipboard
      .writeText(`http://127.0.0.1:${port}/print`)
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 1400);
      });
  }

  return (
    <section className="card">
      <div className="card__head">
        <h2 className="card__title">Agent settings</h2>
        <p className="card__subtitle">
          Tweak how the agent behaves on this machine.
        </p>
      </div>

      <div className="settings">
        <div className="settings__row">
          <div>
            <p className="settings__label">Launch on login</p>
            <p className="settings__help">
              Start the agent automatically when you log into this computer
              so prints work straight away — no manual launch.
            </p>
          </div>
          <label className="switch">
            <input
              type="checkbox"
              checked={!!autostart}
              disabled={autostart === null || pending}
              onChange={() => void toggleAutostart()}
            />
            <span className="switch__track">
              <span className="switch__thumb" />
            </span>
          </label>
        </div>

        <div className="settings__row">
          <div>
            <p className="settings__label">Print endpoint</p>
            <p className="settings__help">
              The web app POSTs PDFs here. Useful for diagnosing connection
              issues — paste it into a curl request to test.
            </p>
          </div>
          <button
            type="button"
            className="btn btn--ghost"
            onClick={copyEndpoint}
            disabled={!listening || !port}
          >
            {copied ? <CheckIcon size={13} /> : <CopyIcon size={13} />}
            <span className="settings__endpoint">
              {port ? `127.0.0.1:${port}` : "—"}
            </span>
          </button>
        </div>

        <div className="settings__row">
          <div>
            <p className="settings__label">Updates</p>
            <p className="settings__help">
              {updaterStatusText(updaterState)}
            </p>
          </div>
          <button
            type="button"
            className="btn btn--ghost"
            onClick={onCheckForUpdates}
            disabled={
              updaterState.status === "checking" ||
              updaterState.status === "downloading"
            }
          >
            {updaterState.status === "checking" ? "Checking…" : "Check now"}
          </button>
        </div>
      </div>
    </section>
  );
}

function updaterStatusText(state: UpdaterState): string {
  switch (state.status) {
    case "idle":
      return "Auto-checks every 6 hours. Trigger one manually here.";
    case "checking":
      return "Talking to the BadgeBadger release server…";
    case "up-to-date":
      return `You're on the latest version (checked ${formatRelative(state.checkedAt)}).`;
    case "available":
      return `v${state.update.version} is ready — see the banner above.`;
    case "downloading":
      return `Installing v${state.update.version}…`;
    case "error":
      return `Last check failed: ${state.message.slice(0, 120)}`;
  }
}

function formatRelative(epochMs: number): string {
  const diff = Date.now() - epochMs;
  if (diff < 60_000) return "just now";
  const mins = Math.floor(diff / 60_000);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  return `${hours}h ago`;
}
