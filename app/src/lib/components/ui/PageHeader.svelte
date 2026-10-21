<script lang="ts">
  import type { Snippet } from "svelte";
  import Cover from "#lib/components/ui/Cover.svelte";
  import PlayButton from "#lib/components/ui/PlayButton.svelte";

  /** Mix / playlist header: cover mosaic (180 px), kind label, title, meta, round play button. */
  let {
    kind,
    title,
    meta,
    covers,
    onplay,
    playLabel = "Lire",
    playDisabled = false,
    titleSlot,
    extra,
  }: {
    kind: string;
    title: string;
    meta: string;
    covers?: (string | undefined)[];
    onplay?: () => void;
    playLabel?: string;
    playDisabled?: boolean;
    /** Replaces the `h1` (inline rename form). */
    titleSlot?: Snippet;
    /** Extra controls next to the play button. */
    extra?: Snippet;
  } = $props();
</script>

<div class="head">
  <div class="art"><Cover src={covers} mosaic /></div>
  <div class="info">
    <span class="kind">{kind}</span>
    {#if titleSlot}{@render titleSlot()}{:else}<h1>{title}</h1>{/if}
    <span class="meta">{meta}</span>
  </div>
  <div class="actions">
    {@render extra?.()}
    {#if onplay}
      <PlayButton size={64} label={playLabel} disabled={playDisabled} onclick={onplay} />
    {/if}
  </div>
</div>

<style>
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 20px 28px;
    padding-top: 20px;
  }
  .art {
    position: relative;
    flex: none;
    width: clamp(110px, 16vw, 180px);
    aspect-ratio: 1;
    overflow: hidden;
    border-radius: var(--rs);
    box-shadow: 0 24px 50px -16px var(--shadow);
  }
  .info {
    display: flex;
    flex: 1 1 220px;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }
  .kind {
    color: var(--mute);
    font-size: 13px;
    font-weight: 600;
  }
  .info :global(h1) {
    overflow-wrap: break-word;
    font-size: clamp(30px, 5vw, 52px) !important;
  }
  .meta {
    color: var(--mute);
    font-size: 14px;
  }
  .actions {
    display: flex;
    flex: none;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    margin-left: auto;
  }
</style>
