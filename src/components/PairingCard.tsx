import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openExternal } from "@tauri-apps/plugin-shell";
import { CheckIcon, CopyIcon } from "./icons";

// Surfaces the agent's pairing token so the operator can paste it
// into the BadgeBadger web app at /settings/printers/agents. The
// agent generates a random 256-bit token on first run; pasting it
// into the web app authorises that workstation to drive this
// agent's printer. Without it, the local listener rejects /print
// with 401.
//
// Token starts masked. Click "Reveal" to see it, "Copy" to put it
// on the clipboard. Clicking either of those is enough — the operator
// then heads to the web app, opens settings → printers → agents →
// "Pair workstation", pastes, saves. Once paired, the token sits in
// localStorage on that browser; future prints carry it as a Bearer
// header automatically.

type Props = {
  ready: boolean;
};

export function PairingCard({ ready }: Props) {
  const [token, setToken] = useState<string | null>(null);
  const [revealed, setRevealed] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!ready) return;
    let cancelled = false;
    invoke<string>("get_pairing_token")
      .then((t) => {
        if (!cancelled) setToken(t);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [ready]);

  function copyToken() {
    if (!token) return;
    void navigator.clipboard.writeText(token).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    });
  }

  async function openPairUrl() {
    try {
      const url = await invoke<string>("get_pair_url");
      await openExternal(url);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="card">
      <div className="card__head">
        <h2 className="card__title">Pair with BadgeBadger</h2>
        <p className="card__subtitle">
          One-click pair: opens the BadgeBadger web app pre-filled
          with this computer&rsquo;s identity — sign in (if you
          aren&rsquo;t already) and confirm. The legacy paste-token
          flow is below if your browser&rsquo;s on another machine.
        </p>
      </div>

      <div className="pair__actions" style={{ marginBottom: 12 }}>
        <button
          type="button"
          className="btn btn--primary"
          onClick={() => void openPairUrl()}
          disabled={!token}
        >
          Pair this computer ↗
        </button>
      </div>

      <details className="pair__advanced">
        <summary>Use a different browser / manual paste</summary>
        <div className="pair">
          <div
            className={
              revealed ? "pair__token pair__token--revealed" : "pair__token"
            }
            aria-label={revealed ? "Pairing token (revealed)" : "Pairing token (hidden)"}
          >
            {token === null
              ? error
                ? "—"
                : "Loading…"
              : revealed
                ? token
                : "•".repeat(token.length)}
          </div>
          <div className="pair__actions">
            <button
              type="button"
              className="btn btn--ghost"
              onClick={() => setRevealed((v) => !v)}
              disabled={!token}
            >
              {revealed ? "Hide" : "Reveal"}
            </button>
            <button
              type="button"
              className="btn btn--ghost"
              onClick={copyToken}
              disabled={!token}
            >
              {copied ? <CheckIcon size={13} /> : <CopyIcon size={13} />}
              {copied ? "Copied" : "Copy"}
            </button>
          </div>
          <p className="pair__hint">
            Paste at <strong>Settings → Printers → Agents → Pair workstation</strong>.
          </p>
        </div>
      </details>

      {error ? (
        <p className="pair__error">Couldn&rsquo;t load the token: {error}</p>
      ) : null}
    </section>
  );
}
