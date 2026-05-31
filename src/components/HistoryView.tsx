import { useEffect, useState } from "react";
import { listHistory } from "../lib/api";
import type { HistoryRow } from "../lib/types";

function fmtTime(ms: number): string {
  if (!ms) return "—";
  return new Date(ms).toLocaleString();
}

function fmtDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

/** Searchable, newest-first log of past runs. */
export function HistoryView() {
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<HistoryRow[]>([]);

  useEffect(() => {
    let cancelled = false;
    const t = setTimeout(() => {
      listHistory(query)
        .then((r) => !cancelled && setRows(r))
        .catch(() => !cancelled && setRows([]));
    }, 120);
    return () => {
      cancelled = true;
      clearTimeout(t);
    };
  }, [query]);

  return (
    <div className="history">
      <input
        className="search"
        type="text"
        placeholder="Search history by command, cwd, or name…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        spellCheck={false}
        autoComplete="off"
      />

      {rows.length === 0 ? (
        <div className="empty">No runs recorded yet.</div>
      ) : (
        <table className="history-table">
          <thead>
            <tr>
              <th>When</th>
              <th>Command</th>
              <th>Directory</th>
              <th>Exit</th>
              <th>Duration</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => (
              <tr key={r.id}>
                <td className="nowrap">{fmtTime(r.started_at)}</td>
                <td>
                  <code>{r.resolved_command}</code>
                </td>
                <td className="cwd-cell" title={r.cwd}>
                  {r.cwd}
                </td>
                <td className={r.exit_code === 0 ? "ok" : "bad"}>
                  {r.exit_code ?? "—"}
                </td>
                <td className="nowrap">{fmtDuration(r.duration_ms)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
