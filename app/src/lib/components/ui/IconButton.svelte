<script lang="ts">
  import type { Snippet } from "svelte";
  import { liquid } from "#lib/motion.ts";
  import Icon, { type IconName } from "#lib/components/ui/Icon.svelte";

  /** Round icon button with the outward halo on hover (no flat background). `children`
   * replaces the icon (e.g. a `ProgressRing`). */
  let {
    icon,
    label,
    size = 36,
    iconSize = 18,
    active = false,
    href,
    el = $bindable(),
    class: klass = "",
    children,
    ...rest
  }: {
    icon?: IconName;
    label: string;
    size?: number;
    iconSize?: number;
    active?: boolean;
    href?: string;
    el?: HTMLElement;
    class?: string;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();
</script>

<svelte:element
  this={href ? "a" : "button"}
  bind:this={el}
  class="halo-btn {klass}"
  class:active
  style:width="{size}px"
  style:height="{size}px"
  {href}
  type={href ? undefined : "button"}
  aria-label={label}
  title={label}
  use:liquid
  {...rest}
>
  {#if children}{@render children()}{:else if icon}<Icon name={icon} size={iconSize} />{/if}
</svelte:element>
