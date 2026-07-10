import { invoke } from "@tauri-apps/api/core";
import type { Capabilities, Game, GameType } from "./types";

/** Generic invoke wrapper that surfaces error messages via thrown Error. */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (err) {
    // Tauri returns the Rust error string in `err`.
    const message = err instanceof Error ? err.message : String(err);
    throw new Error(message);
  }
}

export const listGames = (launcher: GameType) =>
  call<Game[]>("list_games", { launcher });

export const findGame = (launcher: GameType, id: string) =>
  call<Game>("find_game", { launcher, id });

export const launcherExecutable = (launcher: GameType) =>
  call<string>("launcher_executable", { launcher });

export const getCapabilities = () => call<Capabilities>("get_capabilities");

export const installGame = (game: Game) => call<void>("install_game", { game });
export const uninstallGame = (game: Game) => call<void>("uninstall_game", { game });
export const launchGame = (game: Game) => call<void>("launch_game", { game });
export const closeGame = (game: Game) => call<void>("close_game", { game });
export const getProcesses = (game: Game) =>
  call<number[] | null>("get_processes", { game });
