import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { cancelRun, onExit, onOutput, runCommand } from "../lib/api";
import type { RunRequest } from "./RunConfirm";

type Status = "running" | "done";

interface Props {
  req: RunRequest;
  /** Called once the run reaches a terminal state. */
  onFinished: () => void;
}

/**
 * Live PTY output via xterm.js. Mounted per-run (keyed on runId by the parent).
 * It subscribes to this run's output/exit events BEFORE asking the backend to
 * start, so no early output is lost.
 */
export function OutputTerminal({ req, onFinished }: Props) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const [status, setStatus] = useState<Status>("running");
  const [exit, setExit] = useState<{ code: number | null; ms: number } | null>(
    null,
  );

  useEffect(() => {
    const term = new Terminal({
      convertEol: true,
      fontFamily:
        'ui-monospace, SFMono-Regular, "JetBrains Mono", Menlo, monospace',
      fontSize: 13,
      theme: { background: "#0b0b0e", foreground: "#e4e4e7" },
      cursorBlink: false,
      disableStdin: true,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    if (hostRef.current) {
      term.open(hostRef.current);
      fit.fit();
    }

    const onResize = () => {
      try {
        fit.fit();
      } catch {
        /* terminal not mounted */
      }
    };
    window.addEventListener("resize", onResize);

    let disposed = false;
    const unlisteners: Array<() => void> = [];

    (async () => {
      // Subscribe first; only then start the process.
      const offOutput = await onOutput((e) => {
        if (e.run_id === req.runId) term.write(e.chunk);
      });
      const offExit = await onExit((e) => {
        if (e.run_id !== req.runId) return;
        setStatus("done");
        setExit({ code: e.exit_code, ms: e.duration_ms });
        onFinished();
      });
      if (disposed) {
        offOutput();
        offExit();
        return;
      }
      unlisteners.push(offOutput, offExit);

      try {
        await runCommand(req);
      } catch (err) {
        term.write(`\r\n\x1b[31mfailed to start: ${String(err)}\x1b[0m\r\n`);
        setStatus("done");
        setExit({ code: null, ms: 0 });
        onFinished();
      }
    })();

    return () => {
      disposed = true;
      window.removeEventListener("resize", onResize);
      unlisteners.forEach((u) => u());
      term.dispose();
    };
    // Re-run only when the run identity changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [req.runId]);

  return (
    <div className="terminal-panel">
      <div className="terminal-bar">
        <span className="terminal-cmd">{req.resolvedCommand}</span>
        {status === "running" ? (
          <button className="cancel" onClick={() => cancelRun(req.runId)}>
            Cancel
          </button>
        ) : (
          <span className={`exit ${exit?.code === 0 ? "ok" : "bad"}`}>
            exit {exit?.code ?? "—"} · {exit?.ms ?? 0}ms
          </span>
        )}
      </div>
      <div className="terminal-host" ref={hostRef} />
    </div>
  );
}
