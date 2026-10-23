<script lang="ts">
  import type { Snippet } from "svelte";
  import { liquid } from "#lib/motion.ts";
  import Cover from "#lib/components/ui/Cover.svelte";
  import Equalizer from "#lib/components/ui/Equalizer.svelte";

  /** One track row: index (equalizer when current), 44 px cover, then your cells.
   * `cols` is the grid for the cells AFTER the cover; `colsSmall` replaces it under 640 px
   * (cells that disappear: `TrackCell hideSmall`). Cells: `TrackMain`, `TrackCell`. */
  let {
    index,
    cover,
    current = false,
    playing = false,
    cols,
    colsSmall = cols,
    onplay,
    title,
    children,
  }: {
    index?: number;
    cover?: string;
    current?: boolean;
    playing?: boolean;
    cols: string;
    colsSmall?: string;
    onplay?: () => void;
    title?: string;
    children: Snippet;
  } = $props();
</script>

<div
  class="row tr"
  class:current
  data-liquid
  role="row"
  tabindex="0"
  {title}
  style:--cols={cols}
  style:--cols-sm={colsSmall}
  use:liquid
  onclick={onplay}
  onkeydown={(e) => e.key === "Enter" && onplay?.()}
>
  <span class="n">
    {#if current}
      <Equalizer {playing} />
    {:else if index !== undefined}
      {index + 1}
    {/if}
  </span>
  <span class="art"><Cover src={cover} /></span>
  {@render children()}
</div>

<style>
  .tr {
    display: grid;
    grid-template-columns: 28px 44px var(--cols);
    gap: 16px;
    align-items: center;
    padding: 8px 14px;
    flex: none;
    /* Off-screen rows (a whole library) are neither laid out nor painted. */
    content-visibility: auto;
    contain-intrinsic-size: auto 60px;
  }
  .n {
    display: flex;
    justify-content: center;
    color: var(--mute);
    font-family: var(--mono);
    font-size: 13px;
  }
  .art {
    position: relative;
    width: 44px;
    height: 44px;
    overflow: hidden;
    border-radius: calc(var(--rs) / 2);
  }
  @container tracks (max-width: 640px) {
    .tr {
      grid-template-columns: 28px 44px var(--cols-sm);
      gap: 12px;
      padding: 8px 8px;
    }
  }
</style>
