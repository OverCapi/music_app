<script lang="ts">
  import type { Snippet } from "svelte";

  /** Main cell of a `TrackRow`: title over an optional subtitle. `badges` sit after the title
   * (favorite star, "not downloaded"…). `subtitleSmallOnly`: subtitle shown only when the
   * `TrackList` is narrow (the artist then has its own column on wide lists). */
  let {
    title,
    subtitle,
    subtitleSmallOnly = false,
    badges,
    children,
  }: {
    title: string;
    subtitle?: string;
    subtitleSmallOnly?: boolean;
    badges?: Snippet;
    /** Replaces the subtitle line (e.g. search results with extra info). */
    children?: Snippet;
  } = $props();
</script>

<span class="main">
  <span class="line">
    <span class="title">{title}</span>
    {@render badges?.()}
  </span>
  {#if children}
    {@render children()}
  {:else if subtitle !== undefined}
    <span class="sub" class:small-only={subtitleSmallOnly}>{subtitle}</span>
  {/if}
</span>

<style>
  .main {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .title,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title {
    font-size: 14px;
    font-weight: 600;
  }
  .sub {
    color: var(--mute);
    font-size: 12px;
  }
  .small-only {
    display: none;
  }
  @container tracks (max-width: 640px) {
    .small-only {
      display: block;
    }
  }
</style>
