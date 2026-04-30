import { useEffect, useState } from "react";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { CopyIcon, CheckIcon } from "./icons";

type Props = { port: number | null; listening: boolean };

export function SettingsCard({ port, listening }: Props) {
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
      </div>
    </section>
  );
}
