// Surface from the Rust side via the `get_status` command.
export type AgentStatus = {
  listenerPort: number;
  listening: boolean;
  defaultPrinter: string | null;
  printers: string[];
};

// Emitted from Rust on the `print` Tauri event whenever a job is
// dispatched (whether or not the dispatch itself succeeded). Drives
// the activity feed + toast notifications.
export type PrintEvent = {
  startedAt: string; // ISO timestamp
  printer: string;
  jobName: string | null;
  ok: boolean;
  error: string | null;
};

export type ActivityItem = {
  id: string;
  startedAt: string;
  printer: string;
  jobName: string | null;
  ok: boolean;
  error: string | null;
};
