import { useStore } from "../state/store";

export function SearchBar() {
  const query = useStore((s) => s.query);
  const setQuery = useStore((s) => s.setQuery);

  return (
    <input
      className="search"
      type="text"
      placeholder="Filter commands by name, repo, description…"
      value={query}
      onChange={(e) => setQuery(e.target.value)}
      autoComplete="off"
      spellCheck={false}
    />
  );
}
