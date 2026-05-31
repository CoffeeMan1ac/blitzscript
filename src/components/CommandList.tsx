import { useMemo } from "react";
import { useStore } from "../state/store";
import { filterCommands } from "../lib/fuzzy";

export function CommandList() {
  const commands = useStore((s) => s.commands);
  const query = useStore((s) => s.query);
  const selectedId = useStore((s) => s.selectedId);
  const select = useStore((s) => s.select);

  const filtered = useMemo(
    () => filterCommands(commands, query),
    [commands, query],
  );

  if (commands.length === 0) {
    return (
      <div className="empty">
        Nothing indexed yet. Choose a repos folder, then Rescan.
      </div>
    );
  }

  if (filtered.length === 0) {
    return <div className="empty">No commands match “{query}”.</div>;
  }

  return (
    <ul className="command-list">
      {filtered.map((cmd) => {
        const repoName = cmd.repo.split("/").filter(Boolean).pop() ?? cmd.repo;
        return (
          <li
            key={cmd.id}
            className={cmd.id === selectedId ? "selected" : ""}
            onClick={() => select(cmd.id)}
          >
            <div className="cmd-row">
              <span className={`badge badge-${cmd.source}`}>{cmd.source}</span>
              <span className="cmd-name">{cmd.name}</span>
              <span className="cmd-repo">{repoName}</span>
            </div>
            {cmd.description && (
              <div className="cmd-desc">{cmd.description}</div>
            )}
          </li>
        );
      })}
    </ul>
  );
}
