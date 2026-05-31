// Shapes mirrored from the Rust core (serde-serialized).

export type SourceType = "npm" | "make";

export interface DiscoveredCommand {
  id: string;
  repo: string;
  source: SourceType;
  name: string;
  description: string | null;
  invocation: string;
  cwd: string;
}

export interface SafetyAssessment {
  escalate: boolean;
  matched: string[];
}

export interface HistoryRow {
  id: number;
  name: string;
  source: string;
  resolved_command: string;
  cwd: string;
  exit_code: number | null;
  duration_ms: number;
  started_at: number;
}

// Tauri event payloads (see exec.rs).
export interface OutputEvent {
  run_id: string;
  chunk: string;
}

export interface ExitEvent {
  run_id: string;
  exit_code: number | null;
  duration_ms: number;
}

/** Resolve the command string the same way everywhere: invocation + args. */
export function resolveCommand(invocation: string, args: string): string {
  const trimmed = args.trim();
  return trimmed ? `${invocation} ${trimmed}` : invocation;
}
