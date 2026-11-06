import { useId, useState } from "react";
import { PrinterIcon } from "./icons";
import { printerKind, sortPrinters } from "../printers";

// Which printer badges come out of. BadgeBadger can name a printer per
// print; when it doesn't, the agent uses the printer picked here, and
// without one, the computer's default. On a shared office PC the default
// is usually the paper printer, hence the nudge toward a card printer.

type Props = {
  printers: string[];
  badgePrinter: string | null;
  osDefaultPrinter: string | null;
  saving: boolean;
  testing: boolean;
  onChoose: (printer: string | null) => void;
  onTestPrint: () => void;
};

const KIND_LABEL = { card: "Card printer", office: null, virtual: null } as const;

export function BadgePrinter({
  printers,
  badgePrinter,
  osDefaultPrinter,
  saving,
  testing,
  onChoose,
  onTestPrint,
}: Props) {
  const [open, setOpen] = useState(false);
  const listId = useId();

  if (printers.length === 0) return null;

  const chosenInstalled = !!badgePrinter && printers.includes(badgePrinter);
  const target = chosenInstalled ? badgePrinter : osDefaultPrinter;
  const cardPrinters = printers.filter((p) => printerKind(p) === "card");
  const suggestion =
    !badgePrinter && cardPrinters.length > 0 && !cardPrinters.includes(osDefaultPrinter ?? "")
      ? cardPrinters[0]
      : null;

  function choose(printer: string | null) {
    setOpen(false);
    onChoose(printer);
  }

  return (
    <section className="section" aria-labelledby="badge-printer-title">
      <div className="section__head">
        <h2 id="badge-printer-title" className="section__title">
          Badge printer
        </h2>
        <div className="section__links">
          <button type="button" className="btn btn--link" onClick={onTestPrint} disabled={testing || !target}>
            {testing ? "Sending…" : "Print a test card"}
          </button>
          <button
            type="button"
            className="btn btn--link"
            aria-expanded={open}
            aria-controls={listId}
            onClick={() => setOpen((v) => !v)}
          >
            {open ? "Done" : "Change"}
          </button>
        </div>
      </div>

      <div className="chosen">
        <span className="chosen__icon" aria-hidden>
          <PrinterIcon size={18} />
        </span>
        <div className="chosen__copy">
          <p className="chosen__name">{target ?? "No printer picked"}</p>
          <p className="chosen__note">
            {badgePrinter && !chosenInstalled
              ? `${badgePrinter} isn't connected, so badges go to ${osDefaultPrinter ?? "no printer"} until it's back.`
              : badgePrinter
                ? "Badges go here unless you pick another printer in BadgeBadger."
                : target
                  ? "This computer's default printer."
                  : "Pick one so badges have somewhere to go."}
          </p>
        </div>
        {target && printerKind(target) === "card" ? <span className="tag">Card printer</span> : null}
      </div>

      {suggestion && !open ? (
        <div className="suggest">
          <p>
            Found a card printer: <strong>{suggestion}</strong>
          </p>
          <button type="button" className="btn btn--secondary" disabled={saving} onClick={() => choose(suggestion)}>
            Use it for badges
          </button>
        </div>
      ) : null}

      {open ? (
        <fieldset id={listId} className="choices" disabled={saving}>
          <legend className="visually-hidden">Printer for badges</legend>
          <label className="choice">
            <input
              type="radio"
              name="badge-printer"
              checked={!badgePrinter}
              onChange={() => choose(null)}
            />
            <span className="choice__name">
              This computer&rsquo;s default
              {osDefaultPrinter ? <span className="choice__sub">{osDefaultPrinter}</span> : null}
            </span>
          </label>
          {sortPrinters(printers).map((p) => {
            const kind = printerKind(p);
            return (
              <label key={p} className={kind === "virtual" ? "choice choice--quiet" : "choice"}>
                <input
                  type="radio"
                  name="badge-printer"
                  checked={badgePrinter === p}
                  onChange={() => choose(p)}
                />
                <span className="choice__name">{p}</span>
                {KIND_LABEL[kind] ? <span className="tag">{KIND_LABEL[kind]}</span> : null}
              </label>
            );
          })}
        </fieldset>
      ) : null}
    </section>
  );
}
