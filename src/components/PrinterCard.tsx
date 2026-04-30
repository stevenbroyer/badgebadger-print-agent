import { PrinterIcon, ZapIcon } from "./icons";

type Props = {
  printers: string[];
  defaultPrinter: string | null;
  selectedPrinter: string | null;
  onSelectPrinter: (printer: string | null) => void;
  onTestPrint: () => void;
  testing: boolean;
};

export function PrinterCard({
  printers,
  defaultPrinter,
  selectedPrinter,
  onSelectPrinter,
  onTestPrint,
  testing,
}: Props) {
  const effectiveTarget = selectedPrinter ?? defaultPrinter;
  const canPrint = !!effectiveTarget;

  return (
    <section className="card">
      <div className="card__head">
        <h2 className="card__title">Printers</h2>
        <p className="card__subtitle">
          Print jobs without an explicit destination go to your default.
        </p>
      </div>

      {printers.length === 0 ? (
        <div className="empty">
          <PrinterIcon size={28} className="empty__icon" />
          <p className="empty__title">No printers detected</p>
          <p className="empty__body">
            Plug in your printer and install the manufacturer driver. The
            agent picks it up automatically.
          </p>
        </div>
      ) : (
        <>
          <ul className="printer-list">
            {printers.map((p) => (
              <li
                key={p}
                className={
                  p === defaultPrinter ? "printer-row default" : "printer-row"
                }
              >
                <span className="printer-row__icon" aria-hidden>
                  <PrinterIcon size={16} />
                </span>
                <span className="printer-row__name">{p}</span>
                {p === defaultPrinter ? (
                  <span className="badge">Default</span>
                ) : null}
              </li>
            ))}
          </ul>

          <label className="printer-select">
            <span className="printer-select__label">Test print to</span>
            <select
              className="printer-select__input"
              value={selectedPrinter ?? ""}
              onChange={(e) =>
                onSelectPrinter(
                  e.target.value === "" ? null : e.target.value,
                )
              }
            >
              <option value="">
                {defaultPrinter
                  ? `Default — ${defaultPrinter}`
                  : "Default (none set)"}
              </option>
              {printers.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
          </label>
        </>
      )}

      <div className="card__actions">
        <button
          type="button"
          className="btn btn--primary"
          onClick={onTestPrint}
          disabled={testing || !canPrint}
          title={
            !canPrint
              ? "Pick a printer or set an OS default"
              : `Send a small test card to ${effectiveTarget}`
          }
        >
          <ZapIcon size={14} />
          {testing ? "Sending…" : "Run a test print"}
        </button>
      </div>
    </section>
  );
}
