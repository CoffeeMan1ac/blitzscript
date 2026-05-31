import { useEffect, useState } from "react";
import { useStore } from "./state/store";
import { RootPicker } from "./components/RootPicker";
import { SearchBar } from "./components/SearchBar";
import { CommandList } from "./components/CommandList";
import { RunConfirm, type RunRequest } from "./components/RunConfirm";
import { OutputTerminal } from "./components/OutputTerminal";
import { HistoryView } from "./components/HistoryView";

export default function App() {
  const init = useStore((s) => s.init);
  const tab = useStore((s) => s.tab);
  const setTab = useStore((s) => s.setTab);
  const error = useStore((s) => s.error);
  const commands = useStore((s) => s.commands);
  const selectedId = useStore((s) => s.selectedId);

  // The currently active run (drives the OutputTerminal). Lives here so it
  // survives reselecting commands but is replaced when a new run starts.
  const [activeRun, setActiveRun] = useState<RunRequest | null>(null);
  const [running, setRunning] = useState(false);

  useEffect(() => {
    init();
  }, [init]);

  const selected = commands.find((c) => c.id === selectedId) ?? null;

  function handleRun(req: RunRequest) {
    setActiveRun(req);
    setRunning(true);
  }

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <span className="bolt">⚡</span> blitzscript
        </div>
        <RootPicker />
        <nav className="tabs">
          <button
            className={tab === "commands" ? "active" : ""}
            onClick={() => setTab("commands")}
          >
            Commands
          </button>
          <button
            className={tab === "history" ? "active" : ""}
            onClick={() => setTab("history")}
          >
            History
          </button>
        </nav>
      </header>

      {error && <div className="error-bar">{error}</div>}

      {tab === "history" ? (
        <HistoryView />
      ) : (
        <div className="main">
          <section className="left">
            <SearchBar />
            <CommandList />
          </section>

          <section className="right">
            {selected ? (
              <RunConfirm
                command={selected}
                busy={running}
                onRun={handleRun}
              />
            ) : (
              <div className="empty pad">
                Select a command to see its resolved invocation and run it.
              </div>
            )}

            {activeRun && (
              <OutputTerminal
                key={activeRun.runId}
                req={activeRun}
                onFinished={() => setRunning(false)}
              />
            )}
          </section>
        </div>
      )}
    </div>
  );
}
