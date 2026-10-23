<script lang="ts">
  import type { Snippet } from "svelte";
  import { liquid } from "#lib/motion.ts";
  import Icon, { type IconName } from "#lib/components/ui/Icon.svelte";

  /** Header of a collapsible side panel: round chevron toggle + title that fades out when
   * collapsed (66 px panel: the 36 px button sits exactly in the middle). `buttonFirst` puts
   * the toggle before the title (right panel). */
  let {
    title,
    expanded,
    open,
    icon,
    buttonFirst = false,
    ontoggle,
    trailing,
  }: {
    title: string;
    /** Panel is wide enough to show text. */
    expanded: boolean;
    /** Drives the toggle's label. */
    open: boolean;
    /** Chevron pointing toward the panel's edge. */
    icon: IconName;
    buttonFirst?: boolean;
    ontoggle: () => void;
    trailing?: Snippet;
  } = $props();
</script>

{#snippet toggle()}
  <button
    class="halo-btn chev"
    class:flipped={!expanded}
    type="button"
    use:liquid
    onclick={ontoggle}
    aria-label={open ? "Replier" : "Déplier"}
  >
    <Icon name={icon} size={16} />
  </button>
{/snippet}

<header class:collapsed={!expanded} class:buttonFirst>
  {#if buttonFirst}{@render toggle()}{/if}
  <span class="fade head" class:on={expanded}>
    <span class="title">{title}</span>
    {@render trailing?.()}
  </span>
  {#if !buttonFirst}{@render toggle()}{/if}
</header>

<style>
  header {
    display: flex;
    flex: none;
    align-items: center;
    padding: 14px 14px 8px;
    transition: padding 0.55s var(--ease-out);
  }
  header.collapsed {
    padding-inline: 15px;
  }
  .head {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
  }
  header.collapsed .head {
    flex: 0 0 0;
  }
  .title {
    flex: 1;
    padding-left: 4px;
    color: var(--mute);
    font-size: 13px;
    font-weight: 600;
  }
  .buttonFirst .title {
    padding-left: 8px;
  }
  .chev :global(svg) {
    transition: transform 0.5s;
  }
  .chev.flipped :global(svg) {
    transform: rotate(180deg);
  }
</style>
