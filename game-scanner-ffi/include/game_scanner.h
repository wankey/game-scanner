#ifndef GAME_SCANNER_FFI_H
#define GAME_SCANNER_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* game-scanner C ABI — built from game-scanner-ffi.
 *
 * Every function that produces a heap-allocated string hands ownership
 * of that buffer back to the caller. The caller MUST release it with
 * `gs_free`, even on failure (the error payload is also a heap string).
 *
 * Status codes (non-manager ops write JSON to *out):
 *   0 = success, payload is JSON
 *   1 = FFI bug (serialization / out-of-memory)
 *   2 = backend failure (e.g. launcher not installed). Payload is
 *       `{"error": "..."}`.
 */

/* Free a string returned by any `gs_*` function. Safe to call with NULL. */
void gs_free(char *s);

/* List games for `launcher` ("steam", "epicgames", "gog", "blizzard",
 * "ubisoft", "amazongames", "origin", "riotgames", "xbox"). */
int gs_list(const char *launcher, char **out);

/* Find one game by launcher-specific id (e.g. a Steam app id). */
int gs_find(const char *launcher, const char *id, char **out);

/* Return the launcher executable path as a JSON string. */
int gs_executable(const char *launcher, char **out);

/* Manager ops take a `Game` serialized as JSON (same shape as `gs_find`
 * returns on success). They write either `null` (success) or
 * `{"error":"..."}` to *out.
 *
 * Typical round-trip: call `gs_find`, keep the JSON, hand the same
 * bytes back to `gs_launch` / `gs_install` / etc.
 */
int gs_install(const char *game_json, char **out);
int gs_uninstall(const char *game_json, char **out);
int gs_launch(const char *game_json, char **out);
int gs_close(const char *game_json, char **out);

/* Return PIDs running the supplied game. JSON array, or JSON null when
 * the launcher has no introspection (most non-Steam launchers). */
int gs_get_processes(const char *game_json, char **out);

#ifdef __cplusplus
}
#endif

#endif /* GAME_SCANNER_FFI_H */
