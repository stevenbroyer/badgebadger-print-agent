// Surface from the Rust side via the `get_status` command.
export type AgentStatus = {
  listenerPort: number;
  listening: boolean;
  defaultPrinter: string | null;
  printers: string[];
  // Whether the third-party PDF helper (SumatraPDF on Windows; n/a on
  // Mac/Linux which use CUPS) is installed. False on Windows blocks
  // the agent from actually printing — surfaced as a setup-checklist
  // step with an install link in the UI.
  helperInstalled: boolean;
};

// Emitted from Rust on the `print` Tauri event whenever a job is
// dispatched (whether or not the dispatch itself succeeded). Drives
// the activity feed + toast notifications.
//
// `employeeName` + `templateName` are populated when the web client
// sends an X-Job-Meta header — gives the activity feed real "Sam
// Rivera · Lifeguard 2026" rows instead of "badge-a1b2c3d4". When
// absent the UI falls back to `jobName`.
export type PrintEvent = {
  startedAt: string; // ISO timestamp
  printer: string;
  jobName: string | null;
  employeeName: string | null;
  templateName: string | null;
  ok: boolean;
  error: string | null;
};

export type ActivityItem = {
  id: string;
  startedAt: string;
  printer: string;
  jobName: string | null;
  employeeName: string | null;
  templateName: string | null;
  ok: boolean;
  error: string | null;
};
