// Tiny in-house toast system. Lighter than pulling in sonner since
// the agent only ever shows 3-4 toast types and we want the binary
// surface as small as possible.
import { useEffect, useState } from "react";
import { CheckIcon, XIcon, InfoIcon } from "./icons";

export type ToastTone = "success" | "error" | "info";

export type ToastSpec = {
  tone: ToastTone;
  title: string;
  body?: string;
  // duration in ms; defaults to 4000 for info/success, 6000 for error
  duration?: number;
};

type Toast = ToastSpec & { id: string; createdAt: number };

// Module-level event bus — components mount the <Toaster /> once at
// the app root, and any component can call `useToast().show(...)` to
// display a toast. Avoids prop-drilling a context provider for the
// few callsites we have.
const listeners = new Set<(t: Toast) => void>();
let nextId = 0;

export function useToast() {
  return {
    show(spec: ToastSpec) {
      const t: Toast = {
        ...spec,
        id: `toast-${++nextId}`,
        createdAt: Date.now(),
      };
      listeners.forEach((cb) => cb(t));
    },
  };
}

export function Toaster() {
  const [toasts, setToasts] = useState<Toast[]>([]);

  useEffect(() => {
    function add(t: Toast) {
      setToasts((prev) => [...prev, t]);
      const ms = t.duration ?? (t.tone === "error" ? 6000 : 4000);
      setTimeout(() => {
        setToasts((prev) => prev.filter((x) => x.id !== t.id));
      }, ms);
    }
    listeners.add(add);
    return () => {
      listeners.delete(add);
    };
  }, []);

  return (
    <div className="toaster" aria-live="polite" aria-atomic="false">
      {toasts.map((t) => (
        <div
          key={t.id}
          role="status"
          className={`toast toast--${t.tone}`}
        >
          <span className="toast__icon" aria-hidden>
            {t.tone === "success" ? (
              <CheckIcon size={14} />
            ) : t.tone === "error" ? (
              <XIcon size={14} />
            ) : (
              <InfoIcon size={14} />
            )}
          </span>
          <div className="toast__copy">
            <p className="toast__title">{t.title}</p>
            {t.body ? <p className="toast__body">{t.body}</p> : null}
          </div>
        </div>
      ))}
    </div>
  );
}
