<script lang="ts">
  /** `null`: indeterminate (spinning quarter ring). */
  let { progress, stop = false }: { progress: number | null; stop?: boolean } = $props();

  // Circumference of r=9.
  const CIRCUMFERENCE = 2 * Math.PI * 9;
</script>

<svg viewBox="0 0 24 24" width="22" height="22" class:spin={progress === null} aria-hidden="true">
  <circle class="track" cx="12" cy="12" r="9" />
  <circle
    class="bar"
    cx="12"
    cy="12"
    r="9"
    stroke-dasharray={CIRCUMFERENCE}
    stroke-dashoffset={CIRCUMFERENCE * (progress === null ? 0.75 : 1 - progress)}
  />
  {#if stop}
    <rect class="stop" x="9" y="9" width="6" height="6" rx="1" />
  {/if}
</svg>

<style>
  .track,
  .bar {
    fill: none;
    stroke-width: 2.5;
  }

  .track {
    stroke: var(--track);
  }

  .bar {
    stroke: var(--accent);
    stroke-linecap: round;
    transform: rotate(-90deg);
    transform-origin: center;
    transition: stroke-dashoffset 0.2s linear;
  }

  /* Shown when the parent button is hovered. */
  .stop {
    fill: var(--text);
    opacity: 0;
  }

  :global(button:hover) > svg .stop {
    opacity: 1;
  }

  .spin {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
