<script lang="ts">
  import type { Snippet } from "svelte";

  /** Glass surface. `radius` is added to `--rs` (8 px): rows 8, cards 10, panels 10, dock 12. */
  let {
    radius = 8,
    pad,
    class: klass = "",
    children,
    ...rest
  }: {
    radius?: number;
    pad?: number;
    class?: string;
    children: Snippet;
    [key: string]: unknown;
  } = $props();
</script>

<div
  class="glass {klass}"
  style:border-radius="calc(var(--rs) + {radius}px)"
  style:padding={pad === undefined ? undefined : `${pad}px`}
  {...rest}
>
  {@render children()}
</div>
