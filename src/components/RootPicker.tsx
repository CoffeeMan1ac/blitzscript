import { useEffect, useState } from "react";
import { useStore } from "../state/store";

/** Header control: point blitzscript at a repos root, and rescan it on demand.
 *
 * Two ways to choose: the native folder picker, or paste/type an absolute path
 * (more reliable than the fiddly GTK directory chooser on Linux). */
export function RootPicker() {
  const root = useStore((s) => s.root);
  const scanning = useStore((s) => s.scanning);
  const chooseRoot = useStore((s) => s.chooseRoot);
  const scanPath = useStore((s) => s.scanPath);
  const rescan = useStore((s) => s.rescan);

  const [path, setPath] = useState("");

  // Keep the field showing the active root (until the user edits it).
  useEffect(() => {
    setPath(root ?? "");
  }, [root]);

  function submit() {
    if (path.trim()) scanPath(path);
  }

  return (
    <div className="root-picker">
      <input
        className="root-input"
        type="text"
        value={path}
        placeholder="Paste a repos folder path, e.g. /home/denys/Projects"
        onChange={(e) => setPath(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && submit()}
        spellCheck={false}
        autoComplete="off"
        disabled={scanning}
      />
      <div className="root-actions">
        <button onClick={submit} disabled={scanning || !path.trim()}>
          {scanning ? "Scanning…" : "Scan"}
        </button>
        <button onClick={chooseRoot} disabled={scanning} title="Native folder picker">
          Browse…
        </button>
        <button onClick={rescan} disabled={!root || scanning}>
          Rescan
        </button>
      </div>
    </div>
  );
}
