<script lang="ts">
  import {
    capabilities,
    currentLauncher,
    launcherExecutablePath,
    processes,
    selectedGame,
    startProcessPolling,
    stopProcessPolling,
  } from "../lib/stores";
  import { closeGame, getProcesses, installGame, launchGame, launcherExecutable, uninstallGame } from "../lib/tauri";
  import { toast } from "../lib/toast";
  import { LAUNCHER_LABELS, OP_LABELS, type Op } from "../lib/types";

  const ALL_OPS: Op[] = ["install", "launch", "uninstall", "processes", "close"];

  $: caps = $capabilities;
  $: supportedOps = caps ? new Set(caps[$currentLauncher] ?? []) : new Set<Op>();

  $: game = $selectedGame;
  $: launcherLabel = LAUNCHER_LABELS[$currentLauncher];

  async function runOp(op: Op) {
    if (!game) return;
    try {
      switch (op) {
        case "install":
          await installGame(game);
          toast.ok(`Installing ${game.name}`);
          break;
        case "uninstall":
          await uninstallGame(game);
          toast.ok(`Uninstalling ${game.name}`);
          break;
        case "launch":
          await launchGame(game);
          toast.ok(`Launched ${game.name}`);
          startProcessPolling(game);
          break;
        case "close":
          await closeGame(game);
          toast.ok(`Closed ${game.name}`);
          stopProcessPolling();
          break;
        case "processes": {
          const p = await getProcesses(game);
          processes.set(p ?? []);
          toast.info(`${game.name}: ${(p ?? []).length} process(es)`);
          break;
        }
      }
    } catch (err) {
      toast.error(`${OP_LABELS[op]} failed: ${(err as Error).message}`);
      if (op === "launch") stopProcessPolling();
    }
  }

  async function refreshExecutable() {
    try {
      const p = await launcherExecutable($currentLauncher);
      launcherExecutablePath.set(p);
    } catch {
      launcherExecutablePath.set(null);
    }
  }

  $: $currentLauncher, refreshExecutable();
</script>

<aside class="panel">
  <h2>Actions</h2>

  {#if !game}
    <p class="muted">Select a game from the table.</p>
  {:else}
    <div class="game">
      <div class="name">{game.name}</div>
      <div class="id mono">{game.id}</div>
      {#if game.path}
        <div class="path mono" title={game.path}>{game.path}</div>
      {/if}
    </div>

    <div class="buttons">
      {#each ALL_OPS as op}
        {@const supported = supportedOps.has(op)}
        <button
          disabled={!supported}
          title={supported ? OP_LABELS[op] : `${launcherLabel} does not support ${OP_LABELS[op]}`}
          on:click={() => runOp(op)}
        >
          {OP_LABELS[op]}
        </button>
      {/each}
    </div>

    {#if $processes.length > 0}
      <div class="processes">
        <strong>Processes:</strong>
        <span class="mono">{$processes.join(", ")}</span>
      </div>
    {/if}
  {/if}

  <div class="executable">
    <strong>Launcher executable:</strong>
    <span class="mono" title={$launcherExecutablePath ?? "Not available"}>
      {$launcherExecutablePath ?? "—"}
    </span>
  </div>
</aside>

<style>
  .panel {
    border-left: 1px solid var(--border);
    background: var(--bg-elev);
    padding: 1rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  h2 {
    font-size: 0.85rem;
    text-transform: uppercase;
    color: var(--text-muted);
    margin: 0;
    letter-spacing: 0.05em;
  }
  .muted { color: var(--text-muted); }
  .game {
    padding: 0.75rem;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  .name { font-weight: 600; }
  .id, .path { color: var(--text-muted); font-size: 0.85rem; margin-top: 0.25rem; word-break: break-all; }
  .mono { font-family: ui-monospace, SFMono-Regular, monospace; }
  .buttons {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }
  .buttons button {
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    border-radius: 6px;
    cursor: pointer;
  }
  .buttons button:hover:not(:disabled) { background: var(--accent); color: var(--accent-text); border-color: var(--accent); }
  .buttons button:disabled { opacity: 0.4; cursor: not-allowed; }
  .processes, .executable {
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  .processes { color: var(--ok); }
  .executable { margin-top: auto; }
</style>
