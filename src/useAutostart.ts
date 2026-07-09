import { useSyncExternalStore } from "react";
import {
  enable as enableAutostart,
  disable as disableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";

// Single source of truth for the "launch on login" state. Previously
// AutostartCallout and SettingsCard each held their own useState and called
// isEnabled() on mount independently — so enabling autostart from the callout
// left the Settings switch showing OFF until the window was relaunched. A
// module-level store read via useSyncExternalStore keeps every consumer in
// lock-step: one read on first subscribe, and any enable/disable propagates
// to all of them immediately.

type State = boolean | null; // null = unknown / still loading

let state: State = null;
let started = false;
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  if (!started) {
    started = true;
    isAutostartEnabled()
      .then((v) => {
        state = v;
        emit();
      })
      .catch(() => {
        state = null; // leave unknown; consumers render nothing
      });
  }
  return () => {
    listeners.delete(listener);
  };
}

function getSnapshot(): State {
  return state;
}

export function useAutostart() {
  const autostart = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
  return {
    autostart,
    // Both throw on failure so the caller can surface the error; state is
    // only advanced on success, then broadcast to every consumer.
    async enable() {
      await enableAutostart();
      state = true;
      emit();
    },
    async disable() {
      await disableAutostart();
      state = false;
      emit();
    },
  };
}
