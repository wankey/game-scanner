# Xbox / Microsoft Store Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a new `xbox` launcher variant to `game-scanner` that enumerates Microsoft Store and Xbox App for PC (Game Pass) games on Windows via the WinRT `PackageManager` API, wired through the FFI and Tauri GUI layers.

**Architecture:** A new `src/xbox/` module mirrors the existing launcher pattern — `games()` / `find()` / `executable()` plus launcher-specific `uninstall()` and `processes()` for ops that can't run as a child process. Filtering is a curated FamilyName prefix allow-list (no `Publisher == "CN=Microsoft"`). Launch reuses the generic `process::Command` path with a `shell:AppsFolder\…!…` URI; uninstall and process enumeration dispatch through `manager/mod.rs` to `xbox::uninstall` / `xbox::processes` which call the WinRT API directly.

**Tech Stack:** Rust (`windows` crate for WinRT, `quick-xml` for AppxManifest.xml), Svelte 5 + TypeScript, Tauri v2. Windows-only; Linux/macOS return `LauncherNotFound` like `amazon`/`ubisoft` today.

**Spec:** `docs/superpowers/specs/2026-07-10-xbox-support-design.md`

---

## Capability matrix (single source of truth)

Updated cell — used verbatim by `gui/src-tauri/src/capability.rs` and locked by tests:

| Launcher   | List | Find | Exec | Install | Launch | Uninstall | Processes | Close |
|------------|------|------|------|---------|--------|-----------|-----------|-------|
| Amazon     | ✅   | ✅   | ✅   | ❌      | ✅     | ❌        | ❌        | ❌    |
| Blizzard   | ✅   | ✅   | ✅   | ❌      | ✅     | ❌        | ❌        | ❌    |
| EpicGames  | ✅   | ✅   | ✅   | ❌      | ✅     | ❌        | ❌        | ❌    |
| GOG        | ✅   | ✅   | ✅   | ❌      | ✅     | ❌        | ❌        | ❌    |
| Origin     | ✅   | ✅   | ✅   | ✅      | ✅     | ❌        | ❌        | ❌    |
| RiotGames  | ✅   | ✅   | ✅   | ❌      | ✅     | ✅        | ❌        | ❌    |
| Steam      | ✅   | ✅   | ✅   | ✅      | ✅     | ✅        | ✅        | ✅    |
| Ubisoft    | ✅   | ✅   | ✅   | ✅      | ✅     | ✅        | ❌        | ❌    |
| **XboxGames** | ✅ | ✅   | ✅   | ❌      | ✅     | ✅        | ✅        | ✅    |

Seven new operations for Xbox: `List`, `Find`, `Executable`, `Launch`, `Uninstall`, `Processes`, `Close`. `Install` remains ❌ (Store UI / subscription authorization only).

---

## File structure

Files created (C) or modified (M):

```
M Cargo.toml                                          # + windows + quick-xml (Windows-gated)
M src/lib.rs                                          # + pub mod xbox;
M src/prelude.rs                                      # + GameType::XboxGames
M src/manager/mod.rs                                  # + match branches for uninstall_game / get_processes
C src/xbox/mod.rs                                     # public API
C src/xbox/manifest.rs                                # AppxManifest.xml parser
C src/xbox/process.rs                                 # UWP process enumeration
C src/xbox/platform/mod.rs                            # cfg_attr dispatch
C src/xbox/platform/windows.rs                        # WinRT PackageManager
C src/xbox/platform/linux.rs                          # LauncherNotFound stub
C src/xbox/platform/macos.rs                          # LauncherNotFound stub
M tests/list.rs                                       # + mod xbox (Windows-only)
M game-scanner-ffi/src/lib.rs                         # + xbox arm in 4 match blocks
M game-scanner-ffi/include/game_scanner.h             # doc comment
M gui/src-tauri/src/commands.rs                       # + 3 xbox match arms
M gui/src-tauri/src/capability.rs                     # + Xbox row + tests
M gui/src/lib/types.ts                                # + "xbox" union, ALL_GAME_TYPES, LAUNCHER_LABELS
M README.md                                           # + 5 table rows
C docs/xbox.md                                        # launcher detail
```

---

## Phase 1 — Foundation: enum and dependencies

### Task 1: Add `GameType::XboxGames` to the prelude

**Files:**
- Modify: `src/prelude.rs` (the `GameType` enum + `to_string` / `From<String>` impls)

- [ ] **Step 1: Add `XboxGames` variant to the enum**

In `src/prelude.rs`, change the `GameType` enum to:

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GameType {
    AmazonGames,
    Blizzard,
    EpicGames,
    GOG,
    Origin,
    RiotGames,
    Steam,
    Ubisoft,
    XboxGames,
}
```

- [ ] **Step 2: Add the string mapping in `to_string`**

In `src/prelude.rs`, change the `to_string` body to:

```rust
impl GameType {
    pub fn to_string(&self) -> String {
        match self {
            Self::AmazonGames => "amazongames",
            Self::Blizzard => "blizzard",
            Self::EpicGames => "epicgames",
            Self::GOG => "gog",
            Self::Origin => "origin",
            Self::RiotGames => "riotgames",
            Self::Steam => "steam",
            Self::Ubisoft => "ubisoft",
            Self::XboxGames => "xbox",
        }
        .to_string()
    }
}
```

- [ ] **Step 3: Add the inverse in `From<String>`**

In `src/prelude.rs`, change the `From<String>` body to:

```rust
impl From<String> for GameType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "amazongames" => Self::AmazonGames,
            "blizzard" => Self::Blizzard,
            "epicgames" => Self::EpicGames,
            "gog" => Self::GOG,
            "origin" => Self::Origin,
            "riotgames" => Self::RiotGames,
            "steam" => Self::Steam,
            "ubisoft" => Self::Ubisoft,
            "xbox" => Self::XboxGames,
            _ => panic!("invalid game type"),
        }
    }
}
```

- [ ] **Step 4: Build to verify**

Run: `cargo build -p game-scanner`
Expected: compiles. (No new code paths exist yet, so the new variant is just an extra enum arm — every existing `match` over `GameType` would still need an arm added later, but `to_string` and `From<String>` are exhaustive, so compilation passes now.)

- [ ] **Step 5: Run existing tests**

Run: `cargo test -p game-scanner`
Expected: all green, same count as before. No test was added yet.

- [ ] **Step 6: Commit**

```bash
git add src/prelude.rs
git commit -m "feat(prelude): add GameType::XboxGames variant"
```

---

### Task 2: Add Windows-gated `windows` and `quick-xml` dependencies

**Files:**
- Modify: `Cargo.toml` (add Windows-only deps)

- [ ] **Step 1: Add the `[target.'cfg(windows)'.dependencies]` block**

In `Cargo.toml`, add a new block after the existing `[target.'cfg(windows)'.dependencies]` block:

```toml
[target.'cfg(windows)'.dependencies]
case = { version = "1.0.0" }
winreg = { version = "0.56.0" }
rusqlite = { version = "0.40.1", features = ["bundled-windows"] }
windows = { version = "0.61", features = [
    "Management_Deployment",
    "ApplicationModel",
] }
quick-xml = "0.41"
```

(The `case`, `winreg`, and `rusqlite` lines are the existing block — leave them as they are. Only the `windows` and `quick-xml` lines are new.)

> **Why these versions (revised after code review):**
> - `windows = "0.61"` is the version the workspace's `tauri` crate already pulls in. Reusing it avoids a second copy of the WinRT projections and a redundant recompile.
> - The `windows` Rust projection does NOT expose `IPackageManager::FindPackagesForUser` or `Package::InstallLocation` (string) — only the SID-based variants and `Package::InstalledPath` / `InstalledLocation`. Task 5 below has been adjusted to use `pm.FindPackages()` (current user, no args) and `pkg.InstalledPath()` (HSTRING). The two feature flags remain the minimum needed for `PackageManager` + `Package` to be in scope.
> - `default-features = false` is **not** set. The `windows` crate's only default feature is `std`; disabling it removes `windows-core/std` which gates `std::error::Error` impls we need elsewhere. There is no measurable compile-time cost to keeping it.
> - `quick-xml = "0.41"` matches the version already in the lockfile (transitively pulled by another crate), avoiding a second `quick-xml` and a second recompile. 0.41's `Reader` API is also the API Task 3's parser code uses.

- [ ] **Step 2: Build the Windows target**

Run: `cargo build -p game-scanner --target x86_64-pc-windows-msvc`
Expected: compiles. The `windows` crate is a heavy dependency; first build may take 3–5 minutes. If this isn't a Windows machine, skip the run and rely on the existing Windows CI to catch issues.

- [ ] **Step 3: Build the host target to confirm no impact on non-Windows**

Run: `cargo build -p game-scanner`
Expected: compiles. The new deps are gated, so non-Windows hosts are unaffected.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build(deps): add windows-rs and quick-xml for Xbox/Store support"
```

---

## Phase 2 — `xbox` module internals

### Task 3: Manifest parser (the leaf dependency)

**Files:**
- Create: `src/xbox/manifest.rs`
- Create: `src/xbox/mod.rs` (stub; filled out in Task 7)

- [ ] **Step 1: Create `src/xbox/mod.rs` with a stub `mod manifest;`**

Create `src/xbox/mod.rs` with exactly:

```rust
mod manifest;
```

(We add the rest of the module in Task 7. The stub lets us compile the parser in isolation.)

- [ ] **Step 2: Create the `manifest.rs` module with the public types and parser**

Create `src/xbox/manifest.rs` with:

```rust
use crate::error::{Error, ErrorKind, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::path::Path;

pub struct ParsedManifest {
    pub application_id: String,
    pub display_name: String,
}

pub fn parse(path: &Path) -> Result<ParsedManifest> {
    let xml = std::fs::read_to_string(path).map_err(|e| {
        Error::new(
            ErrorKind::IO,
            format!("AppxManifest.xml missing at {}: {}", path.display(), e),
        )
    })?;

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut application_id = String::new();
    let mut display_name = String::new();
    let mut current_app_open = false;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();
                if name_bytes == b"Application" {
                    current_app_open = true;
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"Id" {
                            application_id = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                } else if name_bytes == b"DisplayName" && display_name.is_empty() {
                    // capture text in the next Text event
                }
            }
            Ok(Event::Empty(e)) => {
                let name_bytes = e.name().as_ref();
                if name_bytes == b"Application" && application_id.is_empty() {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"Id" {
                            application_id = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if current_app_open {
                    // ignore text inside <Application> — we only want attributes
                } else if display_name.is_empty() {
                    let raw = t.unescape().unwrap_or_default();
                    let trimmed = raw.trim();
                    if !trimmed.is_empty() {
                        display_name = trimmed.to_string();
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"Application" {
                    current_app_open = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::InvalidManifest,
                    format!("Failed to parse {}: {}", path.display(), e),
                ));
            }
            _ => {}
        }
        buf.clear();
    }

    if application_id.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidManifest,
            format!("No <Application Id=\"...\"> found in {}", path.display()),
        ));
    }

    Ok(ParsedManifest {
        application_id,
        display_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_manifest(body: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(body.as_bytes()).unwrap();
        f
    }

    #[test]
    fn parse_extracts_application_id_and_display_name() {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<Package>
  <Properties>
    <DisplayName>Forza Horizon 5</DisplayName>
  </Properties>
  <Applications>
    <Application Id="Forza" Executable="ForzaHorizon5.exe" />
  </Applications>
</Package>"#;
        let f = write_manifest(body);
        let parsed = parse(f.path()).unwrap();
        assert_eq!(parsed.application_id, "Forza");
        assert_eq!(parsed.display_name, "Forza Horizon 5");
    }

    #[test]
    fn parse_returns_error_when_application_missing() {
        let body = r#"<?xml version="1.0"?><Package><Properties><DisplayName>X</DisplayName></Properties></Package>"#;
        let f = write_manifest(body);
        assert!(parse(f.path()).is_err());
    }

    #[test]
    fn parse_returns_error_on_malformed_xml() {
        let body = "not xml at all";
        let f = write_manifest(body);
        assert!(parse(f.path()).is_err());
    }
}
```

- [ ] **Step 3: Add `tempfile` to dev-dependencies**

In `Cargo.toml`, in the `[dev-dependencies]` block, add:

```toml
[dev-dependencies]
criterion = { version = "0.8.2" }
tempfile = "3"
```

- [ ] **Step 4: Run the new tests**

Run: `cargo test -p game-scanner --lib xbox::manifest`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/xbox/manifest.rs src/xbox/mod.rs Cargo.toml Cargo.lock
git commit -m "feat(xbox): add AppxManifest.xml parser with unit tests"
```

---

### Task 4: Linux / macOS stubs

**Files:**
- Create: `src/xbox/platform/linux.rs`
- Create: `src/xbox/platform/macos.rs`
- Create: `src/xbox/platform/mod.rs` (cfg_attr dispatch only; `windows.rs` arrives in Task 5)

- [ ] **Step 1: Create `src/xbox/platform/mod.rs`**

Create `src/xbox/platform/mod.rs` with:

```rust
use crate::error::{Error, ErrorKind, Result};
use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use self::linux::*;
#[cfg(target_os = "macos")]
pub use self::macos::*;
#[cfg(target_os = "windows")]
pub use self::windows::*;

#[allow(dead_code)]
pub fn get_launcher_executable() -> Result<PathBuf> {
    Err(Error::new(
        ErrorKind::LauncherNotFound,
        "Xbox / Microsoft Store is not supported on this platform",
    ))
}
```

(`get_launcher_executable` is the only function exposed cross-platform at the platform layer. The WinRT scan, manifest parse, and process enumeration are not on this layer — they live in `xbox/mod.rs` and `xbox/process.rs` and gate on `cfg(target_os = "windows")` directly.)

- [ ] **Step 2: Create the Linux stub `src/xbox/platform/linux.rs`**

Create `src/xbox/platform/linux.rs` with:

```rust
// Linux is not supported. The dispatcher in mod.rs already returns
// LauncherNotFound from get_launcher_executable. This file exists so
// the cfg_attr machinery is symmetric with Windows / macOS.
```

- [ ] **Step 3: Create the macOS stub `src/xbox/platform/macos.rs`**

Create `src/xbox/platform/macos.rs` with:

```rust
// macOS is not supported. See linux.rs for the explanation.
```

- [ ] **Step 4: Create the Windows stub `src/xbox/platform/windows.rs`**

Create `src/xbox/platform/windows.rs` with a placeholder so the platform layer compiles. We will replace it in Task 5.

```rust
use crate::error::{Error, ErrorKind, Result};
use std::path::PathBuf;

pub fn get_launcher_executable() -> Result<PathBuf> {
    Err(Error::new(
        ErrorKind::LauncherNotFound,
        "Xbox / Microsoft Store enumeration not yet implemented",
    ))
}
```

- [ ] **Step 5: Wire the platform module into `src/xbox/mod.rs`**

Update `src/xbox/mod.rs` to:

```rust
mod manifest;
mod platform;
```

- [ ] **Step 6: Build all targets**

Run: `cargo build -p game-scanner`
Expected: compiles on the host target. On non-Windows hosts, `windows.rs` is not included; on Windows hosts, the stub `get_launcher_executable` compiles.

- [ ] **Step 7: Commit**

```bash
git add src/xbox/mod.rs src/xbox/platform/
git commit -m "feat(xbox): scaffold platform module with linux/macos stubs"
```

---

### Task 5: WinRT enumeration on Windows

**Files:**
- Modify: `src/xbox/platform/windows.rs` (replace stub with real implementation + tests)
- Create: `src/xbox/types.rs` (private helper struct — or inline if simpler)

- [ ] **Step 1: Add the `is_game` allow-list constant**

In `src/xbox/platform/windows.rs`, replace the entire file with:

```rust
use crate::error::{Error, ErrorKind, Result};
use std::path::PathBuf;
use windows::core::HSTRING;
use windows::Management::Deployment::PackageManager;

const XBOX_FAMILY_PREFIXES: &[&str] = &[
    // Microsoft-first-party Xbox / Gaming clients and overlays
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
    "Microsoft.Oslo",
    // Third-party publishers commonly appearing in Xbox Game Pass
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

fn is_game(family_name: &str) -> bool {
    XBOX_FAMILY_PREFIXES
        .iter()
        .any(|prefix| family_name.starts_with(prefix))
}

pub struct XboxPackage {
    pub full_name: String,
    pub family_name: String,
    pub install_location: PathBuf,
}

pub fn get_packages() -> Result<Vec<XboxPackage>> {
    let pm = PackageManager::new()
        .map_err(|e| Error::new(ErrorKind::IO, format!("PackageManager::new failed: {e}")))?;

    // The `windows` crate's WinRT projection does NOT expose
    // `IPackageManager::FindPackagesForUser` — only the SID-based
    // overloads. We use `FindPackages()` which enumerates the current
    // user (no args), which is what we want.
    let packages = pm
        .FindPackages()
        .map_err(|e| Error::new(ErrorKind::IO, format!("FindPackages failed: {e}")))?;

    let mut out = Vec::new();
    for pkg in packages {
        let id = match pkg.Id() {
            Ok(id) => id,
            Err(_) => continue,
        };
        let family_name = match id.FamilyName() {
            Ok(s) => s.to_string(),
            Err(_) => continue,
        };

        if !is_game(&family_name) {
            continue;
        }

        let full_name = match id.FullName() {
            Ok(s) => s.to_string(),
            Err(_) => continue,
        };

        // The projection exposes `InstalledPath` (HSTRING) and
        // `InstalledLocation` (StorageFolder). `InstalledPath` is the
        // direct string and avoids the StorageFolder round-trip.
        let install_location = match pkg.InstalledPath() {
            Ok(s) => PathBuf::from(s.to_string_lossy()),
            Err(_) => continue,
        };

        out.push(XboxPackage {
            full_name,
            family_name,
            install_location,
        });
    }

    Ok(out)
}

pub fn get_launcher_executable() -> Result<PathBuf> {
    let packages = get_packages()?;
    let gaming_app = packages
        .iter()
        .find(|p| p.family_name.starts_with("Microsoft.GamingApp"))
        .ok_or_else(|| {
            Error::new(
                ErrorKind::LauncherNotFound,
                "Xbox app is not installed (no Microsoft.GamingApp_* package found)",
            )
        })?;
    let exe = gaming_app.install_location.join("XboxApp.exe");
    if !exe.exists() {
        return Err(Error::new(
            ErrorKind::LauncherNotFound,
            format!("XboxApp.exe missing at {}", exe.display()),
        ));
    }
    Ok(exe)
}

pub fn remove_package(full_name: &str) -> Result<()> {
    let pm = PackageManager::new()
        .map_err(|e| Error::new(ErrorKind::IO, format!("PackageManager::new failed: {e}")))?;
    let hn = HSTRING::from(full_name);
    let op = pm
        .RemovePackageAsync(hn)
        .map_err(|e| Error::new(ErrorKind::IO, format!("RemovePackageAsync failed: {e}")))?;
    let result = op
        .get()
        .map_err(|e| Error::new(ErrorKind::IO, format!("RemovePackageAsync wait failed: {e}")))?;
    if result.is_none() {
        return Err(Error::new(
            ErrorKind::IO,
            "RemovePackageAsync returned null completion",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_game_matches_first_party_xbox_clients() {
        for name in [
            "Microsoft.GamingApp_8wekyb3d8bbwe",
            "Microsoft.XboxApp_8wekyb3d8bbwe",
            "Microsoft.MinecraftUWP_8wekyb3d8bbwe",
            "Microsoft.Halo_8wekyb3d8bbwe",
            "Microsoft.MicrosoftSolitaireCollection_8wekyb3d8bbwe",
        ] {
            assert!(is_game(name), "should match: {name}");
        }
    }

    #[test]
    fn is_game_matches_third_party_publishers() {
        for name in [
            "Bethesda.SomeGame_abc123",
            "2K.NBA2K25_def456",
            "Activision.CallOfDuty_ghi789",
            "Ubisoft.AssassinsCreed_jkl012",
        ] {
            assert!(is_game(name), "should match: {name}");
        }
    }

    #[test]
    fn is_game_rejects_non_games() {
        for name in [
            "Microsoft.MicrosoftOfficeHub_8wekyb3d8bbwe",
            "Microsoft.WindowsStore_8wekyb3d8bbwe",
            "Microsoft.ScreenSketch_8wekyb3d8bbwe",
            "Microsoft.YourPhone_8wekyb3d8bbwe",
            "Microsoft.BingNews_8wekyb3d8bbwe",
        ] {
            assert!(!is_game(name), "should NOT match: {name}");
        }
    }

    #[test]
    fn is_game_empty_string_returns_false() {
        assert!(!is_game(""));
    }
}
```

- [ ] **Step 2: Build the Windows target**

Run: `cargo build -p game-scanner --target x86_64-pc-windows-msvc`
Expected: compiles. (Skip if not on Windows; the build will fail elsewhere — a CI check catches this.)

- [ ] **Step 3: Run the new tests on Windows**

Run: `cargo test -p game-scanner --lib xbox::platform::windows`
Expected: 4 `is_game` tests pass. The `get_packages` and `remove_package` functions are tested manually per the spec's manual verification checklist.

- [ ] **Step 4: Commit**

```bash
git add src/xbox/platform/windows.rs
git commit -m "feat(xbox): WinRT PackageManager enumeration on Windows"
```

---

### Task 6: Process enumeration (UWP-friendly)

**Files:**
- Create: `src/xbox/process.rs`

- [ ] **Step 1: Create `src/xbox/process.rs` with the cross-platform stub**

Create `src/xbox/process.rs` with:

```rust
use crate::prelude::Game;
use sysinfo::{Pid, System};

#[cfg(target_os = "windows")]
pub fn pids(game: &Game) -> Vec<u32> {
    let install_path = match game.path.as_ref() {
        Some(p) => p.to_string_lossy().to_string(),
        None => return Vec::new(),
    };

    let sys = System::new_all();
    sys.processes()
        .iter()
        .filter(|(_, p)| {
            let exe_contains = p
                .exe()
                .map(|e| e.to_string_lossy().contains(&install_path))
                .unwrap_or(false);
            let cwd_contains = p
                .cwd()
                .map(|c| c.to_string_lossy().contains(&install_path))
                .unwrap_or(false);
            exe_contains || cwd_contains
        })
        .map(|(pid, _)| pid.as_u32())
        .collect()
}

#[cfg(not(target_os = "windows"))]
pub fn pids(_game: &Game) -> Vec<u32> {
    Vec::new()
}
```

- [ ] **Step 2: Wire `process` into `src/xbox/mod.rs`**

Update `src/xbox/mod.rs` to:

```rust
mod manifest;
mod platform;
mod process;
```

- [ ] **Step 3: Build**

Run: `cargo build -p game-scanner`
Expected: compiles on every host.

- [ ] **Step 4: Commit**

```bash
git add src/xbox/process.rs src/xbox/mod.rs
git commit -m "feat(xbox): UWP process enumeration by install_location"
```

---

### Task 7: Public API in `src/xbox/mod.rs`

**Files:**
- Modify: `src/xbox/mod.rs`

- [ ] **Step 1: Replace the stub with the full public API**

Replace `src/xbox/mod.rs` with:

```rust
mod manifest;
mod platform;
mod process;

use crate::{
    error::{Error, ErrorKind, Result},
    prelude::Game,
};
use std::path::PathBuf;

use self::platform::windows::XboxPackage;

/// Returns the path to `XboxApp.exe` inside the `Microsoft.GamingApp_*`
/// package, or `LauncherNotFound` if the Xbox App is not installed.
pub fn executable() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return platform::windows::get_launcher_executable();
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(Error::new(
            ErrorKind::LauncherNotFound,
            "Xbox / Microsoft Store is not supported on this platform",
        ))
    }
}

/// Returns every installed Microsoft Store / Xbox App game this machine
/// exposes. The list is filtered by an internal allow-list of FamilyName
/// prefixes (see `platform::windows::XBOX_FAMILY_PREFIXES`).
pub fn games() -> Result<Vec<Game>> {
    #[cfg(target_os = "windows")]
    {
        return scan_all();
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(Error::new(
            ErrorKind::LauncherNotFound,
            "Xbox / Microsoft Store is not supported on this platform",
        ))
    }
}

/// Looks up a single Xbox game by its `PackageFullName`.
pub fn find(id: &str) -> Result<Game> {
    let all = games()?;
    all.into_iter()
        .find(|g| g.id == id)
        .ok_or_else(|| Error::new(ErrorKind::GameNotFound, format!("Xbox game with id ({id}) not found")))
}

/// Uninstalls the supplied Xbox game via the WinRT `RemovePackageAsync` API.
pub fn uninstall(game: &Game) -> Result<()> {
    let cmd = game.commands.uninstall.as_ref().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidGame,
            "xbox::uninstall called with a Game that has no uninstall command",
        )
    })?;
    if cmd.first().map(|s| s.as_str()) != Some("__xbox__:remove_package") {
        return Err(Error::new(
            ErrorKind::InvalidGame,
            "xbox::uninstall called with a non-xbox game (sentinel mismatch)",
        ));
    }
    let full_name = cmd.get(1).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidGame,
            "xbox::uninstall command missing the package full name",
        )
    })?;

    #[cfg(target_os = "windows")]
    {
        platform::windows::remove_package(full_name)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = full_name;
        Err(Error::new(
            ErrorKind::LauncherNotFound,
            "Xbox / Microsoft Store is not supported on this platform",
        ))
    }
}

/// Returns the PIDs of processes whose exe path or cwd falls under the
/// game's `install_location`. Pure-UWP processes whose host is
/// `svchost.exe` or a generic runtime broker are not detected — a known
/// v1 limitation.
pub fn processes(game: &Game) -> Option<Vec<u32>> {
    Some(process::pids(game))
}

#[cfg(target_os = "windows")]
fn scan_all() -> Result<Vec<Game>> {
    let packages = platform::windows::get_packages()?;
    let mut out = Vec::new();
    for pkg in packages {
        match build_game(pkg) {
            Ok(g) => out.push(g),
            Err(e) => {
                // Per spec: one bad manifest must not abort the whole scan.
                crate::error::print_error(&e);
            }
        }
    }
    Ok(out)
}

#[cfg(target_os = "windows")]
fn build_game(pkg: XboxPackage) -> Result<Game> {
    let manifest_path = pkg.install_location.join("AppxManifest.xml");
    let parsed = manifest::parse(&manifest_path)?;

    let mut game = Game::default();
    game._type = String::from("xbox");
    game.id = pkg.full_name.clone();
    game.name = parsed.display_name;
    game.path = Some(pkg.install_location.clone());

    game.commands.install = None;
    game.commands.launch = Some(vec![
        String::from("explorer.exe"),
        format!("shell:AppsFolder\\{}!{}", pkg.family_name, parsed.application_id),
    ]);
    game.commands.uninstall = Some(vec![
        String::from("__xbox__:remove_package"),
        pkg.full_name,
    ]);

    game.state.installed = true;

    Ok(game)
}
```

- [ ] **Step 2: Build**

Run: `cargo build -p game-scanner`
Expected: compiles. On Windows the WinRT path is exercised; on other hosts, the cfg-gated branches return `LauncherNotFound`.

- [ ] **Step 3: Run the existing `xbox::manifest` and `xbox::platform::windows` tests**

Run: `cargo test -p game-scanner --lib xbox`
Expected: 3 manifest tests + 4 platform::windows tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/xbox/mod.rs
git commit -m "feat(xbox): public API (games/find/executable/uninstall/processes)"
```

---

### Task 8: Expose the `xbox` module at the crate root

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Add `pub mod xbox;`**

Edit `src/lib.rs` to:

```rust
pub mod amazon;
pub mod blizzard;
pub mod epicgames;
mod error;
pub mod gog;
pub mod manager;
pub mod origin;
pub mod prelude;
pub mod riotgames;
pub mod steam;
pub mod ubisoft;
pub mod xbox;
mod utils;
```

(Insert `pub mod xbox;` between `ubisoft` and `mod utils;` — alphabetical ordering.)

- [ ] **Step 2: Build and run all tests**

Run: `cargo build -p game-scanner && cargo test -p game-scanner`
Expected: compiles; same tests pass as before, plus the new xbox tests.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat(xbox): expose xbox module at crate root"
```

---

## Phase 3 — Manager dispatch

### Task 9: Wire Xbox branches into `manager`

**Files:**
- Modify: `src/manager/mod.rs`

- [ ] **Step 1: Update `install_game` to keep its existing error path for Xbox**

In `src/manager/mod.rs`, no change is needed to `install_game` — Xbox games have `commands.install = None`, so the existing `ErrorKind::InvalidGame` branch fires naturally. (No edit step.)

- [ ] **Step 2: Update `get_processes` to dispatch on `_type`**

In `src/manager/mod.rs`, replace `get_processes` with:

```rust
pub fn get_processes(game: &Game) -> Option<Vec<u32>> {
    if game._type == "xbox" {
        return game_scanner::xbox::processes(game);
    }
    let sys = System::new_all();
    let processes = sys.processes();

    let str_array_contains = |path: &[std::ffi::OsString], value: &str| {
        path.iter().any(|arg| arg.to_string_lossy().contains(value))
    };
    let path_contains = |path: Option<&Path>, value: &str| {
        path.is_some_and(|path| path.display().to_string().contains(value))
    };

    let mut list = Vec::new();

    let path = game.path.as_ref()?.display().to_string();

    for (pid, process) in processes {
        let should_kill = path_contains(process.cwd(), &path)
            || path_contains(process.exe(), &path)
            || str_array_contains(process.cmd(), &path);

        if !should_kill {
            continue;
        }

        list.push(pid.as_u32());
    }

    Some(list)
}
```

- [ ] **Step 3: Update `uninstall_game` to dispatch on `_type`**

In `src/manager/mod.rs`, replace `uninstall_game` with:

```rust
pub fn uninstall_game(game: &Game) -> Result<()> {
    if game._type == "xbox" {
        return game_scanner::xbox::uninstall(game);
    }

    let mut command = process::Command::new("");

    if game.commands.uninstall.is_none() {
        return Err(Error::new(
            ErrorKind::InvalidGame,
            "Error to uninstall a game without uninstall command",
        ));
    }

    let launch_command = game.commands.uninstall.as_ref().unwrap();

    for (index, arg) in launch_command.iter().enumerate() {
        if index == 0 {
            command = process::Command::new(arg);
        } else {
            command.arg(arg);
        }
    }

    if cfg!(debug_assertions) {
        println!("Executing the command: {:?}", command);
    }

    let process = command
        .stdin(process::Stdio::null())
        .stdout(process::Stdio::null())
        .spawn()
        .expect(&format!("Couldn't uninstall {}", game.name));

    if cfg!(debug_assertions) {
        println!("Uninstalling {} [{}]", game.name, process.id());
    }

    Ok(())
}
```

- [ ] **Step 4: Build and test**

Run: `cargo build -p game-scanner && cargo test -p game-scanner`
Expected: compiles; existing tests still pass (no behavioral change for non-xbox games).

- [ ] **Step 5: Commit**

```bash
git add src/manager/mod.rs
git commit -m "feat(manager): dispatch uninstall / get_processes to xbox module"
```

---

## Phase 4 — Integration test

### Task 10: Add Xbox smoke test to `tests/list.rs`

**Files:**
- Modify: `tests/list.rs`

- [ ] **Step 1: Add a Windows-only `mod xbox` block**

In `tests/list.rs`, add the following module just before the `type_of` helper at the end:

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

- [ ] **Step 2: Run the integration test**

Run: `cargo test -p game-scanner --test list xbox`
Expected: 1 test passes (or is filtered out on non-Windows hosts).

- [ ] **Step 3: Commit**

```bash
git add tests/list.rs
git commit -m "test(xbox): add smoke test in tests/list.rs"
```

---

## Phase 5 — FFI bindings

### Task 11: Add `xbox` to the FFI dispatch

**Files:**
- Modify: `game-scanner-ffi/src/lib.rs`

- [ ] **Step 1: Import the `xbox` module**

In `game-scanner-ffi/src/lib.rs`, change the import block to:

```rust
use game_scanner::{
    amazon, blizzard, epicgames, gog, origin,
    prelude::{Game, GameType},
    riotgames, steam, ubisoft, xbox,
};
```

- [ ] **Step 2: Add `xbox` to `parse_launcher`**

In `game-scanner-ffi/src/lib.rs`, change the body of `parse_launcher` to:

```rust
fn parse_launcher(name: &str) -> Option<GameType> {
    Some(match name {
        "amazongames" => GameType::AmazonGames,
        "blizzard" => GameType::Blizzard,
        "epicgames" => GameType::EpicGames,
        "gog" => GameType::GOG,
        "origin" => GameType::Origin,
        "riotgames" => GameType::RiotGames,
        "steam" => GameType::Steam,
        "ubisoft" => GameType::Ubisoft,
        "xbox" => GameType::XboxGames,
        _ => return None,
    })
}
```

- [ ] **Step 3: Add `XboxGames` arms to `gs_list`, `gs_find`, `gs_executable`**

In `game-scanner-ffi/src/lib.rs`, in each of the three `match l` blocks, add the arm:

```rust
GameType::XboxGames => xbox::games(),
```

to `gs_list`, `xbox::find(&id)` to `gs_find`, and `xbox::executable()` to `gs_executable`.

For `gs_list`, the `match` becomes:

```rust
let raw: Result<Vec<Game>, _> = match l {
    GameType::AmazonGames => amazon::games(),
    GameType::Blizzard => blizzard::games(),
    GameType::EpicGames => epicgames::games(),
    GameType::GOG => gog::games(),
    GameType::Origin => origin::games(),
    GameType::RiotGames => riotgames::games(),
    GameType::Steam => steam::games(),
    GameType::Ubisoft => ubisoft::games(),
    GameType::XboxGames => xbox::games(),
};
```

For `gs_find`:

```rust
let raw: Result<Game, _> = match l {
    GameType::AmazonGames => amazon::find(&id),
    GameType::Blizzard => blizzard::find(&id),
    GameType::EpicGames => epicgames::find(&id),
    GameType::GOG => gog::find(&id),
    GameType::Origin => origin::find(&id),
    GameType::RiotGames => riotgames::find(&id),
    GameType::Steam => steam::find(&id),
    GameType::Ubisoft => ubisoft::find(&id),
    GameType::XboxGames => xbox::find(&id),
};
```

For `gs_executable`:

```rust
let raw: Result<std::path::PathBuf, _> = match l {
    GameType::AmazonGames => amazon::executable(),
    GameType::Blizzard => blizzard::executable(),
    GameType::EpicGames => epicgames::executable(),
    GameType::GOG => gog::executable(),
    GameType::Origin => origin::executable(),
    GameType::RiotGames => riotgames::executable(),
    GameType::Steam => steam::executable(),
    GameType::Ubisoft => ubisoft::executable(),
    GameType::XboxGames => xbox::executable(),
};
```

- [ ] **Step 4: Update the round-trip test to include `xbox`**

In `game-scanner-ffi/src/lib.rs`, change the loop in `parse_launcher_round_trip` to:

```rust
#[test]
fn parse_launcher_round_trip() {
    for name in [
        "amazongames", "blizzard", "epicgames", "gog",
        "origin", "riotgames", "steam", "ubisoft", "xbox",
    ] {
        let l = parse_launcher(name).unwrap();
        assert_eq!(l.to_string(), name);
    }
}
```

- [ ] **Step 5: Build and test the FFI crate**

Run: `cargo build -p game-scanner-ffi && cargo test -p game-scanner-ffi`
Expected: compiles; 5 existing tests + 1 updated test pass.

- [ ] **Step 6: Commit**

```bash
git add game-scanner-ffi/src/lib.rs
git commit -m "feat(ffi): expose xbox launcher through C ABI"
```

---

### Task 12: Update FFI header doc comment

**Files:**
- Modify: `game-scanner-ffi/include/game_scanner.h`

- [ ] **Step 1: Add `xbox` to the launcher list comment**

In `game-scanner-ffi/include/game_scanner.h`, change the comment on `gs_list` to:

```c
/* List games for `launcher` ("steam", "epicgames", "gog", "blizzard",
 * "ubisoft", "amazongames", "origin", "riotgames", "xbox"). */
int gs_list(const char *launcher, char **out);
```

- [ ] **Step 2: Verify the file still compiles by rebuilding the demo**

Run: `cargo build -p game-scanner-ffi`
Expected: compiles. (The header is a doc-only change, so this is a sanity check.)

- [ ] **Step 3: Commit**

```bash
git add game-scanner-ffi/include/game_scanner.h
git commit -m "docs(ffi): mention xbox in gs_list doc comment"
```

---

## Phase 6 — Tauri GUI

### Task 13: Add `XboxGames` arms to `gui/src-tauri/src/commands.rs`

**Files:**
- Modify: `gui/src-tauri/src/commands.rs`

- [ ] **Step 1: Add `xbox` to the import list**

In `gui/src-tauri/src/commands.rs`, change the import to:

```rust
use game_scanner::{
    amazon, blizzard, epicgames, gog, origin, prelude::{Game, GameType}, riotgames, steam, ubisoft, xbox,
};
```

- [ ] **Step 2: Add `XboxGames` arm to `list_games`, `find_game`, `launcher_executable`**

In `gui/src-tauri/src/commands.rs`, add `GameType::XboxGames => xbox::games()` to the `list_games` match, `GameType::XboxGames => xbox::find(id)` to the `find_game` match, and `GameType::XboxGames => xbox::executable()` to the `launcher_executable` match. The full updated `list_games` looks like:

```rust
pub fn list_games(launcher: GameType) -> Result<Vec<Game>, String> {
    let raw: Result<Vec<Game>, _> = match launcher {
        GameType::AmazonGames => amazon::games(),
        GameType::Blizzard => blizzard::games(),
        GameType::EpicGames => epicgames::games(),
        GameType::GOG => gog::games(),
        GameType::Origin => origin::games(),
        GameType::RiotGames => riotgames::games(),
        GameType::Steam => steam::games(),
        GameType::Ubisoft => ubisoft::games(),
        GameType::XboxGames => xbox::games(),
    };
    raw.map_err(|e| e.to_string())
}
```

`find_game` and `launcher_executable` follow the same pattern (one new arm each).

- [ ] **Step 3: Build the GUI Rust backend**

Run: `cargo build -p game-scanner-gui`
Expected: compiles. (Adjust the `-p` argument if the GUI crate has a different name — `cargo metadata --format-version=1 | grep name` will tell you.)

- [ ] **Step 4: Commit**

```bash
git add gui/src-tauri/src/commands.rs
git commit -m "feat(gui): wire xbox through Tauri commands"
```

---

### Task 14: Update the capability matrix and tests

**Files:**
- Modify: `gui/src-tauri/src/capability.rs`

- [ ] **Step 1: Add Xbox entries to the `supports` match**

In `gui/src-tauri/src/capability.rs`, in `supports`, add the arms that make Xbox support `List | Find | Executable | Launch | Uninstall | Processes | Close` but not `Install`. The full updated function body is:

```rust
pub fn supports(launcher: GameType, op: Op) -> bool {
    use GameType::*;
    use Op::*;
    matches!(
        (launcher, op),
        // Every launcher supports list / find / executable / launch.
        (_, List) | (_, Find) | (_, Executable) | (_, Launch)
        // Install: Origin, Steam, Ubisoft.
        | (Origin, Install) | (Steam, Install) | (Ubisoft, Install)
        // Uninstall: Riot, Steam, Ubisoft, Xbox.
        | (RiotGames, Uninstall) | (Steam, Uninstall) | (Ubisoft, Uninstall) | (XboxGames, Uninstall)
        // Get Processes & Close: Steam, Xbox.
        | (Steam, Processes) | (Steam, Close)
        | (XboxGames, Processes) | (XboxGames, Close)
    )
}
```

- [ ] **Step 2: Add `XboxGames` to the `matrix` function launcher list**

In `gui/src-tauri/src/capability.rs`, change the `for launcher in [...]` in `matrix` to:

```rust
for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Steam, Ubisoft, XboxGames] {
```

- [ ] **Step 3: Update the existing tests to include `XboxGames`**

In `gui/src-tauri/src/capability.rs`, in the `#[cfg(test)] mod tests` block, change every `for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Steam, Ubisoft]` to `for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Steam, Ubisoft, XboxGames]`. There are 7 such loops. Also update the `matrix_contains_all_eight_launchers` test:

```rust
#[test]
fn matrix_contains_all_nine_launchers() {
    let m = matrix();
    for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Steam, Ubisoft, XboxGames] {
        let key = launcher.to_string();
        assert!(m.contains_key(&key), "matrix missing {:?}", launcher);
    }
    assert_eq!(m.len(), 9);
}
```

- [ ] **Step 4: Add new Xbox-specific assertions**

In `gui/src-tauri/src/capability.rs`, in `mod tests`, add:

```rust
#[test]
fn xbox_supports_uninstall_processes_close() {
    assert!(supports(XboxGames, Op::Uninstall));
    assert!(supports(XboxGames, Op::Processes));
    assert!(supports(XboxGames, Op::Close));
}

#[test]
fn xbox_does_not_support_install() {
    assert!(!supports(XboxGames, Op::Install));
}

#[test]
fn matrix_xbox_has_seven_ops() {
    let m = matrix();
    let ops = m.get("xbox").unwrap();
    assert_eq!(ops.len(), 7);
    for op in [Op::List, Op::Find, Op::Executable, Op::Launch, Op::Uninstall, Op::Processes, Op::Close] {
        assert!(ops.contains(&op), "xbox matrix missing {:?}", op);
    }
    assert!(!ops.contains(&Op::Install));
}
```

- [ ] **Step 5: Run the capability tests**

Run: `cargo test -p <gui crate name> --lib capability`
Expected: all green, including the 3 new xbox tests.

- [ ] **Step 6: Commit**

```bash
git add gui/src-tauri/src/capability.rs
git commit -m "feat(gui): add XboxGames to capability matrix with tests"
```

---

### Task 15: Add `xbox` to the Svelte types

**Files:**
- Modify: `gui/src/lib/types.ts`

- [ ] **Step 1: Add `xbox` to the `GameType` union**

In `gui/src/lib/types.ts`, change the `GameType` type to:

```ts
export type GameType =
  | "amazongames"
  | "blizzard"
  | "epicgames"
  | "gog"
  | "origin"
  | "riotgames"
  | "steam"
  | "ubisoft"
  | "xbox";
```

- [ ] **Step 2: Add `xbox` to `ALL_GAME_TYPES`**

In `gui/src/lib/types.ts`, change the `ALL_GAME_TYPES` array to:

```ts
export const ALL_GAME_TYPES: GameType[] = [
  "amazongames",
  "blizzard",
  "epicgames",
  "gog",
  "origin",
  "riotgames",
  "steam",
  "ubisoft",
  "xbox",
];
```

- [ ] **Step 3: Add the label**

In `gui/src/lib/types.ts`, add `xbox: "Xbox / Microsoft Store"` to `LAUNCHER_LABELS`:

```ts
export const LAUNCHER_LABELS: Record<GameType, string> = {
  amazongames: "Amazon Games",
  blizzard: "Blizzard",
  epicgames: "Epic Games",
  gog: "GOG",
  origin: "Origin",
  riotgames: "Riot Games",
  steam: "Steam",
  ubisoft: "Ubisoft",
  xbox: "Xbox / Microsoft Store",
};
```

- [ ] **Step 4: Type-check the frontend**

Run: `cd gui && pnpm exec svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors. (If `pnpm` is unavailable, skip — the change is small and the type checker will validate it on next dev start.)

- [ ] **Step 5: Commit**

```bash
git add gui/src/lib/types.ts
git commit -m "feat(gui): expose Xbox launcher in Svelte types"
```

---

## Phase 7 — Documentation

### Task 16: Add Xbox rows to the README capability tables

**Files:**
- Modify: `README.md` (5 capability tables)

- [ ] **Step 1: Add a `| Xbox     | ...` row to each of the 5 tables**

The rows to add (in order of appearance in `README.md`):

| Section | Row to add |
| --- | --- |
| OS | `| Xbox     | ❓                                        | ✅       | ❌     | ❌     |` |
| Game Commands support | `| Xbox     | ❌       | ✅      | ✅         |` |
| Game State support | `| Xbox     | ❌         | ❌            | ❌           | ❌           | ❌              |` |
| Operations | `| Xbox     | ✅          | ✅                       | ✅                      |` |
| Management | `| Xbox     | ✅      | ❓             | ❓     |` |

(Use the existing row formatting — `|`-separated columns, single space after the pipe.)

- [ ] **Step 2: Render the README to verify formatting**

Run: `cat README.md | head -100`
Expected: every table now has 9 rows, properly aligned.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs(readme): add Xbox rows to capability tables"
```

---

### Task 17: Create `docs/xbox.md`

**Files:**
- Create: `docs/xbox.md`

- [ ] **Step 1: Create the file with the launcher detail document**

Create `docs/xbox.md` with:

```markdown
# Xbox / Microsoft Store

## Windows

### Launcher Info

- LAUNCHER_EXECUTABLE:
  `Microsoft.GamingApp_*\XboxApp.exe` — the path is discovered by enumerating
  installed Appx packages with the WinRT `PackageManager::FindPackagesForUser`
  API and locating the one whose `FamilyName` starts with `Microsoft.GamingApp`.

### Games

Games are enumerated the same way: every Appx package the current user has
installed is inspected. A package is included in the result if its
`FamilyName` matches one of the allow-listed prefixes in
`src/xbox/platform/windows.rs::XBOX_FAMILY_PREFIXES`.

The `Name`, `Publisher`, `InstallLocation`, `Version`, and `FullName` come
from the `Package` object directly. The `<Application Id="...">` and
`<DisplayName>` come from parsing
`<install_location>\AppxManifest.xml` via `src/xbox/manifest.rs`.

### Start game

```commandline
explorer.exe shell:AppsFolder\<FamilyName>!<ApplicationId>
```

### Uninstall game

Uninstall is a WinRT call to `PackageManager::RemovePackageAsync(<FullName>)`
— the standard Windows Apps & Features flow runs underneath, with the
usual confirmation prompt for per-user packages.

### Close launcher

There is no "close launcher" op exposed. The Xbox App can be terminated by
the OS, not by an in-band command.

### Limitations

- Pure-UWP processes (no Win32 sibling) are not detected by `get_processes`
  because the host process (`svchost.exe` or a runtime broker) does not
  expose the package's install path. Win32-packaged games such as Forza
  Horizon 5, Minecraft Bedrock, and the Age of Empires series are detected
  normally.
- The allow-list is curated and may miss long-tail indie Game Pass titles.
  New publishers can be added by appending their `FamilyName` prefix to
  `XBOX_FAMILY_PREFIXES` in `src/xbox/platform/windows.rs`.
- Linux and macOS are not supported. The module compiles on every host but
  returns `LauncherNotFound` on non-Windows.
```

- [ ] **Step 2: Commit**

```bash
git add docs/xbox.md
git commit -m "docs(xbox): launcher detail page"
```

---

## Phase 8 — End-to-end verification

### Task 18: Full build + test sweep

**Files:**
- (no source change)

- [ ] **Step 1: Build the full workspace**

Run: `cargo build --workspace`
Expected: compiles on the host target.

- [ ] **Step 2: Run all tests**

Run: `cargo test --workspace`
Expected: all green, including the new `xbox` unit and integration tests.

- [ ] **Step 3: Build the Windows target**

Run: `cargo build --workspace --target x86_64-pc-windows-msvc`
Expected: compiles. (Skip on non-Windows hosts.)

- [ ] **Step 4: Type-check the GUI frontend**

Run: `cd gui && pnpm install --frozen-lockfile && pnpm exec svelte-check`
Expected: 0 errors.

- [ ] **Step 5: Manual verification on a real Windows machine**

On a Windows host with the Xbox App / Microsoft Store installed and at
least one Game Pass title:

1. `cargo run -p game-scanner-ffi --example ffi_demo` (or your preferred
   smoke harness) — confirm `gs_list("xbox")` returns ≥ 1 entry with
   `_type == "xbox"`.
2. `gs_find("xbox", "<known PackageFullName>")` returns the same entry.
3. `gs_executable("xbox")` returns a path ending in `XboxApp.exe` that
   exists on disk.
4. `gs_launch("<Game JSON>")` opens the game within ~5 s.
5. `gs_get_processes("<Game JSON>")` returns ≥ 1 PID for a Win32-packaged game.
6. `gs_close("<Game JSON>")` terminates the launched processes.
7. `gs_uninstall("<Game JSON>")` opens the standard Windows Apps & Features
   uninstall prompt; completing it removes the package.
8. `pnpm tauri dev` shows "Xbox / Microsoft Store" in the launcher list
   with a badge of `7`.

Capture the results in the PR description.

- [ ] **Step 6: Final commit (if any doc tweak was needed)**

```bash
git status  # if anything changed
git add -A
git commit -m "docs: minor tweaks from end-to-end verification"
```

---

## Self-review checklist (run after writing the plan)

- ✅ Every spec section has a task:
  - Module structure → Tasks 3–8
  - Data flow (games/find/executable) → Task 7
  - WinRT integration → Task 5
  - Filtering allow-list → Task 5
  - Manager dispatch (uninstall_game, get_processes) → Task 9
  - Capability matrix (supports / matrix) → Task 14
  - FFI surface → Tasks 11–12
  - GUI surface (types, commands) → Tasks 13, 15
  - Error handling → Tasks 5, 7 (WinRT → `ErrorKind` mapping inline)
  - Unit tests → Tasks 3, 5
  - Integration test → Task 10
  - Capability tests → Task 14
  - FFI tests → Task 11
  - Manual verification checklist → Task 18
  - Dependencies → Task 2
  - Docs (`docs/xbox.md` + README rows) → Tasks 16, 17
- ✅ No TBD / TODO / "fill in" / "similar to" placeholders.
- ✅ Type names consistent across tasks: `XboxGames`, `"xbox"`, `XboxPackage`, `ParsedManifest`, `xbox::games` / `xbox::find` / `xbox::executable` / `xbox::uninstall` / `xbox::processes`.
- ✅ Sentinel string `__xbox__:remove_package` appears identically in
  Task 7 (`build_game`) and Task 7 (`uninstall` check).
- ✅ `install_game` is intentionally untouched in Task 9 — verified
  against the spec's "fall through to InvalidGame" decision.
- ✅ Capability matrix table at the top of this plan matches Task 14
  one-to-one (7 ✅ for Xbox, ❌ for Install).
