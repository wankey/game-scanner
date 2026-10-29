# WeGame (Windows)

Supports installed Rail games registered in Windows uninstall entries. Use
`wegame::games()`, `wegame::find("2001481")`, and `wegame::executable()`.
Node exposes the same three methods as `scanner.wegame`; errors throw.
C bindings accept `"wegame"` in the existing list/find/executable functions.

## Discovery

Read user/machine uninstall entries in both registry views. Try InstallLocation,
the direct uninstall executable's parent, then DisplayIcon's parent. A candidate
must have valid `rail_files/rail_game_identify.json` (positive numeric game_id),
`rail_files/rail_game_build_info.dat` (JSON with nonempty repo_build_id), and
nonempty game content beyond metadata/cache and the known uninstaller.

A local `rail_files/download_cache.dat` excludes the candidate. No download
tasks or library roots are scanned. Games without uninstall registrations,
non-Rail titles and network installations are outside this implementation's
coverage. A single damaged registration does not stop other games from loading;
diagnostics use the crate's existing logger.

The client is located independently through its wegame protocol registration
or uninstall entry. Game enumeration does not require the client to be running
or still installed.

## State and limitations

Returned games have installed=true. This indicates committed installation
metadata and some payload, not full file integrity, authorization or readiness
to play. Updates and leftover download markers can cause conservative omissions.
Metadata/payload left behind after uninstall can still cause false positives.

Download and update detection are unsupported: downloading/needs_update=false
are compatibility defaults, not verified absence of activity or updates. Byte
counts, match_identity and all game commands are absent. Only List, Find and
Executable are exposed in the demo capability matrix.

IDs are decimal strings from the identity JSON, not directory names. Duplicate
IDs select one installation by stable canonical path order. `find` returning
not found does not prove that an unregistered game is absent.

## Process-name detection

Static process-name detection is deferred until an engine-independent source
can be verified. The experimental Unity layout heuristic, process_name field
and GUI column have been removed. Installed-game detection remains available.

Run `cargo run --example wegame` for a read-only local scan and find round trip.
The implementation was researched using WeGame 6.7.0.10150 and Honkai Impact 3
(ID 2001481); coverage is not a guarantee for every WeGame game or client version.

Adding GameType::WeGame requires downstream exhaustive matches to add an arm.

## Validation (2026-10-29)

The workspace's 64 tests, GUI type check/build, and standalone Node check/build
passed on Windows. Rust, Node and C FFI read-only scans found Honkai Impact 3
(2001481), round-tripped ID lookup and located the client. DeltaForce (2001918),
still carrying a download marker, was excluded. Node/FFI invalid-ID errors were
also checked. Linux/macOS cross-compilation was not run.

