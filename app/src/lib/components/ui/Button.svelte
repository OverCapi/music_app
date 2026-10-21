<script lang="ts">
  import type { Snippet } from "svelte";
  import { liquid } from "#lib/motion.ts";

  /** Text pill. `primary` = accent, `ghost` = outlined glass, `outline` = bare outline that
   * turns red on hover (Quitter). A link when `href` is given. */
  let {
    variant = "ghost",
    href,
    size = 40,
    type = "button",
    class: klass = "",
    children,
    ...rest
  }: {
    variant?: "primary" | "ghost" | "outline";
    href?: string;
    size?: number;
    type?: "button" | "submit";
    class?: string;
    children: Snippet;
    [key: string]: unknown;
  } = $props();
</script>

<svelte:element
  this={href ? "a" : "button"}
  class="pill {variant} {klass}"
  style:height="{size}px"
  {href}
  type={href ? undefined : type}
  use:liquid
  {...rest}
>
  {@render children()}
</svelte:element>

<style>
  .pill.outline {
    border: 1px solid var(--mute);
    background: transparent;
    color: var(--fg);
    font-weight: 700;
    transition:
      border-color 0.25s,
      color 0.25s;
  }
  .pill.outline:hover {
    border-color: var(--danger);
    color: var(--danger);
  }
</style>
