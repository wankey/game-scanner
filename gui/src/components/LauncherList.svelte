<script lang="ts">
  import { capabilities, currentLauncher, games } from "../lib/stores";
  import { listGames } from "../lib/tauri";
  import { toast } from "../lib/toast";
  import { ALL_GAME_TYPES, LAUNCHER_LABELS, type GameType } from "../lib/types";

  async function select(launcher: GameType) {
    currentLauncher.set(launcher);
    games.set([]);
    try {
      const list = await listGames(launcher);
      games.set(list);
    } catch (err) {
      toast.error(`${LAUNCHER_LABELS[launcher]}: ${(err as Error).message}`);
    }
  }

  $: caps = $capabilities;
  function count(launcher: GameType): number {
    return caps ? (caps[launcher] ?? []).length : 0;
  }
</script>

<aside>
  <h2>Launchers</h2>
  <ul>
    {#each ALL_GAME_TYPES as launcher}
      <li>
        <button
          class:active={$currentLauncher === launcher}
          on:click={() => select(launcher)}
        >
          <span class="name">{LAUNCHER_LABELS[launcher]}</span>
          <span class="badge" title="Number of supported operations">{count(launcher)}</span>
        </button>
      </li>
    {/each}
  </ul>
</aside>

<style>
  aside {
    border-right: 1px solid var(--border);
    background: var(--bg-elev);
    padding: 1rem;
    overflow-y: auto;
  }
  h2 {
    font-size: 0.85rem;
    text-transform: uppercase;
    color: var(--text-muted);
    margin: 0 0 0.75rem;
    letter-spacing: 0.05em;
  }
  ul { list-style: none; margin: 0; padding: 0; }
  li { margin-bottom: 0.25rem; }
  button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
  }
  button:hover { background: var(--bg); }
  button.active {
    background: var(--accent);
    color: var(--accent-text);
  }
  .badge {
    font-size: 0.75rem;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    background: var(--bg);
    color: var(--text-muted);
  }
  button.active .badge { background: rgba(255, 255, 255, 0.2); color: var(--accent-text); }
</style>
