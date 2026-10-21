<script lang="ts">
  import type { Snippet } from "svelte";
  import { liquid } from "#lib/motion.ts";

  /** Glass card that lifts on hover. A link when `href` is given, otherwise a button. */
  let {
    href,
    radius = 10,
    pad = 10,
    class: klass = "",
    children,
    ...rest
  }: {
    href?: string;
    radius?: number;
    pad?: number;
    class?: string;
    children: Snippet;
    [key: string]: unknown;
  } = $props();
</script>

<svelte:element
  this={href ? "a" : "button"}
  class="glass-flat lift card {klass}"
  data-liquid
  style:border-radius="calc(var(--rs) + {radius}px)"
  style:padding="{pad}px"
  {href}
  type={href ? undefined : "button"}
  use:liquid
  {...rest}
>
  {@render children()}
</svelte:element>

<style>
  .card {
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 12px;
    box-sizing: border-box;
    width: 100%;
    color: var(--fg);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
</style>
