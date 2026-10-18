<script lang="ts">
  import Icon from "#lib/components/ui/Icon.svelte";
  import { toasts } from "#lib/stores/toasts.svelte.ts";
</script>

<div class="toasts" role="status" aria-live="polite">
  {#each toasts.items as toast (toast.id)}
    <button class="toast" onclick={() => toasts.dismiss(toast.id)} title="Fermer">
      <span class="icon"><Icon name="errorCircle" size={22} /></span>
      {toast.message}
    </button>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 1rem;
    bottom: calc(var(--dock-height) + 32px);
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-width: 380px;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    border-radius: 12px;
    background: var(--surface-2);
    box-shadow: var(--shadow-float);
    color: var(--text);
    font-size: 0.85rem;
    text-align: left;
    white-space: pre-line;
    user-select: text;
  }

  .icon {
    display: flex;
    color: var(--danger);
  }
</style>
