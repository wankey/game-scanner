<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { currentLauncher, games, selectedGame } from "../lib/stores";
  import { findGame } from "../lib/tauri";
  import { toast } from "../lib/toast";
  import { LAUNCHER_LABELS } from "../lib/types";

  export let open = false;

  const dispatch = createEventDispatcher<{ close: void }>();

  let id = "";
  let busy = false;

  async function submit() {
    if (!id.trim() || busy) return;
    busy = true;
    try {
      const game = await findGame($currentLauncher, id.trim());
      // Make sure the table has the game in scope.
      const current = $games;
      if (!current.find((g) => g.id === game.id)) {
        games.set([...current, game]);
      }
      selectedGame.set(game);
      dispatch("close");
    } catch (err) {
      toast.error(`${LAUNCHER_LABELS[$currentLauncher]}: ${(err as Error).message}`);
    } finally {
      busy = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") dispatch("close");
  }
</script>

<svelte:window on:keydown={onKey} />

{#if open}
  <div class="backdrop" on:click={() => dispatch("close")}>
    <div class="dialog" on:click|stopPropagation>
      <h3>Find game by Id</h3>
      <p class="muted">Searching in {LAUNCHER_LABELS[$currentLauncher]}</p>
      <input
        type="text"
        bind:value={id}
        placeholder="e.g. 945360"
        autofocus
        on:keydown={(e) => e.key === "Enter" && submit()}
      />
      <div class="actions">
        <button on:click={() => dispatch("close")} disabled={busy}>Cancel</button>
        <button class="primary" on:click={submit} disabled={!id.trim() || busy}>
          {busy ? "Searching..." : "Find"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .dialog {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 1.5rem;
    width: min(420px, 90vw);
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  h3 { margin: 0; }
  .muted { color: var(--text-muted); margin: 0; }
  input {
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-elev);
    color: var(--text);
    font: inherit;
  }
  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
  .actions button {
    padding: 0.4rem 0.9rem;
    border: 1px solid var(--border);
    background: var(--bg-elev);
    color: var(--text);
    border-radius: 6px;
    cursor: pointer;
  }
  .actions button.primary {
    background: var(--accent);
    color: var(--accent-text);
    border-color: var(--accent);
  }
  .actions button:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
