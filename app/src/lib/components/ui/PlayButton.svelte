<script lang="ts">
  import { liquid } from "#lib/motion.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  /** Round accent play button (76 on the home hero, 64 on headers, 40 on cards). */
  let {
    size = 64,
    label = "Lire",
    playing = false,
    ...rest
  }: { size?: number; label?: string; playing?: boolean; [key: string]: unknown } = $props();
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
  <Icon name={playing ? "pause" : "play"} size={Math.round(size * 0.4)} />
</button>
