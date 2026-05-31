import { useEffect, useMemo, useState } from "react";
import { assessCommand } from "../lib/api";
import { resolveCommand, type DiscoveredCommand, type SafetyAssessment } from "../lib/types";

export interface RunRequest {
  runId: string;
  name: string;
  source: string;
  resolvedCommand: string;
  cwd: string;
}

interface Props {
  command: DiscoveredCommand;
  /** True while a run started from here is still in flight. */
  busy: boolean;
  onRun: (req: RunRequest) => void;
}

/**
 * The safety surface. Shows the fully resolved command string and the absolute
 * working directory, and requires explicit confirmation before running. When
 * the resolved command trips a destructive-token, the confirmation escalates to
 * type-to-confirm.
 *
 * This is a best-effort operator-error guard (wrong env, wrong target), NOT a
 * security boundary.
 */
export function RunConfirm({ command, busy, onRun }: Props) {
  const [args, setArgs] = useState("");
  const [assessment, setAssessment] = useState<SafetyAssessment | null>(null);
  const [confirmText, setConfirmText] = useState("");

  const resolved = useMemo(
    () => resolveCommand(command.invocation, args),
    [command.invocation, args],
  );

  // Reset transient input when the selected command changes.
  useEffect(() => {
    setArgs("");
    setConfirmText("");
  }, [command.id]);

  // Re-run the heuristic whenever the resolved string changes.
  useEffect(() => {
    let cancelled = false;
    assessCommand(resolved)
      .then((a) => !cancelled && setAssessment(a))
      .catch(() => !cancelled && setAssessment(null));
    return () => {
      cancelled = true;
    };
  }, [resolved]);

  const escalate = assessment?.escalate ?? false;
  // For the stronger confirmation, the user types the command's name.
  const confirmed = !escalate || confirmText.trim() === command.name;

  function start() {
    if (!confirmed || busy) return;
    onRun({
      runId: crypto.randomUUID(),
      name: command.name,
      source: command.source,
      resolvedCommand: resolved,
      cwd: command.cwd,
    });
    setConfirmText("");
  }

  return (
    <div className="run-confirm">
      <div className="field">
        <label>Resolved command</label>
        <code className="resolved">{resolved}</code>
      </div>

      <div className="field">
        <label>Working directory</label>
        <code className="cwd">{command.cwd}</code>
      </div>

      <div className="field">
        <label>
          Arguments <span className="muted">(optional, appended)</span>
        </label>
        <input
          type="text"
          value={args}
          placeholder="e.g. --watch --filter web"
          onChange={(e) => setArgs(e.target.value)}
          spellCheck={false}
          autoComplete="off"
        />
      </div>

      {escalate && (
        <div className="escalation">
          <div className="escalation-head">
            ⚠ This looks consequential — matched: {assessment?.matched.join(", ")}
          </div>
          <div className="escalation-note">
            Best-effort operator-error guard, not a security check. Type the
            command name <strong>{command.name}</strong> to confirm.
          </div>
          <input
            type="text"
            value={confirmText}
            placeholder={command.name}
            onChange={(e) => setConfirmText(e.target.value)}
            spellCheck={false}
            autoComplete="off"
          />
        </div>
      )}

      <button
        className={escalate ? "run danger" : "run"}
        disabled={!confirmed || busy}
        onClick={start}
      >
        {busy ? "Running…" : escalate ? "Confirm & run" : "Run"}
      </button>
    </div>
  );
}
