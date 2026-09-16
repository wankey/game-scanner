// Linux is not supported. The dispatcher in mod.rs already returns
// LauncherNotFound from get_launcher_executable. This file exists so
// the cfg_attr machinery is symmetric with Windows / macOS.
