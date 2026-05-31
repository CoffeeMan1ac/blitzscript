// Plain client-side fuzzy filter for v1. Good enough to narrow a list of
// discovered commands by name / description / repo / source. Not a ranking
// engine — just a subsequence match with a light relevance score.

import type { DiscoveredCommand } from "./types";

/** True if every char of `needle` appears in order within `haystack`. */
function subsequence(needle: string, haystack: string): boolean {
  if (!needle) return true;
  let i = 0;
  for (let j = 0; j < haystack.length && i < needle.length; j++) {
    if (haystack[j] === needle[i]) i++;
  }
  return i === needle.length;
}

/** Higher is better. Exact substring beats scattered subsequence. */
function score(needle: string, haystack: string): number {
  if (!needle) return 0;
  const idx = haystack.indexOf(needle);
  if (idx === 0) return 100;
  if (idx > 0) return 60 - Math.min(idx, 40);
  return subsequence(needle, haystack) ? 10 : -1;
}

export function filterCommands(
  commands: DiscoveredCommand[],
  query: string,
): DiscoveredCommand[] {
  const q = query.trim().toLowerCase();
  if (!q) return commands;

  const scored = commands
    .map((cmd) => {
      const fields = [
        cmd.name,
        cmd.description ?? "",
        cmd.repo,
        cmd.source,
        cmd.invocation,
      ].map((f) => f.toLowerCase());
      // Best field score wins; name is weighted most.
      const best = Math.max(
        score(q, fields[0]) * 1.5,
        score(q, fields[1]),
        score(q, fields[2]),
        score(q, fields[3]),
        score(q, fields[4]),
      );
      return { cmd, best };
    })
    .filter((s) => s.best > 0)
    .sort((a, b) => b.best - a.best);

  return scored.map((s) => s.cmd);
}
