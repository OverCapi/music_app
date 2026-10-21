<script lang="ts">
  /** Segmented control with a glass indicator. The indicator can be dragged: it stretches
   * while dragging and snaps (spring) to the nearest segment on release. Segments with an
   * `href` are links. `value` is the selected id, or `undefined` when none is selected. */
  import { liquid } from "#lib/motion.ts";
  import Icon, { type IconName } from "#lib/components/ui/Icon.svelte";

  type Item = { id: string; label: string; href?: string; icon?: IconName };

  let {
    items,
    value,
    onselect,
    tabWidth = 108,
    height = 38,
    glass = true,
    iconOnly = false,
  }: {
    items: Item[];
    value?: string;
    onselect: (id: string) => void;
    /** Width of one segment in px; shrinks to fit when the parent is narrower. */
    tabWidth?: number;
    height?: number;
    /** Draws its own glass container (otherwise it sits inside another pill). */
    glass?: boolean;
    /** Shows each item's `icon` instead of its label (narrow windows). */
    iconOnly?: boolean;
  } = $props();

  let rootWidth = $state(0);
  const index = $derived(items.findIndex((i) => i.id === value));
  const w = $derived(rootWidth > 0 ? rootWidth / items.length : tabWidth);

  let dragX = $state<number | null>(null);
  let settling = $state(false);
  let suppressClick = 0;

  function pointerdown(e: PointerEvent) {
    if (e.button !== 0) return;
    const sx = e.clientX;
    const base = Math.max(0, index) * w;
    let moved = false;
    let x = base;
    const move = (ev: PointerEvent) => {
      const dx = ev.clientX - sx;
      if (!moved && Math.abs(dx) < 5) return;
      moved = true;
      x = Math.max(0, Math.min((items.length - 1) * w, base + dx));
      dragX = x;
    };
    const up = () => {
      removeEventListener("pointermove", move);
      removeEventListener("pointerup", up);
      if (!moved) return;
      const j = Math.round(x / w);
      suppressClick = performance.now() + 80;
      settling = true;
      dragX = j * w;
      setTimeout(() => {
        dragX = null;
        settling = false;
      }, 650);
      if (j !== index) onselect(items[j].id);
    };
    addEventListener("pointermove", move);
    addEventListener("pointerup", up);
  }

  function pick(e: MouseEvent, item: Item) {
    if (performance.now() < suppressClick) return e.preventDefault();
    // Links navigate by themselves.
    if (!item.href) onselect(item.id);
  }
</script>

<div class="seg" class:glass role="group" onpointerdown={pointerdown}>
  <div
    class="track"
    bind:clientWidth={rootWidth}
    style:--tab-w="{tabWidth}px"
    style:--h="{height}px"
  >
    <span
      class="ind"
      class:hidden={index < 0 && dragX === null}
      class:dragging={dragX !== null && !settling}
      class:settling
      style:width="{w}px"
      style:transform="translateX({dragX ?? Math.max(0, index) * w}px)"
    ></span>
    {#each items as item (item.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <svelte:element
        this={item.href ? "a" : "button"}
        class="tab"
        aria-current={item.id === value ? "true" : undefined}
        aria-label={item.label}
        href={item.href}
        type={item.href ? undefined : "button"}
        use:liquid
        onclick={(e: MouseEvent) => pick(e, item)}
      >
        {#if iconOnly && item.icon}
          <Icon name={item.icon} size={20} />
        {:else}
          {item.label}
        {/if}
      </svelte:element>
    {/each}
  </div>
</div>

<style>
  .seg {
    position: relative;
    max-width: 100%;
    box-sizing: border-box;
    touch-action: none;
    border-radius: 999px;
  }
  .seg.glass {
    padding: 5px;
  }
  .track {
    position: relative;
    display: flex;
  }
  .tab {
    position: relative;
    overflow: hidden;
    display: grid;
    flex: 0 1 var(--tab-w);
    place-items: center;
    min-width: 0;
    height: var(--h);
    padding: 0 6px;
    border-radius: 999px;
    background: transparent;
    color: var(--fg);
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
  }
  .ind {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    border-radius: 999px;
    background: var(--glass2);
    box-shadow:
      inset 0 1px 0 var(--hi),
      0 6px 16px -6px var(--shadow);
    transition:
      transform 0.6s var(--ease-spring),
      width 0.3s,
      opacity 0.3s;
    pointer-events: none;
  }
  .ind.hidden {
    opacity: 0;
  }
  .ind.dragging {
    transition: transform 0.1s linear;
    scale: 1.08 0.92;
  }
  .ind.settling {
    transition: transform 0.55s cubic-bezier(0.34, 1.4, 0.64, 1);
  }
</style>
