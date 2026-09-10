# Xbox / Microsoft Store

## Windows

### Launcher Info

- LAUNCHER_EXECUTABLE:
  `Microsoft.GamingApp_*\XboxApp.exe` — the path is discovered by enumerating
  installed Appx packages with the WinRT `PackageManager::FindPackages`
  API and locating the one whose `FamilyName` starts with `Microsoft.GamingApp`.

### Games

The scan follows GameFinder's Xbox handler. It enumerates every mounted drive,
uses `Program Files\ModifiableWindowsApps` when present, and parses each
drive's `.GamingRoot` file for additional Xbox installation folders. It then
inspects each direct child directory for `AppxManifest.xml`, falling back to
`Content\AppxManifest.xml`.

`<Identity Name="...">` is the game ID and `<DisplayName>` is the displayed
name. The game path is the directory containing the manifest.

### Close launcher

There is no "close launcher" op exposed. The Xbox App can be terminated by
the OS, not by an in-band command.

### Limitations

- Pure-UWP processes (no Win32 sibling) are not detected by `get_processes`
  because the host process (`svchost.exe` or a runtime broker) does not
  expose the package's install path. Win32-packaged games such as Forza
  Horizon 5, Minecraft Bedrock, and the Age of Empires series are detected
  normally.
- This discovery model exposes no package full name or package family name,
  so it cannot generate the `AppsFolder` launch command or call
  `RemovePackageAsync` to uninstall a game.
- Linux and macOS are not supported. The module compiles on every host but
  returns `LauncherNotFound` on non-Windows.
