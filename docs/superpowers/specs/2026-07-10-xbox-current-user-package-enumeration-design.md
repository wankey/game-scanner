# Xbox Current-User Package Enumeration

## Goal

Allow the Xbox scanner to run without elevation while continuing to discover
games installed for the user running the application.

## Design

Replace the all-users `PackageManager::FindPackages()` call with
`PackageManager::FindPackagesForUser("")`. In the WinRT API, an empty user SID
selects the current user. This avoids the administrative privilege requirement
of `FindPackages()` and intentionally excludes packages registered only for
other Windows users.

The existing package filtering, installed-path lookup, manifest parsing, and
public scanner API remain unchanged.

## Error Handling

If the current-user enumeration call fails, preserve the existing `ErrorKind::IO`
mapping and include the new API name in the diagnostic message. Per-package
metadata failures continue to skip only the affected package.

## Testing

Extract the current-user SID argument into a small helper. Add a unit test that
locks the helper to an empty WinRT string, preventing a future regression to an
all-users enumeration. Run the Xbox unit tests and the workspace test suite.

## Scope

This does not add scanning of other users' packages, alter launch or uninstall
behavior, or change package-family filtering.
