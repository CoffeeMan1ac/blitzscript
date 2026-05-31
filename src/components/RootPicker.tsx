import { useStore } from "../state/store";

/** Header control: pick the repos root, and rescan it on demand. */
export function RootPicker() {
  const root = useStore((s) => s.root);
  const scanning = useStore((s) => s.scanning);
  const chooseRoot = useStore((s) => s.chooseRoot);
  const rescan = useStore((s) => s.rescan);

  return (
    <div className="root-picker">
      <div className="root-path" title={root ?? ""}>
        {root ? (
          <>
            <span className="muted">root</span> {root}
          </>
        ) : (
          <span className="muted">No repos folder selected</span>
        )}
      </div>
      <div className="root-actions">
        <button onClick={chooseRoot} disabled={scanning}>
          {root ? "Change folder" : "Choose folder"}
        </button>
        <button onClick={rescan} disabled={!root || scanning}>
          {scanning ? "Scanning…" : "Rescan"}
        </button>
      </div>
    </div>
  );
}
