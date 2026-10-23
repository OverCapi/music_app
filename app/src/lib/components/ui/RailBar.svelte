<script lang="ts">
  import { dragValue, pointerFraction } from "#lib/actions.ts";

  /** Horizontal rail with a fill: `seek` (accent glow, optional knob, click to jump) or
   * `volume` (plain fill, press and drag). `value` and the fraction given to `onvalue` are 0–1.
   * `dock`: compact volume that widens on hover (the player dock). */
  let {
    value,
    label,
    onvalue,
    variant = "seek",
    thickness = 4,
    knob = false,
    dock = false,
  }: {
    value: number;
    label: string;
    onvalue: (fraction: number) => void;
    variant?: "seek" | "volume";
    /** Rail height in px (seek only). */
    thickness?: number;
    knob?: boolean;
    dock?: boolean;
  } = $props();

  let dragging = $state(false);
  const width = $derived(`${Math.max(0, Math.min(1, value || 0)) * 100}%`);
</script>

{#snippet rail()}
  <span class="rail"
    ><span class="fill" class:dragging style:width
      >{#if knob}<span class="knob"></span>{/if}</span
    ></span
  >
{/snippet}

{#if variant === "volume"}
  <button
    class="bar volume"
    class:dock
    type="button"
    aria-label={label}
    use:dragValue={{ onvalue, ondragging: (on) => (dragging = on) }}
  >
    {@render rail()}
  </button>
{:else}
  <button
    class="bar seek"
    type="button"
    style:--t="{thickness}px"
    aria-label={label}
    onclick={(e) => onvalue(pointerFraction(e, e.currentTarget))}
  >
    {@render rail()}
  </button>
{/if}

<style>
  .bar {
    padding: 0;
    background: none;
  }
  .rail {
    position: relative;
    display: block;
    border-radius: 6px;
    background: var(--line);
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 6px;
  }
  .seek {
    flex: 1;
    height: calc(var(--t) + 10px);
  }
  .seek .rail {
    height: var(--t);
  }
  .seek .fill {
    background: var(--accent);
    box-shadow: 0 0 14px var(--accent);
    transition: width 0.3s linear;
  }
  .knob {
    position: absolute;
    right: -6px;
    top: 50%;
    width: 12px;
    height: 12px;
    margin-top: -6px;
    border-radius: 50%;
    background: var(--fg);
    box-shadow: 0 2px 8px var(--shadow);
  }
  .volume {
    flex: 1;
    height: 18px;
    padding: 6px 0;
    box-sizing: border-box;
    touch-action: none;
  }
  .volume .rail {
    height: 100%;
  }
  .volume .fill {
    background: var(--fg);
    transition: width 0.35s var(--ease-out);
  }
  .volume .fill.dragging {
    transition: width 0.07s linear;
  }
  .dock {
    flex: none;
    width: 84px;
    height: 16px;
    transition:
      width 0.35s var(--ease-out),
      padding 0.3s var(--ease-out);
  }
  .dock:hover {
    width: 108px;
    padding: 4px 0;
  }
</style>
