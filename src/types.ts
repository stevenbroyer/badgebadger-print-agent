// Surface from the Rust side via the `get_status` command.
export type AgentStatus = {
  listenerPort: number;
  listening: boolean;
  // Where a badge goes when BadgeBadger doesn't name a printer: the badge
  // printer if it's installed, else the OS default.
  defaultPrinter: string | null;
  osDefaultPrinter: string | null;
  // The printer picked in this window, installed or not. Null means
  // "follow the OS default".
  badgePrinter: string | null;
  printers: string[];
  // Whether the third-party PDF helper (SumatraPDF on Windows; n/a on
  // Mac/Linux which use CUPS) is installed. False on Windows blocks
  // the agent from actually printing.
  helperInstalled: boolean;
  // What the last polls of BadgeBadger said about this computer.
  connection: Connection;
  computerName: string;
};

export type Connection = "checking" | "connected" | "notPaired" | "offline";

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
