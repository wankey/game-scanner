import { writable } from "svelte/store";
import type { Capabilities, Game, GameType } from "./types";

export const currentLauncher = writable<GameType>("steam");
export const games = writable<Game[]>([]);
export const selectedGame = writable<Game | null>(null);
export const launcherExecutablePath = writable<string | null>(null);
export const processes = writable<number[]>([]);
export const capabilities = writable<Capabilities | null>(null);

let pollTimer: ReturnType<typeof setInterval> | null = null;

export function startProcessPolling(game: Game) {
  stopProcessPolling();
  const tick = async () => {
    const { getProcesses } = await import("./tauri");
    try {
      const p = await getProcesses(game);
      processes.set(p ?? []);
    } catch {
      // swallow polling errors silently
    }
  };
  void tick();
  pollTimer = setInterval(tick, 5000);
}

export function stopProcessPolling() {
  if (pollTimer !== null) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
  processes.set([]);
}
