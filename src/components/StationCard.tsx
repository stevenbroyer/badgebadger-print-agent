import { PrinterIcon } from "./icons";
import type { Connection } from "../types";

// The top of the window, drawn as this computer's own ID badge: a slot
// punch, a colour band that says how things are (the way badge bands mark
// staff or visitor), a "photo" with the status light, and the computer's
// name where a badge number would go. It answers the one question people
// open the window with: will my badges print, and if not, what do I do?

export type StationTone = "ready" | "attention" | "problem" | "neutral";

export type StationAction = {
  label: string;
  onClick: () => void;
  primary?: boolean;
  busy?: boolean;
};

type Props = {
  tone: StationTone;
  headline: string;
  body: string;
  action?: StationAction | undefined;
  computerName: string | null;
  connection: Connection | null;
  printsToday: number;
};

const CONNECTION_LABEL: Record<Connection, string> = {
  checking: "Checking connection",
  connected: "Connected to BadgeBadger",
  notPaired: "Not connected to BadgeBadger",
  offline: "Can't reach BadgeBadger",
};

export function StationCard({
  tone,
  headline,
  body,
  action,
  computerName,
  connection,
  printsToday,
}: Props) {
  return (
    <section className={`station station--${tone}`} aria-labelledby="station-headline">
      <div className="station__band" aria-hidden>
        <span className="station__punch" />
      </div>

      <div className="station__main">
        <div className="station__photo" aria-hidden>
          <PrinterIcon size={26} strokeWidth={1.5} />
          <span className="station__light" />
        </div>
        <div className="station__copy">
          {/* Polite live region: a screen reader hears "Not connected
              yet" turn into "Ready to print" without hunting for it. */}
          <h1 id="station-headline" className="station__headline" aria-live="polite">
            {headline}
          </h1>
          <p className="station__body">{body}</p>
          {action ? (
            <button
              type="button"
              className={action.primary ? "btn btn--primary" : "btn btn--secondary"}
              onClick={action.onClick}
              disabled={action.busy}
            >
              {action.label}
            </button>
          ) : null}
        </div>
      </div>

      <footer className="station__id">
        <span className="station__name" title="This computer's name">
          {computerName ?? "…"}
        </span>
        <span className="station__facts">
          {connection ? CONNECTION_LABEL[connection] : ""}
          {printsToday > 0 ? (
            <>
              <span aria-hidden> · </span>
              {printsToday} printed today
            </>
          ) : null}
        </span>
      </footer>
    </section>
  );
}
