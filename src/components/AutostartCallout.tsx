import { useEffect, useState } from "react";
import {
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { CheckIcon } from "./icons";

// First-run friendly affordance for the "launch on login" registration.
// The same toggle exists down in SettingsCard for tweaking later, but a
// brand-new operator's eye lands at the top of the window — they shouldn't
// have to scroll past three cards to discover that the agent can start
// itself. When autostart is already on we render a small inline "✓ Set
// to launch on login" pill instead of the card; the operator can still
// flip it off from SettingsCard if they ever need to.

export function AutostartCallout() {
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

  async function handleEnable() {
    setError(null);
    setPending(true);
    try {
      await enableAutostart();
      setAutostart(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPending(false);
    }
  }

  // Loading or autostart status unavailable — render nothing rather
  // than a skeleton card that pops in.
  if (autostart === null) return null;

  if (autostart) {
    return (
      <div className="autostart-pill">
        <span className="autostart-pill__check">
          <CheckIcon size={11} />
        </span>
        <span>Set to launch on login</span>
      </div>
    );
  }

  return (
    <section className="card autostart-callout">
      <div className="card__head">
        <h2 className="card__title">Auto-launch on login</h2>
        <p className="card__subtitle">
          Register the agent with your OS login so it&rsquo;s already
          running when an operator sits down in the morning. One click.
        </p>
      </div>
      <div className="card__actions">
        <button
          type="button"
          className="btn btn--primary"
          onClick={() => void handleEnable()}
          disabled={pending}
        >
          {pending ? "Enabling…" : "Enable launch on login"}
        </button>
      </div>
      {error ? <p className="autostart-callout__error">{error}</p> : null}
    </section>
  );
}
