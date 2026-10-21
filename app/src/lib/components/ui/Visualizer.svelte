<script lang="ts">
  /** Mirrored capsule bars (grow from the center). Used in the queue panel cover and
   * fullscreen. Without real levels it animates a believable pseudo-spectrum.
   * `intro`: bars rise left→right in a wave (520 ms each, 16 ms stagger) on mount. */
  import { onMount } from "svelte";

  let {
    playing,
    bars = 22,
    levels,
    color = "oklch(1 0 0 / .9)",
    gap = 4,
    intensity = 3,
    intro = false,
  }: {
    playing: boolean;
    bars?: number;
    /** 0..1 per bar, from a future `Spectrum` event. Optional. */
    levels?: number[];
    color?: string;
    gap?: number;
    intensity?: number;
    intro?: boolean;
  } = $props();

  let root = $state<HTMLElement>();

  onMount(() => {
    const els = Array.from(root!.children) as HTMLElement[];
    const v = els.map(() => (intro ? 0 : 0.07));
    const start = performance.now();
    const m = intensity / 3;
    let raf = 0;
    let stop: () => void = () => {};
    const tick = (t: number) => {
      let settled = true;
      els.forEach((b, i) => {
        let target =
          levels?.[i] ??
          (playing
            ? Math.min(
                1,
                0.12 +
                  0.6 *
                    m *
                    (Math.abs(Math.sin(t / (160 + (i % 9) * 37) + i * 1.7)) * 0.65 +
                      Math.abs(Math.sin(t / (520 + (i % 5) * 90) + i)) * 0.45),
              )
            : 0.07);
        if (intro) {
          const e = Math.max(0, Math.min(1, (t - start - 180 - i * 16) / 520));
          target *= 1 - Math.pow(1 - e, 3);
        }
        v[i] += (target - v[i]) * 0.2;
        b.style.transform = `scaleY(${v[i].toFixed(3)})`;
        if (Math.abs(target - v[i]) > 0.004) settled = false;
      });
      // Paused and settled: nothing to animate, stop the loop (restarted by `playing`).
      raf = settled && !playing && !levels ? 0 : requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    stop = () => cancelAnimationFrame(raf);
    wake = () => {
      if (!raf) raf = requestAnimationFrame(tick);
    };
    return () => stop();
  });

  let wake = () => {};
  $effect(() => {
    if (playing) wake();
  });
</script>

<div class="viz" bind:this={root} style:gap="{gap}px" aria-hidden="true">
  {#each Array.from({ length: bars }, (_, i) => i) as i (i)}
    <span style:background={color}></span>
  {/each}
</div>

<style>
  .viz {
    display: flex;
    align-items: center;
    height: 100%;
  }
  span {
    flex: 1;
    height: 100%;
    border-radius: 999px;
    transform-origin: center;
    transform: scaleY(0.07);
  }
</style>
