<script lang="ts">
  import Icon from "#lib/components/ui/Icon.svelte";
  import { toasts } from "#lib/stores/toasts.svelte.ts";
</script>

<div class="toasts" role="status" aria-live="polite">
  {#each toasts.items as toast (toast.id)}
    <button class="toast glass" onclick={() => toasts.dismiss(toast.id)} title="Fermer">
      <span class="icon"><Icon name="errorCircle" size={20} /></span>
      {toast.message}
    </button>
  {/each}
</div>

<style>
  /* Glass pill, 108 px from the bottom, centered; it rises and scales in. */
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 108px;
    left: 16px;
    z-index: 50;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    pointer-events: none;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: min(520px, 100%);
    padding: 12px 20px;
    border-radius: 999px;
    color: var(--fg);
    font-size: 13px;
    font-weight: 600;
    text-align: left;
    white-space: pre-line;
    user-select: text;
    pointer-events: auto;
    animation: rise 0.45s var(--ease-enter);
  }
  .icon {
    display: flex;
    flex: none;
    color: var(--danger);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(14px) scale(0.96);
    }
  }
</style>
