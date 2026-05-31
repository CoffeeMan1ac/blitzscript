// Minimal global state with Zustand. Holds the chosen root, the discovered
// command cache (mirrored from the Rust side), the search query, and the
// current selection. Execution/terminal state lives in the OutputTerminal
// component because it is tied to the xterm lifecycle.

import { create } from "zustand";
import * as api from "../lib/api";
import type { DiscoveredCommand } from "../lib/types";

export type Tab = "commands" | "history";

interface AppState {
  root: string | null;
  commands: DiscoveredCommand[];
  query: string;
  selectedId: string | null;
  scanning: boolean;
  error: string | null;
  tab: Tab;

  init: () => Promise<void>;
  chooseRoot: () => Promise<void>;
  rescan: () => Promise<void>;
  setQuery: (q: string) => void;
  select: (id: string | null) => void;
  setTab: (t: Tab) => void;
}

export const useStore = create<AppState>((set, get) => ({
  root: null,
  commands: [],
  query: "",
  selectedId: null,
  scanning: false,
  error: null,
  tab: "commands",

  // On startup, load the persisted root and the cached command list. We do not
  // auto-rescan: the cache is shown immediately; the user rescans on demand.
  init: async () => {
    try {
      const root = await api.getRoot();
      const commands = root ? await api.listCommands() : [];
      set({ root, commands });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  chooseRoot: async () => {
    const picked = await api.pickDirectory();
    if (!picked) return;
    set({ root: picked, scanning: true, error: null, selectedId: null });
    try {
      const commands = await api.scan(picked);
      set({ commands, scanning: false });
    } catch (e) {
      set({ error: String(e), scanning: false });
    }
  },

  rescan: async () => {
    const { root } = get();
    if (!root) return;
    set({ scanning: true, error: null });
    try {
      const commands = await api.scan(root);
      set({ commands, scanning: false });
    } catch (e) {
      set({ error: String(e), scanning: false });
    }
  },

  setQuery: (q) => set({ query: q }),
  select: (id) => set({ selectedId: id }),
  setTab: (t) => set({ tab: t }),
}));
