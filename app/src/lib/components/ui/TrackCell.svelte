<script lang="ts">
  import type { Snippet } from "svelte";

  /** A cell of a `TrackRow`: `sub` (muted text), `dur` (muted mono, right-aligned) or `plain`.
   * `hideSmall` hides it when the `TrackList` is narrower than 640 px; `showSmall` shows it
   * only then. */
  let {
    kind = "sub",
    hideSmall = false,
    showSmall = false,
    children,
  }: {
    kind?: "sub" | "dur" | "plain";
    hideSmall?: boolean;
    showSmall?: boolean;
    children: Snippet;
  } = $props();
</script>

<span class="cell {kind}" class:hideSmall class:showSmall>{@render children()}</span>

<style>
  .sub,
  .dur {
    color: var(--mute);
  }
  .sub {
    overflow: hidden;
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dur {
    font-family: var(--mono);
    font-size: 12px;
    text-align: right;
  }
  .showSmall {
    display: none;
  }
  @container tracks (max-width: 640px) {
    .hideSmall {
      display: none;
    }
    .showSmall {
      display: block;
    }
  }
</style>
