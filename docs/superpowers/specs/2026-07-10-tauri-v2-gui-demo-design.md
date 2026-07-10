# Tauri v2 GUI Demo for game-scanner

Date: 2026-07-10
Status: Draft
Owner: wankey

## Purpose

Provide a cross-platform desktop GUI demo of the `game-scanner` Rust library. The existing `examples/` folder exposes only CLI snippets; there is no way for users to visually browse scanned games across all 8 supported launchers, see the capability matrix in action, or interact with launch / close / install / uninstall from a window. This project fills that gap with a Tauri v2 application that surfaces every public API of the library.

## Goals

1. Demonstrate every public operation of `game-scanner`: list, find, get launcher executable, install, launch, uninstall, get processes, close.
2. Demonstrate all 8 launchers through one unified UI with a capability matrix that mirrors the README.
3. Run on Windows, macOS, and Linux with graceful degradation for unsupported launchers.
4. Stay small: minimal frontend dependencies, fast `tauri dev` startup, no heavy framework lock-in.
5. Be easy to read as a reference implementation for someone wanting to embed `game-scanner` in their own Tauri app.

## Non-goals

- Production launcher / game store. This is a demo, not a competitor to Steam.
- Account authentication, online store browsing, library management.
- Multi-language UI. English only.
- Persistence (no saved settings, no recent launchers, no theme toggle).
- Auto-update. The Tauri updater plugin is out of scope.
- Code signing / notarization. Distribution is left to whoever runs the demo locally.

## Architecture

### Repository layout

The root `Cargo.toml` becomes a Cargo workspace. `game-scanner` keeps its current package as the library. A new `gui/` directory at the repo root holds the Tauri application. `gui/src-tauri/` is the Rust backend (a workspace member that depends on `game-scanner` via path); `gui/src/` is the Svelte frontend.

```
game-scanner/
├── Cargo.toml                     # workspace [members = [".", "gui/src-tauri"]]
├── src/                           # game-scanner 库（不变）
├── examples/                      # 现有 CLI 示例（不变）
├── gui/                           # Tauri demo 应用
│   ├── package.json
│   ├── vite.config.ts
│   ├── index.html
│   ├── tsconfig.json
│   ├── src/                       # Svelte 前端
│   │   ├── App.svelte
│   │   ├── main.ts
│   │   ├── app.css
│   │   ├── lib/
│   │   │   ├── stores.ts
│   │   │   ├── capabilities.ts
│   │   │   └── tauri.ts
│   │   └── components/
│   │       ├── LauncherList.svelte
│   │       ├── GamesTable.svelte
│   │       ├── ActionPanel.svelte
│   │       └── FindDialog.svelte
│   └── src-tauri/
│       ├── Cargo.toml             # game-scanner = { path = "../.." }
│       ├── tauri.conf.json
│       ├── build.rs
│       ├── icons/
│       └── src/
│           ├── main.rs
│           ├── commands.rs
│           └── capability.rs
└── docs/superpowers/specs/
    └── 2026-07-10-tauri-v2-gui-demo-design.md
```

### Backend (Rust)

`gui/src-tauri/src/main.rs` boots Tauri, registers every command and its capability allow-list in `tauri.conf.json`, and calls `setup` only for logging. No business logic lives here.

`commands.rs` exposes one `#[tauri::command]` per public operation. Each command is a thin adapter that calls into `game_scanner` and converts `Result<T, game_scanner::error::Error>` into `Result<T, String>` via `map_err(|e| e.to_string())`.

Commands:

| Command | Rust call | Args |
| --- | --- | --- |
| `list_games` | dispatch to `steam::games()`, `epicgames::games()`, etc. | `launcher: GameType` |
| `find_game` | dispatch to per-launcher `find(id)` | `launcher: GameType, id: String` |
| `launcher_executable` | dispatch to per-launcher `executable()` | `launcher: GameType` |
| `install_game` | `manager::install_game(&game)` | `game: Game` |
| `uninstall_game` | `manager::uninstall_game(&game)` | `game: Game` |
| `launch_game` | `manager::launch_game(&game)` | `game: Game` |
| `close_game` | `manager::close_game(&game)` | `game: Game` |
| `get_processes` | `manager::get_processes(&game)` | `game: Game` |
| `get_capabilities` | `capability::matrix()` returns `HashMap<GameType, Vec<Op>>` | none |

`GameType` is serialized to its lowercase string form (`steam`, `epicgames`, etc.) so the wire format matches `GameType::to_string()`.

`capability.rs` defines `enum Op { List, Find, Executable, Install, Launch, Uninstall, Close, GetProcesses }` and `fn supports(launcher: GameType, op: Op) -> bool`. The matrix is hard-coded from the README tables.

### Frontend (Svelte 5 + TypeScript + Vite)

`lib/stores.ts` holds five `writable` stores: `currentLauncher`, `games`, `selectedGame`, `processes`, `launcherExecutable`. Stores are updated by command responses and user actions; components subscribe to render.

`lib/capabilities.ts` does not exist as a separate source of truth. Instead, the frontend calls a `get_capabilities` command once on app boot, which returns `HashMap<GameType, Vec<Op>>` from the Rust `capability.rs` matrix. The result is cached in a `writable` store (`capabilities`) and consumed by `ActionPanel` and `LauncherList`. This eliminates the risk of the TS mirror drifting from the Rust source of truth.

`lib/tauri.ts` exports typed wrappers (`listGames(launcher): Promise<Game[]>`, `launchGame(game): Promise<void>`, etc.) over `invoke()`. Each wrapper handles the rejection path and rethrows so callers can toast.

Components:

- `LauncherList.svelte` — vertical list of 8 launchers. Each row shows the launcher name plus a small badge with the count of supported ops (read from the `capabilities` store). Selecting a launcher updates `currentLauncher`.
- `GamesTable.svelte` — subscribes to `currentLauncher` and `games`. On launcher change, calls `listGames` and resets `selectedGame`. Columns: name, id, installed, needs update, downloading. Click a row to set `selectedGame`. Toolbar above the table holds `[Refresh]` and `[Find by Id]`.
- `ActionPanel.svelte` — reads `selectedGame`, `currentLauncher`, and the `capabilities` store. Renders buttons for every op; each button's `disabled` is `!capabilities.get(currentLauncher)?.includes(op)` and its tooltip explains why an op is unsupported when disabled. Also shows the launcher executable path fetched via `launcherExecutable`. When the selected game is `Launched`, a 5-second interval calls `getProcesses` and updates `processes`.
- `FindDialog.svelte` — modal with one text input. On submit calls `findGame(currentLauncher, id)` and sets `selectedGame`.

`App.svelte` lays out the three panels with CSS grid: sidebar (240px) | table (flex 1) | actions (320px). A small footer shows `game-scanner v{version}` for context.

## Data flow

1. App startup → `invoke('get_capabilities')` populates the `capabilities` store → `currentLauncher.set('steam')` → `GamesTable` effect fires → `invoke('list_games', { launcher: 'steam' })` → JSON array back → `games.set([...])` → table renders. In parallel, `invoke('launcher_executable', { launcher: 'steam' })` populates the action panel header.
2. User selects a row → `selectedGame.set(game)` → `ActionPanel` re-renders with the right enabled buttons.
3. User clicks `Launch` → `invoke('launch_game', { game })`. On success a toast appears and a polling timer starts; on failure the error is toasted.
4. User clicks `Close` → `invoke('close_game', { game })` → polling stops, `processes` is cleared.
5. User clicks `Find by Id` → modal opens → submits → `invoke('find_game', ...)` → on success `selectedGame` updates; on `GameNotFound` a toast says so.

## Error handling

| Layer | Source | Action |
| --- | --- | --- |
| Rust command | `game_scanner::error::Error` (`ErrorKind::LauncherNotInstalled`, `GameNotFound`, `InvalidLauncher`, etc.) | `map_err(|e| e.to_string())` and return. Frontend toasts the message and logs to console. |
| Tauri IPC | invoke rejection (network, serialization) | `tauri.ts` wrappers catch and rethrow as `Error`; UI shows generic "IPC failed" toast. |
| Capability matrix | Op not supported for selected launcher | Button is `disabled` with tooltip citing the README row (e.g. "Amazon does not support install"). |
| Empty state | `games` is empty | Table shows "No games found. Is <launcher> installed?" |
| Unsupported platform | `executable()` returns `InvalidLauncher` on non-Windows for Amazon | Frontend treats as "launcher not available" and disables only Amazon's row. |

## Testing

The Rust backend is mostly a thin adapter over `game-scanner`, which has its own test suite. The unit-testable surface in the new code is small but real:

- `capability.rs`: one `#[test]` per (launcher, op) cell of the matrix. Locks the truth table so changes to `game-scanner` are visible here too. ~50 assertions, all in one `mod tests`.
- `commands.rs`: no dedicated tests; covered indirectly by `cargo test -p game-scanner` and manual `tauri dev` smoke.

Frontend is not unit-tested. ROI is low for a demo and the test surface is mostly Svelte reactivity glue. Manual verification checklist (kept in `gui/README.md`):

1. `pnpm install && pnpm tauri dev` opens a window.
2. Each of the 8 launchers can be selected without panic.
3. Steam + Ubisoft + GOG (where applicable) show non-empty game lists on a real install.
4. Unsupported launchers (e.g. Amazon on Linux) show the "launcher not available" toast and stay clickable so users can see the empty state.
5. `Launch` opens a real game process; `Get Processes` returns its PID within 5 seconds; `Close` terminates it.
6. `Find by Id` with a real id selects the right row; with a fake id shows the not-found toast.

## Dependencies

Backend (added to `gui/src-tauri/Cargo.toml`):

- `tauri = { version = "2", features = [] }`
- `tauri-build = { version = "2", features = [] }` (build dep)
- `serde = { version = "1", features = ["derive"] }`
- `serde_json = "1"`
- `game-scanner = { path = "../.." }`

Frontend (added to `gui/package.json`):

- `@tauri-apps/api` (^2)
- `@tauri-apps/cli` (^2, dev)
- `svelte` (^5)
- `vite` (^5, dev)
- `typescript` (^5, dev)
- `@sveltejs/vite-plugin-svelte` (^4, dev)
- `svelte-check` (^4, dev)

No CSS framework, no icon library, no state library beyond Svelte's built-in stores.

## Open risks

- **Steam-only state data.** Only Steam populates `state` fields like `needs_update` and `downloading` per the README. The UI must render other launchers' state columns as `—` to avoid implying functionality that does not exist.
- **Process polling cost.** `get_processes` calls `System::new_all()` every 5 seconds while a game is launched. This is acceptable for a demo with one game at a time; documented in code comment so a future change to a longer interval is obvious.
- **Tauri v2 capability file churn.** Tauri v2 is still adding capability features; if `commands.rs` grows beyond the default allow-list, the `capabilities/*.json` file under `src-tauri/` will need updates. Mitigated by keeping every command in the default allow-list and revisiting only if Tauri prompts for a permission error.
- **macOS / Linux build verification.** Development machine is Windows; the cross-platform CI configuration is out of scope. The README will state "tested on Windows; macOS and Linux should work but are not part of CI for this demo."

## Implementation order

1. Workspace conversion: add `[workspace]` to root `Cargo.toml`, scaffold `gui/src-tauri/Cargo.toml` with `path = "../.."` to `game-scanner`, verify `cargo build -p game-scanner` still passes.
2. Minimal Tauri scaffold: `pnpm create tauri-app` baseline (Svelte + TS) adapted to our paths; confirm `pnpm tauri dev` opens a window with a placeholder.
3. Backend commands: implement `commands.rs` and `capability.rs`; smoke test each command from `tauri.conf.json` allow-list.
4. Frontend stores and `tauri.ts` wrappers: typed contracts, no UI yet.
5. Components: build `LauncherList`, `GamesTable`, `ActionPanel`, `FindDialog` against the stores.
6. Polish: layout, tooltips on disabled buttons, empty states, version footer.
7. Tests: write the `capability.rs` matrix assertions.
8. README in `gui/` with the manual verification checklist.

## Reference

- Tauri v2 docs: https://tauri.app/start/
- Svelte 5 docs: https://svelte.dev/docs
- `game-scanner` capability matrix: README.md in repo root (tables for "Game Commands support", "Game State support", "Operations", "Management").