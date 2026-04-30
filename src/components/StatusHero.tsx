// Single big status block at the top of the window. Calm green
// "Ready" when everything's set up; amber "Almost there" when the
// checklist still has open items; red "Not running" when the listener
// failed to bind. The whole point of an agent app is reassurance —
// the operator should be able to glance at this and know whether
// they're good to go.

type Props = {
  ready: boolean;
  listening: boolean;
  defaultPrinter: string | null;
  port: number | null;
};

export function StatusHero({ ready, listening, defaultPrinter, port }: Props) {
  const tone = !listening ? "error" : ready ? "ok" : "warning";
  const headline = !listening
    ? "Agent stopped"
    : ready
      ? "Ready to print"
      : "Almost ready";
  const body = !listening
    ? "The local listener didn't start. Quit and re-launch the agent."
    : ready
      ? `Listening on http://127.0.0.1:${port ?? "9988"} → ${defaultPrinter}`
      : "Open the checklist below to finish setup.";

  return (
    <section className={`hero hero--${tone}`}>
      <div className="hero__indicator" aria-hidden>
        <span className="hero__pulse" />
        <span className="hero__dot" />
      </div>
      <div className="hero__copy">
        <p className="hero__eyebrow">Status</p>
        <h2 className="hero__title">{headline}</h2>
        <p className="hero__body">{body}</p>
      </div>
    </section>
  );
}
