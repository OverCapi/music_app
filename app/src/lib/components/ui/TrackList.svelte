<script lang="ts">
  import type { Snippet } from "svelte";

  /** Glass card holding `TrackRow`s. It is a size container: rows drop their `.hide-sm`
   * cells (album, artist column…) when the card gets narrower than 640 px. */
  let { children, empty }: { children: Snippet; empty?: string } = $props();
</script>

<div class="glass list" role="table">
  {#if empty}<div class="empty">{empty}</div>{/if}
  {@render children()}
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    padding: 8px;
    border-radius: calc(var(--rs) + 8px);
    container: tracks / inline-size;
  }
  .empty {
    padding: 16px 14px;
    color: var(--mute);
    font-size: 14px;
  }
</style>
