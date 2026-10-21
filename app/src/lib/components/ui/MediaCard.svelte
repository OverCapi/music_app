<script lang="ts">
  import type { Snippet } from "svelte";
  import Card from "#lib/components/ui/Card.svelte";
  import Cover from "#lib/components/ui/Cover.svelte";
  import PlayButton from "#lib/components/ui/PlayButton.svelte";

  /** Square cover (single or mosaic) + title + subtitle. With `href` the card navigates and
   * `onplay` shows a round play button over the cover; without it the whole card plays. */
  let {
    title,
    subtitle,
    covers,
    mosaic = false,
    href,
    onplay,
    playLabel = "Lire",
    badge,
  }: {
    title: string;
    subtitle?: string;
    covers?: (string | undefined)[] | string;
    mosaic?: boolean;
    href?: string;
    onplay?: () => void;
    playLabel?: string;
    badge?: Snippet;
  } = $props();
</script>

<Card {href} onclick={href ? undefined : onplay}>
  <span class="art">
    <Cover src={covers} {mosaic} />
    {@render badge?.()}
    {#if onplay && href}
      <span class="play">
        <PlayButton
          size={40}
          label={playLabel}
          onclick={(e: MouseEvent) => {
            e.preventDefault();
            e.stopPropagation();
            onplay();
          }}
        />
      </span>
    {/if}
  </span>
  <span class="text">
    <span class="title">{title}</span>
    {#if subtitle}<span class="sub">{subtitle}</span>{/if}
  </span>
</Card>

<style>
  .art {
    position: relative;
    display: block;
    flex: none;
    aspect-ratio: 1;
    overflow: hidden;
    border-radius: var(--rs);
    box-shadow: 0 20px 40px -16px var(--shadow);
  }
  .play {
    position: absolute;
    right: 10px;
    bottom: 10px;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    padding: 0 4px 4px;
  }
  .title,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title {
    font-size: 14px;
    font-weight: 600;
  }
  .sub {
    color: var(--mute);
    font-size: 12px;
  }
</style>
