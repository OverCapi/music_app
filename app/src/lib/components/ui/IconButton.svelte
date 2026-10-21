<script lang="ts">
  import { liquid } from "#lib/motion.ts";
  import Icon, { type IconName } from "#lib/components/ui/Icon.svelte";

  /** Round icon button with the outward halo on hover (no flat background). */
  let {
    icon,
    label,
    size = 36,
    iconSize = 18,
    active = false,
    href,
    class: klass = "",
    ...rest
  }: {
    icon: IconName;
    label: string;
    size?: number;
    iconSize?: number;
    active?: boolean;
    href?: string;
    class?: string;
    [key: string]: unknown;
  } = $props();
</script>

<svelte:element
  this={href ? "a" : "button"}
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
  <Icon name={icon} size={iconSize} />
</svelte:element>
