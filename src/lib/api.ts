// Typed wrappers around the Tauri command + event surface. Everything the
// frontend needs from the Rust core goes through here.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  DiscoveredCommand,
  ExitEvent,
  HistoryRow,
  OutputEvent,
  SafetyAssessment,
} from "./types";

export const EVENT_OUTPUT = "bs://output";
export const EVENT_EXIT = "bs://exit";

export async function getRoot(): Promise<string | null> {
  return invoke("get_root");
}

export async function setRoot(root: string): Promise<void> {
  return invoke("set_root", { root });
}

/** Rescan the tree, rebuild the cache, return the fresh list. */
export async function scan(root: string): Promise<DiscoveredCommand[]> {
  return invoke("scan", { root });
}

/** Read the cached list (no rescan) — used on startup. */
export async function listCommands(): Promise<DiscoveredCommand[]> {
  return invoke("list_commands");
}

export async function assessCommand(
  resolvedCommand: string,
): Promise<SafetyAssessment> {
  return invoke("assess_command", { resolvedCommand });
}

export async function runCommand(args: {
  runId: string;
  name: string;
  source: string;
  resolvedCommand: string;
  cwd: string;
}): Promise<void> {
  return invoke("run_command", args);
}

export async function cancelRun(runId: string): Promise<boolean> {
  return invoke("cancel_run", { runId });
}

export async function listHistory(
  query: string,
  limit = 200,
): Promise<HistoryRow[]> {
  return invoke("list_history", { query, limit });
}

/** Native directory picker. Returns the chosen absolute path, or null. */
export async function pickDirectory(): Promise<string | null> {
  const result = await open({ directory: true, multiple: false });
  return typeof result === "string" ? result : null;
}

export function onOutput(
  cb: (e: OutputEvent) => void,
): Promise<UnlistenFn> {
  return listen<OutputEvent>(EVENT_OUTPUT, (event) => cb(event.payload));
}

export function onExit(cb: (e: ExitEvent) => void): Promise<UnlistenFn> {
  return listen<ExitEvent>(EVENT_EXIT, (event) => cb(event.payload));
}
