# Xbox / Microsoft Store

## Windows

### Launcher Info

- LAUNCHER_EXECUTABLE:
  `Microsoft.GamingApp_*\XboxApp.exe` — the path is discovered by enumerating
  installed Appx packages with the WinRT `PackageManager::FindPackages`
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