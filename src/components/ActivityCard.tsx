import { CheckIcon, XIcon, ClockIcon } from "./icons";
import type { ActivityItem } from "../types";

type Props = { items: ActivityItem[] };

function formatRelative(iso: string): string {
  const ts = new Date(iso).getTime();
  if (Number.isNaN(ts)) return iso;
  const diff = Date.now() - ts;
  if (diff < 5_000) return "just now";
  if (diff < 60_000) return `${Math.floor(diff / 1_000)}s ago`;
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
  return new Date(ts).toLocaleString();
}

export function ActivityCard({ items }: Props) {
  return (
    <section className="card">
      <div className="card__head">
        <h2 className="card__title">Activity</h2>
        <p className="card__subtitle">
          Recent print jobs the agent has dispatched.
        </p>
      </div>

      {items.length === 0 ? (
        <div className="empty">
          <ClockIcon size={28} className="empty__icon" />
          <p className="empty__title">No prints yet</p>
          <p className="empty__body">
            Print a badge from BadgeBadger and the job will appear here.
          </p>
        </div>
      ) : (
        <ul className="activity-list">
          {items.map((item) => {
            // Concierge-style label when the web client sent meta;
            // fall back to the raw job_name for v0.2.x web clients.
            const headline = item.employeeName
              ? item.employeeName
              : item.jobName ?? "Print job";
            const subline = item.employeeName
              ? [item.templateName, item.printer].filter(Boolean).join(" · ")
              : item.printer;
            return (
              <li
                key={item.id}
                className={
                  item.ok ? "activity-row ok" : "activity-row error"
                }
              >
                <span className="activity-row__icon" aria-hidden>
                  {item.ok ? <CheckIcon size={14} /> : <XIcon size={14} />}
                </span>
                <div className="activity-row__copy">
                  <p className="activity-row__title">
                    {headline}
                    {subline ? (
                      <>
                        {" — "}
                        <span className="activity-row__printer">{subline}</span>
                      </>
                    ) : null}
                  </p>
                  <p className="activity-row__detail">
                    {item.ok
                      ? `Sent ${formatRelative(item.startedAt)}`
                      : item.error ?? "Unknown error"}
                  </p>
                </div>
                <time
                  className="activity-row__time"
                  dateTime={item.startedAt}
                  title={new Date(item.startedAt).toLocaleString()}
                >
                  {formatRelative(item.startedAt)}
                </time>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
