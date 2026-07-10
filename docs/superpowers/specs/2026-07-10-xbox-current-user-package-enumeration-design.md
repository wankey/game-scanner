# Xbox Current-User Package Enumeration

## Goal

Allow the Xbox scanner to run without elevation while continuing to discover
games installed for the user running the application. When the process is
actually elevated through UAC, include packages registered for every user.

## Design

Check whether the process token is elevated. An elevated process calls
`PackageManager::FindPackages()` to enumerate packages for every user. A
non-elevated process calls `PackageManager::FindPackagesForUser("")`; in the
WinRT API, an empty user SID selects the current user and needs no elevation.

The check is for the process's elevation token, not whether its account belongs
to the Administrators group. An administrator account running without UAC
elevation follows the current-user path and must not call `FindPackages()`.

The existing package filtering, installed-path lookup, manifest parsing, and
public scanner API remain unchanged.

## Error Handling

If the selected enumeration call fails, preserve the existing `ErrorKind::IO`
mapping and include the API name in the diagnostic message. If the token check
itself fails, return a distinct `ErrorKind::IO` diagnostic rather than guessing
that the process is elevated. Per-package metadata failures continue to skip
only the affected package.

## Testing

Extract the enumeration choice into a small helper. Add unit tests that map an
elevated token to the all-users API and a non-elevated token to the empty-SID,
current-user API. Run the Xbox unit tests and the workspace test suite.

## Scope

This does not alter launch or uninstall behavior, or change package-family
filtering. It enumerates other users' packages only from an elevated process.
