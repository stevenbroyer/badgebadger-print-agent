// Setup checklist surfaced when any precondition is unmet — once
// everything is green, the card hides entirely so the UI stays
// calm during normal operation.
import { CheckIcon, CircleIcon } from "./icons";

type Item = {
  id: string;
  label: string;
  done: boolean;
  helpText: string;
};

export function ChecklistCard({ items }: { items: Item[] }) {
  return (
    <section className="card">
      <div className="card__head">
        <h2 className="card__title">Set up</h2>
        <p className="card__subtitle">
          A few one-time items so prints land on the right printer.
        </p>
      </div>
      <ul className="checklist">
        {items.map((item) => (
          <li
            key={item.id}
            className={item.done ? "checklist__item done" : "checklist__item"}
          >
            <span className="checklist__bullet" aria-hidden>
              {item.done ? (
                <CheckIcon size={14} />
              ) : (
                <CircleIcon size={14} />
              )}
            </span>
            <div className="checklist__copy">
              <p className="checklist__label">{item.label}</p>
              <p className="checklist__help">{item.helpText}</p>
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
