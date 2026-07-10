<script lang="ts">
  import { toasts } from "../lib/toast";
</script>

<div class="container" aria-live="polite">
  {#each $toasts as t (t.id)}
    <div class="toast" class:ok={t.kind === "ok"} class:error={t.kind === "error"}>
      {t.message}
    </div>
  {/each}
</div>

<style>
  .container {
    position: fixed;
    bottom: 1rem;
    right: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    z-index: 200;
    pointer-events: none;
  }
  .toast {
    padding: 0.75rem 1rem;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    max-width: 360px;
  }
  .toast.ok { border-left: 4px solid var(--ok); }
  .toast.error { border-left: 4px solid var(--danger); }
</style>
