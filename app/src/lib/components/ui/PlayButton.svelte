<script lang="ts">
  import { liquid } from "#lib/motion.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  /** Round accent play / pause button; the two icons cross-fade when `playing` changes. */
  let {
    size = 64,
    label = "Lire",
    playing = false,
    ...rest
  }: { size?: number; label?: string; playing?: boolean; [key: string]: unknown } = $props();

  const iconSize = $derived(Math.round(size * 0.4));
</script>

<button
  class="play-btn"
  type="button"
  style:width="{size}px"
  style:height="{size}px"
  aria-label={label}
  title={label}
  use:liquid
  {...rest}
>
  <span class="swap-ico" class:on={playing}><Icon name="pause" size={iconSize} /></span>
  <span class="swap-ico" class:on={!playing}><Icon name="play" size={iconSize} /></span>
</button>

<style>
  .swap-ico {
    grid-area: 1 / 1;
    display: grid;
    opacity: 0;
    transform: scale(0.5) rotate(-30deg);
    transition:
      opacity 0.25s,
      transform 0.35s var(--ease-spring);
  }
  .swap-ico.on {
    opacity: 1;
    transform: none;
  }
</style>
