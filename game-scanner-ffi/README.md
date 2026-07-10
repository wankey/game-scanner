# game-scanner-ffi

C ABI for [`game-scanner`](../). Lets non-Rust callers (C, C++, C#, Go, ...)
use the library without depending on the Rust toolchain.

## Layout

```
game-scanner-ffi/
├── Cargo.toml          # builds staticlib + cdylib
├── include/
│   └── game_scanner.h  # C header (include this from C/C++)
├── src/lib.rs          # all #[no_mangle] extern "C" functions
└── examples/
    ├── ffi_demo.cpp    # minimal C++ consumer
    └── build_demo.bat  # MSVC helper to compile the demo
```

## Build

```bash
cargo build -p game-scanner-ffi --release
```

Produces (on Windows):

| Artifact                          | Use                                       |
| --------------------------------- | ----------------------------------------- |
| `target/release/game_scanner_ffi.dll`     | Runtime DLL to ship next to your exe |
| `target/release/game_scanner_ffi.dll.lib` | Import lib — link your C/C++ against this |
| `target/release/game_scanner_ffi.lib`     | Static archive (requires the same CRT — see below) |

On macOS: `libgame_scanner_ffi.dylib` / `.a`. On Linux:
`libgame_scanner_ffi.so` / `.a`.

## ABI at a glance

Every function that returns a string writes a JSON-encoded buffer into
`char **out` and returns an `int` status:

| Code | Meaning                                                       |
| ---- | ------------------------------------------------------------- |
| `0`  | Success — `*out` holds the JSON result.                       |
| `1`  | FFI bug (serialization / out-of-memory).                      |
| `2`  | Backend failure — `*out` holds `{"error": "..."}` instead.    |

Always release the buffer with `gs_free`.

```c
char *json = NULL;
int rc = gs_list("steam", &json);
if (rc == 0) {
    /* parse json ... */
}
gs_free(json);  /* safe with rc != 0 too */
```

The `launcher` argument is the snake-case name from
`game_scanner::prelude::GameType::to_string`:

```
amazongames  blizzard  epicgames  gog
origin       riotgames steam      ubisoft
```

Manager operations (`gs_install` / `gs_uninstall` / `gs_launch` /
`gs_close` / `gs_get_processes`) take the `Game` serialized as JSON
— the same shape `gs_find` returns. Round-trip is zero-copy:

```c
char *game_json = NULL;
gs_find("steam", "945360", &game_json);   /* rc == 0 */
gs_launch(game_json, NULL);               /* hand the same bytes back */
gs_free(game_json);
```

## Linking modes

**Dynamic (recommended).** Link `game_scanner_ffi.dll.lib`, ship
`game_scanner_ffi.dll` next to your exe. No CRT friction.

**Static.** Add `game_scanner_ffi.lib` to your link line. Because
`game-scanner` pulls in Rust's std and bundled SQLite, the C++ side
must use the **same C runtime** Rust was built against (MSVC CRT by
default on Windows). Mixing CRTs surfaces as duplicate-symbol errors
at link or runtime.

## Demo

```cmd
cargo build -p game-scanner-ffi --release
game-scanner-ffi\examples\build_demo.bat
game-scanner-ffi\examples\ffi_demo.exe
```

Expected output (your installed launchers may differ):

```
steam: rc=2 body={"error":"Invalid Steam path, maybe this launcher is not installed: ..."}
epicgames: rc=2 body={"error":"Invalid Epic Games path, maybe this launcher is not installed: {}"}
```

The `rc=2` and JSON error payload are the FFI working correctly on a
machine without these launchers installed. Use `nlohmann/json` (or any
other parser) in real code to consume `body`.

## Tests

```bash
cargo test -p game-scanner-ffi
```

Covers launcher-name parsing, JSON round-tripping, and the
`gs_free(NULL)` no-op contract.
