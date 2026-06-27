# blitzscript

Local-first launcher that indexes and runs the commands that already exist across your repos.

You accumulate npm scripts and Makefile targets across dozens of projects, then
forget which exist and what they're called. blitzscript points at a folder of
repos, finds every runnable command inside, and gives you one searchable place
to run them.

## What it does

- Discovers npm scripts and Makefile targets under a chosen root, tagged by repo.
- Fuzzy-search and run, with output streamed to an embedded terminal.
- Flags destructive commands (e.g. `rm -rf`) for a deliberate confirmation.
- Keeps a searchable history of every run.

## Build

Requires Rust and Node.

```sh
npm install
npm run tauri:dev     # run in development
npm run tauri:build   # release build
```

## License

MIT — see [LICENSE](LICENSE).
