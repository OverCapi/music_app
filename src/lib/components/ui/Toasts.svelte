<script lang="ts">
  import { toasts } from "#lib/stores/toasts.svelte.ts";
</script>

<div class="toasts" role="status" aria-live="polite">
  {#each toasts.items as toast (toast.id)}
    <button class="toast" onclick={() => toasts.dismiss(toast.id)} title="Fermer">
      {toast.message}
    </button>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 1rem;
    bottom: 6rem;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-width: 380px;
  }

  .toast {
    padding: 0.75rem 1rem;
    border-left: 3px solid var(--danger);
    border-radius: 6px;
    background: var(--surface-2);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.5);
    color: var(--text);
    font-size: 0.85rem;
    text-align: left;
    white-space: pre-line;
    user-select: text;
  }
</style>
