<script lang="ts">
  import { currentLauncher, games, selectedGame } from "../lib/stores";
  import { listGames } from "../lib/tauri";
  import { toast } from "../lib/toast";
  import { LAUNCHER_LABELS } from "../lib/types";

  export let onFindClick: () => void = () => {};

  async function refresh() {
    try {
      const list = await listGames($currentLauncher);
      games.set(list);
      toast.ok(`Loaded ${list.length} game(s)`);
    } catch (err) {
      toast.error(`${LAUNCHER_LABELS[$currentLauncher]}: ${(err as Error).message}`);
    }
  }
</script>

<section>
  <header>
    <h2>{LAUNCHER_LABELS[$currentLauncher]} — Games</h2>
    <div class="actions">
      <button on:click={refresh}>Refresh</button>
      <button on:click={onFindClick}>Find by Id</button>
    </div>
  </header>

  {#if $games.length === 0}
    <div class="empty">
      No games found. Is {LAUNCHER_LABELS[$currentLauncher]} installed?
    </div>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Name</th>
          <th>Id</th>
          <th>Installed</th>
          <th>Needs update</th>
          <th>Downloading</th>
        </tr>
      </thead>
      <tbody>
        {#each $games as game (game.id)}
          <tr
            class:selected={$selectedGame?.id === game.id}
            on:click={() => selectedGame.set(game)}
          >
            <td>{game.name}</td>
            <td class="mono">{game.id}</td>
            <td>{game.state.installed ? "✅" : "—"}</td>
            <td>{game.state.needs_update ? "✅" : "—"}</td>
            <td>{game.state.downloading ? "✅" : "—"}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid var(--border);
  }
  h2 { margin: 0; font-size: 1rem; }
  .actions { display: flex; gap: 0.5rem; }
  .actions button {
    padding: 0.4rem 0.8rem;
    border: 1px solid var(--border);
    background: var(--bg-elev);
    color: var(--text);
    border-radius: 6px;
    cursor: pointer;
  }
  .actions button:hover { background: var(--bg); }
  .empty {
    padding: 2rem;
    color: var(--text-muted);
    text-align: center;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    overflow-y: auto;
  }
  thead th {
    position: sticky;
    top: 0;
    background: var(--bg-elev);
    text-align: left;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--border);
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  tbody td {
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--border);
  }
  tbody tr { cursor: pointer; }
  tbody tr:hover { background: var(--bg-elev); }
  tbody tr.selected { background: var(--accent); color: var(--accent-text); }
  tbody tr.selected .mono { color: inherit; }
  .mono { font-family: ui-monospace, SFMono-Regular, monospace; color: var(--text-muted); }
</style>
