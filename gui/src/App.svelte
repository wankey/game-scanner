<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import LauncherList from "./components/LauncherList.svelte";
  import GamesTable from "./components/GamesTable.svelte";
  import ActionPanel from "./components/ActionPanel.svelte";
  import FindDialog from "./components/FindDialog.svelte";
  import Toast from "./components/Toast.svelte";
  import { capabilities, currentLauncher, games, stopProcessPolling } from "./lib/stores";
  import { getCapabilities, listGames } from "./lib/tauri";
  import { toast } from "./lib/toast";

  let findOpen = false;

  onMount(async () => {
    try {
      const caps = await getCapabilities();
      capabilities.set(caps);
    } catch (err) {
      toast.error(`Failed to load capabilities: ${(err as Error).message}`);
      return;
    }
    try {
      const list = await listGames($currentLauncher);
      games.set(list);
    } catch (err) {
      toast.error(`Failed to load games: ${(err as Error).message}`);
    }
  });

  // Stop process polling when the app unmounts. We use `onDestroy`
  // instead of returning a cleanup from the async `onMount` because
  // Svelte's TypeScript signature expects async mounts to return
  // `Promise<undefined>`.
  onDestroy(() => {
    stopProcessPolling();
  });
</script>

<main class="layout">
  <LauncherList />
  <GamesTable onFindClick={() => (findOpen = true)} />
  <ActionPanel />
</main>

<FindDialog bind:open={findOpen} on:close={() => (findOpen = false)} />
<Toast />

<footer class="version">game-scanner demo</footer>

<style>
  .layout {
    display: grid;
    grid-template-columns: 240px 1fr 340px;
    height: 100%;
    width: 100%;
  }
  .version {
    position: fixed;
    bottom: 0.25rem;
    left: 0.5rem;
    font-size: 0.7rem;
    color: var(--text-muted);
    pointer-events: none;
  }
</style>
