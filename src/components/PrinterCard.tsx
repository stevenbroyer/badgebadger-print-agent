import { PrinterIcon, ZapIcon } from "./icons";

type Props = {
  printers: string[];
  defaultPrinter: string | null;
  onTestPrint: () => void;
  testing: boolean;
};

export function PrinterCard({
  printers,
  defaultPrinter,
  onTestPrint,
  testing,
}: Props) {
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
      )}

      <div className="card__actions">
        <button
          type="button"
          className="btn btn--primary"
          onClick={onTestPrint}
          disabled={testing || !defaultPrinter}
          title={
            !defaultPrinter
              ? "Set a default printer first"
              : "Send a small test card"
          }
        >
          <ZapIcon size={14} />
          {testing ? "Sending…" : "Run a test print"}
        </button>
      </div>
    </section>
  );
}
