<script lang="ts">
  import type { Snippet } from "svelte";

  /** Empty-state message: centered paragraphs (`<p>`, `<p class="muted">`, links in accent).
   * `compact`: left-aligned muted line inside a card (e.g. an empty `TrackList`). */
  let { compact = false, children }: { compact?: boolean; children: Snippet } = $props();
</script>

<div class="empty" class:compact>{@render children()}</div>

<style>
  .empty {
    padding: 3rem 0;
    text-align: center;
  }
  .empty :global(p) {
    margin: 0.3rem 0;
  }
  .empty :global(a) {
    color: var(--accent);
    font-weight: 600;
  }
  .compact {
    padding: 16px 14px;
    color: var(--mute);
    font-size: 14px;
    text-align: left;
  }
</style>
