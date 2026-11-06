import { useState } from "react";
import { CheckIcon, XIcon } from "./icons";
import type { ActivityItem } from "../types";

// The last few badges through this computer, newest first, so "did Sam's
// badge go through?" is answered without opening BadgeBadger. The list is
// in memory only and starts empty each time the agent starts.

type Props = { items: ActivityItem[]; printsToday: number };

const SHOWN = 3;

function ago(iso: string): string {
  const ts = new Date(iso).getTime();
  if (Number.isNaN(ts)) return "";
  const diff = Date.now() - ts;
  if (diff < 60_000) return "now";
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} min`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} hr`;
  return new Date(ts).toLocaleDateString();
}

export function RecentPrints({ items, printsToday }: Props) {
  const [all, setAll] = useState(false);
  const shown = all ? items : items.slice(0, SHOWN);

  return (
    <section className="section" aria-labelledby="recent-title">
      <div className="section__head">
        <h2 id="recent-title" className="section__title">
          Recent prints
        </h2>
        {printsToday > 0 ? <span className="section__count">{printsToday} today</span> : null}
      </div>

      {items.length === 0 ? (
        <p className="quiet">Badges you print from BadgeBadger show up here.</p>
      ) : (
        <ul className="prints">
          {shown.map((item) => {
            const name = item.employeeName ?? item.jobName ?? "Print job";
            const detail = item.ok
              ? [item.templateName, item.printer].filter(Boolean).join(" · ")
              : (item.error ?? "It didn't print.");
            return (
              <li key={item.id} className={item.ok ? "print" : "print print--failed"}>
                <span className="print__mark" aria-label={item.ok ? "Printed" : "Didn't print"}>
                  {item.ok ? <CheckIcon size={12} /> : <XIcon size={12} />}
                </span>
                <span className="print__name">{name}</span>
                <time
                  className="print__when"
                  dateTime={item.startedAt}
                  title={new Date(item.startedAt).toLocaleString()}
                >
                  {ago(item.startedAt)}
                </time>
                <span className="print__detail" title={detail}>
                  {detail}
                </span>
              </li>
            );
          })}
        </ul>
      )}

      {items.length > SHOWN ? (
        <button type="button" className="btn btn--link" onClick={() => setAll((v) => !v)}>
          {all ? "Show fewer" : `Show ${items.length - SHOWN} more`}
        </button>
      ) : null}
    </section>
  );
}
