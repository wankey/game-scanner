# game-scanner Tauri Demo

A cross-platform desktop GUI that exercises every public API of the
[`game-scanner`](https://github.com/EqualGames/game-scanner) Rust library
across all 8 supported launchers (Steam, Epic Games, GOG, Blizzard,
Ubisoft, Amazon Games, Origin, Riot Games).

The capability matrix in the UI mirrors the matrix in the root
[`README.md`](../README.md). Steam supports the most operations; other
launchers gradually lose install/uninstall/process-management buttons.

## Requirements

- [Rust](https://rustup.rs) stable (with the `cargo` and `rustc` tools on `PATH`)
- [Node.js](https://nodejs.org) >= 18
- [pnpm](https://pnpm.io) >= 8
- Tauri v2 platform prerequisites for your OS — see
  <https://tauri.app/start/prerequisites/>.

### Windows note

If `pnpm install` fails with `ERR_PNPM_EPERM` against a global pnpm
store, point the project at a writable store:

```bash
pnpm config set store-dir "$(pwd)/node_modules/.pnpm" --location project
pnpm install
```

## Run in development

```bash
cd gui
pnpm install
pnpm tauri dev
```

The first run downloads Tauri Rust dependencies and can take several
minutes. Subsequent runs are fast.

## Build a release bundle

```bash
cd gui
pnpm tauri build
```

The bundle (`.msi` / `.dmg` / `.AppImage` / `.deb`) lands in
`gui/src-tauri/target/release/bundle/`.

To skip bundling and just compile the release binary:

```bash
pnpm tauri build --no-bundle
```

The exe lands in `target/release/game-scanner-gui[.exe]`.

## Manual verification checklist

After `pnpm tauri dev`:

1. Window opens with the three-column layout (Launchers | Games | Actions).
2. Steam is selected by default; games load (or the empty state appears if Steam is not installed).
3. Each of the 8 launchers can be selected without a crash.
4. Capability buttons reflect the README matrix:
   - Steam: Install, Launch, Uninstall, Get Processes, Close all enabled.
   - Amazon Games: only Launch enabled.
5. `Find by Id` resolves a real id (e.g. Steam `945360`) and selects the row.
6. `Launch` opens a real game process; `Get Processes` returns its PID within 5 seconds.
7. `Close` terminates the game and stops process polling.
8. A launcher that is not installed produces an error toast instead of a crash.

## Architecture

- `gui/src-tauri/` — Rust backend. Each `game-scanner` API is wrapped by a
  thin `#[tauri::command]` in `src/commands.rs`. The capability matrix in
  `src/capability.rs` is the single source of truth and is exposed to the
  frontend via `get_capabilities`.
- `gui/src/` — Svelte 5 + TypeScript frontend.
  - `lib/types.ts` mirrors the Rust types.
  - `lib/tauri.ts` provides typed `invoke()` wrappers.
  - `lib/stores.ts` holds Svelte writable stores.
  - `components/` contains `LauncherList`, `GamesTable`, `ActionPanel`,
    `FindDialog`, and `Toast`.

### IPC contract

The Rust commands accept the snake-case launcher name (`"steam"`,
`"epicgames"`, etc.) as a `String` rather than `GameType` directly:

- `game_scanner::prelude::GameType` does not implement `Deserialize`,
  so it cannot cross the IPC boundary as a JSON value.
- The `game_scanner::error` module is private, so each launcher call is
  matched into `Result<T, _>` with an opaque error type, then
  `map_err(|e| e.to_string())`-ed into a `Result<T, String>`.

## Tests

```bash
cd gui/src-tauri
cargo test
```

Runs the capability-matrix table-driven tests in `capability.rs`
(13 cases covering every (launcher × op) cell).

## Tested platforms

Development and manual verification were performed on Windows. The
backend code is platform-agnostic (it dispatches to per-launcher modules
that already handle their own OS gating), but the demo GUI has not been
exercised on macOS or Linux in CI.
