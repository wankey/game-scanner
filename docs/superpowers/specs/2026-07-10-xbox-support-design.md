# Xbox / Microsoft Store Game Support

Date: 2026-07-10
Status: Draft
Owner: wankey

## Purpose

`game-scanner` enumerates installed games from eight launchers (Amazon, Blizzard, Epic, GOG, Origin, Riot, Steam, Ubisoft). Two adjacent platforms — **Microsoft Store** and the **Xbox App for PC** (which is the host of the Xbox Game Pass for PC library) — are missing. Both distribute Windows packages (UWP / MSIX / Win32 packaged) through the same `WindowsApps` infrastructure and are enumerable through one WinRT API call. Adding them as a single new launcher variant gives users visibility into a large slice of installed games that the tool currently does not see.

This spec covers the Rust core module, the C ABI bindings, the Tauri demo GUI, the capability matrix, tests, and docs that need to change in lockstep.

## Goals

1. Add an `xbox` launcher variant that enumerates installed Microsoft Store and Xbox App games on Windows.
2. Re-use the existing `Game` data contract — no new public fields on `Game` or `GameState`.
3. Re-use the existing `manager::launch_game` / `close_game` paths where possible; only `uninstall_game` and `get_processes` need a launcher-specific dispatch.
4. Wire the new variant into the FFI `gs_list` / `gs_find` / `gs_executable` functions and into the Tauri `list_games` / `find_game` / `launcher_executable` commands.
5. Surface the new variant in the GUI launcher list and capability matrix.
6. Add `docs/xbox.md` and README matrix rows so the change is discoverable.

## Non-goals

- Linux / macOS support. Microsoft Store and Xbox App are Windows-only. On other platforms `xbox::games()` returns `LauncherNotFound`, mirroring how `amazon` / `ubisoft` behave on macOS today.
- Network calls. No `https://apps.microsoft.com/store/...` lookups. Offline enumeration only.
- 100% accurate "is this UWP a game?" classification. The spec adopts a curated family-name allow-list (see § Filtering); coverage of long-tail indie games is best-effort.
- Install support. Microsoft Store / Game Pass installs go through Store UI and subscription authorization; not reproducible from outside.
- Pure-UWP process tracking. v1 enumerates processes by exe path / cwd under the package's `install_location`; pure-UWP processes that have neither may be missed (see § Processes).
- Microsoft Store analytics, achievements, play time, friend lists, or any online feature.

## Scope summary

| Aspect | Decision |
| --- | --- |
| Coverage | Microsoft Store **and** Xbox App for PC installed games (Game Pass library included) |
| Platform | Windows only |
| Data source | `Windows.Management.Deployment.PackageManager::FindPackages` WinRT API (current user) |
| Filtering | Family-name prefix allow-list (~25 entries), 0 false positives |
| Operations exposed | `list`, `find`, `executable`, `launch`, `uninstall`, `processes`, `close` (no `install`) |
| Manifest parsing | `quick-xml` for `<Application Id>` and `<DisplayName>` only |
| Launcher executable | `XboxApp.exe` from the `Microsoft.GamingApp_*` package |

## Architecture

### Repository layout

```
game-scanner/
├── Cargo.toml                       # + windows + quick-xml (Windows-only deps)
├── src/
│   ├── lib.rs                       # + pub mod xbox;
│   ├── prelude.rs                   # GameType::XboxGames + to_string / From<String>
│   ├── manager/mod.rs               # + match dispatch for _type == "xbox"
│   └── xbox/                        # NEW
│       ├── mod.rs                   # games / find / executable / uninstall / processes
│       ├── manifest.rs              # AppxManifest.xml 解析
│       ├── process.rs               # UWP 进程枚举（按 install_location 匹配）
│       └── platform/
│           ├── mod.rs               # cfg_attr 分发
│           ├── windows.rs           # WinRT PackageManager 调用
│           ├── linux.rs             # stub 返回 LauncherNotFound
│           └── macos.rs             # stub 返回 LauncherNotFound
├── game-scanner-ffi/
│   ├── include/game_scanner.h       # 文档注释更新
│   └── src/lib.rs                   # parse_launcher / gs_list / gs_find / gs_executable 加 arm
├── gui/
│   ├── src/lib/types.ts             # GameType 联合 / ALL_GAME_TYPES / LAUNCHER_LABELS
│   └── src-tauri/src/
│       ├── commands.rs              # 3 处 match 加 arm
│       └── capability.rs            # supports + matrix + tests
├── tests/list.rs                    # + mod xbox (Windows-only)
├── README.md                        # 5 张表各加 Xbox 行
└── docs/xbox.md                     # NEW — launcher 内部细节记录
```

### `xbox` module

```rust
// src/xbox/mod.rs
pub fn executable() -> Result<PathBuf>;     // 返回 XboxApp.exe
pub fn games() -> Result<Vec<Game>>;        // 扫描所有 game 包
pub fn find(id: &str) -> Result<Game>;      // 按 PackageFullName 查找
pub fn uninstall(game: &Game) -> Result<()>;
pub fn processes(game: &Game) -> Option<Vec<u32>>;

// platform/{windows,linux,macos}.rs —— 只有 windows.rs 是真实现
// linux / macos 各自 stub 返回 LauncherNotFound
```

Internal helper type (private to the module):

```rust
struct XboxGame {
    full_name: String,         // Package.Id.FullName()
    family_name: String,       // Package.Id.FamilyName()
    application_id: String,    // 来自 manifest <Application Id="...">
    display_name: String,      // 来自 manifest <DisplayName>
    install_location: PathBuf,
}
```

## Data flow

### `games()`

1. `platform::windows::get_packages()` → `Vec<XboxGame>` via `PackageManager::FindPackages()` (current user, no args).
2. Filter via `is_game(&family_name)` (allow-list, see § Filtering).
3. For each survivor, parse `<install_location>\AppxManifest.xml` via `manifest::parse` to fill `application_id` + `display_name`. If the manifest is missing or malformed, skip the package and `print_error` at debug level — never abort the whole scan for one bad entry.
4. Map each `XboxGame` to the public `Game` shape:
   - `_type = "xbox"`
   - `id = full_name`
   - `name = display_name`
   - `path = Some(install_location)`
   - `commands.install = None`
   - `commands.launch = Some(vec!["explorer.exe".into(), format!("shell:AppsFolder\\{}!{}", family_name, application_id)])`
   - `commands.uninstall = Some(vec!["__xbox__:remove_package".into(), full_name.clone()])` (sentinel — see § Manager dispatch)
   - `state.installed = true` (PackageManager only returns installed packages), `needs_update = false`, `downloading = false`, byte counts `None`.

### `find(id)`

1. `get_packages()`.
2. Linear scan for `full_name == id` (or build a `HashMap<full_name, XboxGame>` once and reuse — the `HashMap` is the preferred form because `find` and `games` share the scan).
3. Re-parse manifest if the cached `XboxGame` does not already carry `application_id` / `display_name`.
4. Return the constructed `Game`.

### `executable()`

1. `get_packages()` (or cached scan).
2. Find the entry whose `family_name` starts with `Microsoft.GamingApp_`.
3. Return `<install_location>\XboxApp.exe`.
4. If none found → `Error { kind: LauncherNotFound, ... }`.

## WinRT integration

### Dependencies

```toml
# Cargo.toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Management_Deployment",
    "ApplicationModel",
] }
quick-xml = "0.41"
```

`windows` is gated on `cfg(windows)` so Linux / macOS builds remain unaffected.

### Calling shape

```rust
use windows::{core::HSTRING, Management::Deployment::PackageManager};

let pm = PackageManager::new()?;
let packages = pm.FindPackages()?;  // current user; FindPackagesForUser is not in the projection
for pkg in packages {
    let id = pkg.Id()?;
    let full_name = id.FullName()?.to_string();
    let family_name = id.FamilyName()?.to_string();
    let install_location = pkg.InstalledPath()?.to_string_lossy().to_string();
    // ... filter + manifest parse
}
```

### `RemovePackageAsync` (uninstall)

```rust
let pm = PackageManager::new()?;
let full_name = HSTRING::from(&game.id);
let op = pm.RemovePackageAsync(full_name)?;
op.get()?;  // 阻塞直到异步完成
```

## Filtering

The function `is_game(family_name: &str) -> bool` returns true if any entry in the allow-list is a prefix of `family_name`. The allow-list has two parts:

**Microsoft-first-party Xbox / Gaming family-name prefixes** (high confidence):

```rust
const XBOX_FAMILY_PREFIXES: &[&str] = &[
    // Game platforms and clients
    "Microsoft.GamingApp",
    "Microsoft.XboxApp",
    "Microsoft.XboxGameOverlay",
    "Microsoft.XboxGamingOverlay",
    "Microsoft.XboxSpeechToTextOverlay",
    "Microsoft.XboxGameCallableUI",
    "Microsoft.XboxIdentityProvider",
    "Microsoft.Xbox.TCUI",
    // Microsoft Studios first-party IPs that ship as MSIX
    "Microsoft.MicrosoftSolitaireCollection",
    "Microsoft.MinecraftUWP",
    "Microsoft.Mojang",
    "Microsoft.Halo",
    "Microsoft.Forza",
    "Microsoft.AgeOfEmpires",
    "Microsoft.GearsOfWar",
    "Microsoft.FlightSimulator",
    "Microsoft.Oslo",  // internal codename retained
];
```

**Third-party publisher family-name prefixes commonly seen in Xbox Game Pass**:

```rust
const XBOX_THIRD_PARTY_PREFIXES: &[&str] = &[
    "Bethesda",
    "2K",
    "Activision",
    "ElectronicArts",
    "Rockstar",
    "Ubisoft",
    "SEGA",
    "SquareEnix",
    "BandaiNamco",
    "ParadoxInteractive",
    "CoffeeStainStudios",
    "DevolverDigital",
    "505Games",
    "DeepSilver",
    "FocusHomeInteractive",
    "KalypsoMedia",
];
```

Prefix matching is `family_name.starts_with(prefix)`. The allow-list is a `const &[&str]` so it is testable and editable in one place. Future expansion: append a new entry when a Game Pass title with an unknown prefix surfaces in issue reports.

> **Why not `Publisher == "CN=Microsoft"`?** That Publisher ID is used by all Microsoft Store apps including OneDrive, Mail, Office, Cortana, and Weather — accepting it would dump dozens of non-games into the result. Family-name prefix matching is the narrower, more reliable signal.

## Manager dispatch

Two functions in `src/manager/mod.rs` need a launcher-specific branch. The `launch_game` and `close_game` paths stay generic.

```rust
pub fn uninstall_game(game: &Game) -> Result<()> {
    // Xbox: WinRT API path — UWP packages cannot be uninstalled by spawning a child process.
    if game._type == "xbox" {
        return game_scanner::xbox::uninstall(game);
    }
    // ... existing process::Command path unchanged ...
}

pub fn get_processes(game: &Game) -> Option<Vec<u32>> {
    if game._type == "xbox" {
        return game_scanner::xbox::processes(game);
    }
    // ... existing sysinfo path unchanged ...
}
```

`launch_game` keeps using `process::Command::new("explorer.exe").arg("shell:AppsFolder\\...!...")`, which is the standard UWP launch idiom and needs no special handling.

`close_game` continues to call `get_processes` then `sysinfo::process(pid).kill()` — once `get_processes` returns Xbox PIDs the kill step is identical.

`install_game` falls through to the existing `ErrorKind::InvalidGame` branch because `commands.install = None`.

### Sentinel in `commands.uninstall`

The launch string is reusable as-is by `process::Command`. The uninstall string carries a sentinel because the actual uninstall is a Rust function call, not a process spawn. The sentinel is detected only in `xbox::uninstall` itself:

```rust
// xbox::uninstall 内部
let cmd = game.commands.uninstall.as_ref()
    .ok_or_else(|| Error::new(ErrorKind::InvalidGame, "missing uninstall command"))?;
if cmd.first().map(|s| s.as_str()) != Some("__xbox__:remove_package") {
    return Err(Error::new(ErrorKind::InvalidGame, "not an xbox game"));
}
let full_name = &cmd[1];
// ... call RemovePackageAsync(full_name) ...
```

The sentinel is private to the `xbox` module. No other launcher sees it. If `xbox::uninstall` is ever called with a `Game` whose `_type` is not `"xbox"`, the sentinel check fails fast with `InvalidGame`.

## Capability matrix

`gui/src-tauri/src/capability.rs::supports` gains one match arm per Op and one new entry in the `matrix` launcher array. The truth table:

| Op | Xbox |
| --- | --- |
| List | ✅ |
| Find | ✅ |
| Executable | ✅ |
| Install | ❌ |
| Launch | ✅ |
| Uninstall | ✅ |
| Processes | ✅ |
| Close | ✅ |

Xbox is the third launcher to support `uninstall` after Steam and Ubisoft, and the second after Steam to support `processes` + `close`.

## FFI surface

`game-scanner-ffi/src/lib.rs` adds one arm to four match blocks:

```rust
fn parse_launcher(name: &str) -> Option<GameType> {
    Some(match name {
        "amazongames" => GameType::AmazonGames,
        // ...
        "xbox"        => GameType::XboxGames,  // NEW
        _ => return None,
    })
}
```

`gs_list`, `gs_find`, `gs_executable` each gain `GameType::XboxGames => xbox::games()` / `xbox::find(&id)` / `xbox::executable()`. The `xbox` module is imported at the top of the file alongside the other eight.

`game_scanner.h` gets one line updated in the doc comment listing supported launchers.

## GUI surface

### Svelte (`gui/src/lib/types.ts`)

```ts
export type GameType =
  | "amazongames" | "blizzard" | "epicgames" | "gog"
  | "origin" | "riotgames" | "steam" | "ubisoft"
  | "xbox";  // NEW

export const ALL_GAME_TYPES: GameType[] = [
  // ... existing 8 ...
  "xbox",
];

export const LAUNCHER_LABELS: Record<GameType, string> = {
  // ... existing ...
  xbox: "Xbox / Microsoft Store",
};
```

`LauncherList.svelte` already iterates `ALL_GAME_TYPES` — no component change.

### Tauri (`gui/src-tauri/src/commands.rs`)

Three `match launcher` blocks add the `GameType::XboxGames => xbox::*` arm. No new command, no new capability allow-list entry.

### Tauri capability matrix

`capability.rs::supports` and `capability.rs::matrix` gain one Xbox row each. The existing `matrix_contains_all_eight_launchers` test updates to nine; a new `matrix_xbox_has_seven_ops` test locks in the Xbox capability count.

## Data flow (end-to-end)

```
Tauri command: list_games(XboxGames)
  ↓
commands::list_games → game_scanner::xbox::games()
  ↓
xbox::platform::windows::get_packages()
  ↓
PackageManager::FindPackages()                 [WinRT]
  ↓
for each pkg: filter via XBOX_FAMILY_PREFIXES     [in-memory]
  ↓
parse <install_location>\AppxManifest.xml        [quick-xml]
  ↓
build Vec<Game> with sentinel commands           [in-memory]
  ↓
serialize to JSON → return to frontend           [tauri IPC]
  ↓
LauncherList renders row "Xbox / Microsoft Store"
GamesTable populates with rows
ActionPanel shows 7 enabled buttons (Install disabled with tooltip)
```

## Error handling

WinRT errors are uniformly wrapped as `Error::new(ErrorKind::IO, format!("{ctx}: {e}"))`. Specific error codes map to slightly more descriptive `ErrorKind` variants where the user-facing message would be clearer:

| WinRT / source | `ErrorKind` | User-facing message shape |
| --- | --- | --- |
| `FindPackages` throws | `IO` | "Package enumeration failed: <winrt message>" |
| `RemovePackageAsync` → 0x80073CF9 | `InvalidManifest` | "Package is referenced by another app; uninstall dependents first" |
| `RemovePackageAsync` → 0x80073CFA | `LauncherNotFound` | "Package not installed or owned by another user" |
| `RemovePackageAsync` → 0x80073D02 / D06 | `IO` | "Package is currently in use" |
| `std::io::Error` reading manifest | `IO` | "AppxManifest.xml missing for <full_name>" — print, skip the package, continue scanning |
| `quick_xml::Error` parsing manifest | `InvalidManifest` | "Failed to parse <path>: <xml error>" — print, skip the package, continue scanning |
| `executable()` — no `Microsoft.GamingApp_*` package | `LauncherNotFound` | "Xbox app not installed" |

No new `ErrorKind` variant is added. New variants would propagate to the FFI / Node binding surfaces and the capability matrix; the existing ones are sufficient.

## Testing

### Unit tests

`src/xbox/platform/windows.rs` carries an inline `#[cfg(test)] mod tests`:

- `is_game_matches_known_xbox_prefixes` — feed each entry in `XBOX_FAMILY_PREFIXES` and `XBOX_THIRD_PARTY_PREFIXES`, assert true.
- `is_game_rejects_non_games` — assert `Microsoft.MicrosoftOfficeHub`, `Microsoft.WindowsStore`, `Microsoft.ScreenSketch`, `Microsoft.YourPhone`, `Microsoft.BingNews` all return false.
- `is_game_empty_string_returns_false`.

`src/xbox/manifest.rs`:

- `parse_extracts_application_id_and_display_name` — feed a minimal `<Package><Properties><DisplayName>X</DisplayName></Properties><Applications><Application Id="Game" Executable="X.exe" /></Applications></Package>` fixture, assert.
- `parse_returns_error_on_malformed_xml`.

### Integration test (`tests/list.rs`)

```rust
#[cfg(target_os = "windows")]
mod xbox {
    use super::*;
    #[test]
    fn games() -> Result<(), Error> {
        let games = game_scanner::xbox::games()
            .or::<Error>(Ok(Vec::<Game>::new()))
            .unwrap();
        assert_eq!(GAME_LIST_RETURN_TYPE, type_of(&games));
        Ok(())
    }
}
```

Same shape as the existing launchers; on a machine without Xbox / Microsoft Store installed, the call returns `LauncherNotFound`, which `or::<Error>(Ok(Vec::new()))` swallows.

### Capability matrix tests (`gui/src-tauri/src/capability.rs::tests`)

- Update `every_launcher_supports_{list,find,executable,launch}` to include `XboxGames`.
- New `xbox_supports_uninstall_processes_close`.
- New `xbox_does_not_support_install`.
- Update `matrix_contains_all_eight_launchers` → `matrix_contains_all_nine_launchers` (renamed and updated count).
- New `matrix_xbox_has_seven_ops` to lock the count.

### FFI tests (`game-scanner-ffi/src/lib.rs::tests`)

- Update `parse_launcher_round_trip` loop to include `"xbox"`.

### Manual verification checklist (PR description + `docs/xbox.md`)

On a Windows machine with Microsoft Store / Xbox App installed and at least one Game Pass title:

1. `gs_list("xbox")` returns ≥ 1 entry with `_type == "xbox"`.
2. `gs_find("xbox", "<known PackageFullName>")` returns the same entry.
3. `gs_executable("xbox")` returns a path ending in `XboxApp.exe` that exists.
4. `gs_launch("<Game JSON>")` opens the game within ~5 s.
5. `gs_get_processes("<Game JSON>")` returns ≥ 1 PID for a Win32-packaged game.
6. `gs_close("<Game JSON>")` terminates the launched processes.
7. `gs_uninstall("<Game JSON>")` opens the standard Windows Apps & Features uninstall prompt; completing it removes the package.
8. `pnpm tauri dev` shows "Xbox / Microsoft Store" in the launcher list with a badge of `7`.

No CI environment has the prerequisite Microsoft Store + Game Pass install; the checklist is for contributor verification on a real machine.

## Dependencies

```toml
# Cargo.toml — new section
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Management_Deployment",
    "ApplicationModel",
] }
quick-xml = "0.41"
```

No new top-level dependencies. `windows` and `quick-xml` are Windows-gated and do not affect Linux / macOS builds.

## Open risks

- **Compilation time.** `windows` crate adds ~3–5 minutes to a clean Windows build. Mitigated by gating on `cfg(windows)` so non-Windows builds stay fast, and by using `windows = "0.61"` — the same version already in the workspace's `tauri` lockfile, so no second copy of the WinRT projections is compiled.
- **Allow-list coverage.** Long-tail indie Game Pass titles whose family names do not match any prefix are silently skipped. The list is editable in one place; expansion is low-risk but not free.
- **Pure-UWP process detection.** v1 matches by exe path / cwd under `install_location`. Pure UWP apps whose host process is `svchost.exe` or a generic runtime broker may not be matched. The README / `docs/xbox.md` note this as a known limitation.
- **Windows 10 vs Windows 11 manifest schema drift.** `quick-xml` does not validate the schema; it just reads the elements we care about. New optional elements in the schema are ignored without error.
- **`windows` crate API churn.** Microsoft occasionally restructures WinRT projections. Pinning to `0.61` and watching the crate's changelog is the recommended forward path.
- **Permissions.** `FindPackages` (current user) works without elevation. `RemovePackageAsync` triggers the standard Windows Apps & Features confirmation prompt for per-user packages — no UAC escalation needed for normal users.

## Implementation order (high level)

1. `Cargo.toml` — add `windows` and `quick-xml` (Windows-gated).
2. `src/prelude.rs` — `GameType::XboxGames` + `to_string` / `From<String>`.
3. `src/xbox/manifest.rs` — `parse` + unit tests.
4. `src/xbox/platform/windows.rs` — `get_packages` + `is_game` + WinRT error wrapping + unit tests.
5. `src/xbox/platform/mod.rs` + linux / macos stubs.
6. `src/xbox/mod.rs` — `games` / `find` / `executable` / `uninstall` / `processes`.
7. `src/lib.rs` — `pub mod xbox;`.
8. `src/manager/mod.rs` — Xbox branches in `uninstall_game` and `get_processes`.
9. `tests/list.rs` — `mod xbox` smoke test.
10. `game-scanner-ffi/src/lib.rs` + `include/game_scanner.h` — match arms + doc comment.
11. `gui/src-tauri/src/commands.rs` — three match arms.
12. `gui/src-tauri/src/capability.rs` — `supports` + `matrix` + tests.
13. `gui/src/lib/types.ts` — `GameType` union, `ALL_GAME_TYPES`, `LAUNCHER_LABELS`.
14. `README.md` — five table rows.
15. `docs/xbox.md` — launcher detail document.
16. `cargo build -p game-scanner` + `cargo test -p game-scanner` clean.
17. `cargo build -p game-scanner-ffi` + FFI tests pass.
18. `pnpm tauri dev` boots; manual checklist § Testing runs on a real Windows machine.

## Reference

- WinRT `PackageManager` reference: https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager
- Appx package schema: https://learn.microsoft.com/en-us/uwp/schemas/appxpackage/uapmanifestschema/element-package
- `windows` crate: https://crates.io/crates/windows
- Existing launcher docs to mirror: `docs/steam.md`, `docs/blizzard.md`, `docs/epicgames.md`.
- Existing capability matrix precedent: `docs/superpowers/specs/2026-07-10-tauri-v2-gui-demo-design.md`.