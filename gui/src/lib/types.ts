// Mirrors `game_scanner::prelude::GameType` and `Op` enum exactly.
// `Op` is renamed to snake_case on the Rust side via #[serde(rename_all = "snake_case")].

export type GameType =
  | "amazongames"
  | "blizzard"
  | "epicgames"
  | "gog"
  | "origin"
  | "riotgames"
  | "steam"
  | "ubisoft";

export const ALL_GAME_TYPES: GameType[] = [
  "amazongames",
  "blizzard",
  "epicgames",
  "gog",
  "origin",
  "riotgames",
  "steam",
  "ubisoft",
];

export const LAUNCHER_LABELS: Record<GameType, string> = {
  amazongames: "Amazon Games",
  blizzard: "Blizzard",
  epicgames: "Epic Games",
  gog: "GOG",
  origin: "Origin",
  riotgames: "Riot Games",
  steam: "Steam",
  ubisoft: "Ubisoft",
};

export type Op =
  | "list"
  | "find"
  | "executable"
  | "install"
  | "launch"
  | "uninstall"
  | "processes"
  | "close";

export const OP_LABELS: Record<Op, string> = {
  list: "List",
  find: "Find",
  executable: "Executable",
  install: "Install",
  launch: "Launch",
  uninstall: "Uninstall",
  processes: "Get Processes",
  close: "Close",
};

export interface Game {
  _type: string;
  id: string;
  name: string;
  path: string | null;
  commands: {
    install: string[] | null;
    launch: string[] | null;
    uninstall: string[] | null;
  };
  state: {
    installed: boolean;
    needs_update: boolean;
    downloading: boolean;
    total_bytes: number | null;
    received_bytes: number | null;
  };
}

export type Capabilities = Record<GameType, Op[]>;
