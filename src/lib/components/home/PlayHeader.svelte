<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "#lib/components/ui/Icon.svelte";

  let {
    title,
    playLabel,
    onplay,
    playDisabled = false,
    children,
  }: {
    title: string;
    playLabel?: string;
    onplay?: () => void;
    playDisabled?: boolean;
    /** Extra buttons on the right. */
    children?: Snippet;
  } = $props();
</script>

<div class="head">
  <h2>{title}</h2>
  {#if onplay}
    <button
      class="play"
      onclick={onplay}
      disabled={playDisabled}
      aria-label={playLabel ?? "Lire"}
      title={playLabel ?? "Lire"}
    >
      <Icon name="play" size={20} />
    </button>
  {/if}
  <div class="extra">{@render children?.()}</div>
</div>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    margin: 0 0 0.75rem;
  }

  h2 {
    margin: 0;
    font-size: 1.3rem;
  }

  .play {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border-radius: 50%;
    background: var(--accent);
    color: var(--on-accent);
  }

  .play:not(:disabled):hover {
    transform: scale(1.05);
  }

  .head :global(.pill) {
    padding: 0.4rem 1rem;
    font-size: 0.8rem;
  }

  .extra {
    display: flex;
    gap: 0.5rem;
    margin-left: auto;
  }
</style>
