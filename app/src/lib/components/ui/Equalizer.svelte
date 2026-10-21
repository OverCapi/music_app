<script lang="ts">
  /** Tiny animated equalizer (3 bars in lists, 4 in the dock). Static when not playing. */
  let {
    bars = 3,
    height = 14,
    playing = true,
  }: { bars?: number; height?: number; playing?: boolean } = $props();
</script>

<span class="eq" class:paused={!playing} style:height="{height}px" aria-hidden="true">
  {#each Array.from({ length: bars }, (_, i) => i) as i (i)}
    <span style:animation-delay="{i * -0.27}s" style:animation-duration="{0.9 + (i % 3) * 0.22}s"
    ></span>
  {/each}
</span>

<style>
  .eq {
    display: inline-flex;
    align-items: flex-end;
    gap: 2px;
  }
  span span {
    width: 3px;
    height: 100%;
    border-radius: 2px;
    background: var(--accent);
    transform-origin: bottom;
    animation: eq 1s ease-in-out infinite alternate;
  }
  .paused span {
    animation-play-state: paused;
    transform: scaleY(0.3);
  }
  @keyframes eq {
    from {
      transform: scaleY(0.25);
    }
    to {
      transform: scaleY(1);
    }
  }
</style>
