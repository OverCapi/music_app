<script lang="ts">
  import type { Snippet } from "svelte";

  /** Responsive auto-fill grid (cards, channels…). `min` is the column minimum in px; under
   * 640 px of page width `minSmall` / `gapSmall` apply (`minSmall="100%"` = one column). */
  let {
    min = 170,
    gap = 18,
    minSmall = min,
    gapSmall = gap,
    dim = false,
    children,
  }: {
    min?: number | string;
    gap?: number;
    minSmall?: number | string;
    gapSmall?: number;
    /** Fades the grid (stale results while reloading). */
    dim?: boolean;
    children: Snippet;
  } = $props();

  const px = (v: number | string) => (typeof v === "number" ? `${v}px` : v);
</script>

<div
  class="grid"
  class:dim
  style:--min={px(min)}
  style:--gap="{gap}px"
  style:--min-sm={px(minSmall)}
  style:--gap-sm="{gapSmall}px"
>
  {@render children()}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--min), 1fr));
    gap: var(--gap);
    transition: opacity 0.3s;
  }
  .dim {
    opacity: 0.5;
  }
  @container page (max-width: 640px) {
    .grid {
      grid-template-columns: repeat(auto-fill, minmax(var(--min-sm), 1fr));
      gap: var(--gap-sm);
    }
  }
</style>
