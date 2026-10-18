<script lang="ts">
  import type { Snippet } from "svelte";
  import type { MixCategory, MixTrack } from "#lib/bindings.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import Thumb from "#lib/components/home/Thumb.svelte";

  let {
    title,
    subtitle,
    tracks = [],
    href,
    playLabel,
    onplay,
    category,
    cover,
  }: {
    title: string;
    subtitle: string;
    /** Tracks whose thumbnails form the 2x2 mosaic cover. */
    tracks?: MixTrack[];
    /** Makes the card a link; otherwise clicking the card plays. */
    href?: string;
    playLabel: string;
    onplay: () => void;
    category?: MixCategory;
    /** Replaces the mosaic. */
    cover?: Snippet;
  } = $props();

  const mosaic = $derived(tracks.slice(0, 4));
</script>

<div class="media">
  {#if href}
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- `href` comes from resolve() -->
    <a class="main" {href}>{@render body()}</a>
  {:else}
    <button class="main" onclick={onplay} title={playLabel}>{@render body()}</button>
  {/if}
  {#if href}
    <button class="play" onclick={onplay} aria-label={playLabel} title={playLabel}>
      <Icon name="play" size={22} />
    </button>
  {/if}
</div>

{#snippet body()}
  <span class="cover" data-category={category}>
    {#if cover}
      {@render cover()}
    {:else}
      <span class="mosaic" class:single={mosaic.length === 1}>
        {#each mosaic.length === 1 ? mosaic : [0, 1, 2, 3].map((n) => mosaic[n]) as track, i (i)}
          {#if track}
            <Thumb id={track.youtube_id} />
          {:else}
            <span class="blank"></span>
          {/if}
        {/each}
      </span>
    {/if}
  </span>
  <span class="title">{title}</span>
  <span class="subtitle">{subtitle}</span>
{/snippet}

<style>
  .media {
    position: relative;
    border-radius: 8px;
    background: var(--surface);
    transition: background 0.15s;
  }

  .media:hover,
  .media:focus-within {
    background: var(--surface-2);
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    box-sizing: border-box;
    width: 100%;
    padding: 0.8rem;
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    text-align: left;
  }

  .cover {
    display: block;
    aspect-ratio: 1;
    margin-bottom: 0.6rem;
    overflow: hidden;
    border-radius: 6px;
    background: var(--surface-3);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.4);
  }

  .mosaic {
    display: grid;
    grid-template: 1fr 1fr / 1fr 1fr;
    width: 100%;
    height: 100%;
  }

  .mosaic.single {
    grid-template: 1fr / 1fr;
  }

  .mosaic > :global(*) {
    min-height: 0;
    min-width: 0;
  }

  .blank {
    background: var(--surface-2);
  }

  .title,
  .subtitle {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-weight: 700;
    line-height: 1.3rem;
  }

  .subtitle {
    color: var(--muted);
    font-size: 0.8rem;
    line-height: 1.1rem;
  }

  .play {
    position: absolute;
    right: 1.2rem;
    /* Bottom-right corner of the cover: card padding + text block (fixed line heights) + gap. */
    bottom: calc(0.8rem + 0.6rem + 0.2rem + 2.4rem + 0.5rem);
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    padding: 0;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 8px 16px rgb(0 0 0 / 0.5);
    color: #000;
    opacity: 0;
    transform: translateY(8px);
    transition:
      opacity 0.15s,
      transform 0.15s;
  }

  .media:hover .play,
  .media:focus-within .play {
    opacity: 1;
    transform: none;
  }

  .play:hover {
    transform: scale(1.06) !important;
  }
</style>
