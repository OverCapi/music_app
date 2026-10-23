<script lang="ts">
  import SwapImage from "#lib/components/ui/SwapImage.svelte";

  /** Round cover with a center hole that spins while `spinning`. */
  let {
    src,
    spinning = false,
    size = 52,
  }: { src?: string; spinning?: boolean; size?: number } = $props();
</script>

<span class="disc" class:spinning style:width="{size}px" style:height="{size}px">
  <SwapImage {src} />
  <span class="hole"></span>
</span>

<style>
  .disc {
    position: relative;
    flex: none;
    overflow: hidden;
    border-radius: 50%;
    background: var(--line);
    box-shadow: 0 8px 20px -6px var(--shadow);
    animation: spin 9s linear infinite paused;
  }
  .spinning {
    animation-play-state: running;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .hole {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 12px;
    height: 12px;
    margin: -6px 0 0 -6px;
    border-radius: 50%;
    background: var(--bg);
    box-shadow: inset 0 0 0 1px var(--border);
  }
</style>
