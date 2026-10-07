<script lang="ts">
  import type { Snippet } from "svelte";
  import type { Track } from "#lib/bindings.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import { formatTime } from "#lib/format.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  let {
    tracks,
    onplay,
    actions,
    actionsWidth = "5.5rem",
  }: {
    tracks: Track[];
    /** Plays the list starting at `index`. */
    onplay: (index: number) => void;
    /** Per-row buttons, shown on hover; give them the `row-action` class. */
    actions?: Snippet<[Track, number]>;
    /** Width of the actions column (fits three buttons by default). */
    actionsWidth?: string;
  } = $props();
</script>

<div class="table" role="table" style:--actions-width={actionsWidth}>
  <div class="row head" role="row">
    <span role="columnheader">#</span>
    <span role="columnheader">Titre</span>
    <span role="columnheader">Album</span>
    <span role="columnheader" class="right">Durée</span>
    <span></span>
  </div>
  {#each tracks as track, i (track.id)}
    {@const current = player.state.track_id === track.id}
    {@const playing = current && player.state.playing}
    <div
      class="row track"
      class:current
      role="row"
      tabindex="0"
      ondblclick={() => onplay(i)}
      onkeydown={(e) => e.key === "Enter" && onplay(i)}
      title={track.path}
    >
      <span class="index">
        <span class="num">{playing ? "▶" : i + 1}</span>
        <button
          class="row-play"
          onclick={() => (current ? player.togglePlay() : onplay(i))}
          aria-label={playing ? "Pause" : "Lire"}
        >
          <Icon name={playing ? "pause" : "play"} size={16} />
        </button>
      </span>
      <span class="title-cell">
        <span class="title">{track.title}</span>
        <span class="artist">{track.artist ?? "Artiste inconnu"}</span>
      </span>
      <span class="album">{track.album ?? ""}</span>
      <span class="duration right">{formatTime(track.duration)}</span>
      <span class="row-actions">
        {@render actions?.(track, i)}
      </span>
    </div>
  {/each}
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 2.5rem minmax(0, 3fr) minmax(0, 2fr) 4rem var(--actions-width);
    align-items: center;
    gap: 1rem;
    padding: 0.4rem 0.75rem;
    border-radius: 4px;
  }

  .head {
    margin-bottom: 0.5rem;
    border-bottom: 1px solid var(--surface-2);
    border-radius: 0;
    color: var(--muted);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .track {
    color: var(--muted);
    font-size: 0.9rem;
    cursor: default;
    outline: none;
  }

  .track:hover,
  .track:focus-visible,
  .track:focus-within {
    background: var(--surface-2);
  }

  .right {
    text-align: right;
  }

  .index {
    display: grid;
    place-items: center;
    font-variant-numeric: tabular-nums;
  }

  .index > * {
    grid-area: 1 / 1;
  }

  .row-play,
  .track :global(.row-action) {
    display: grid;
    place-items: center;
    padding: 0;
    background: transparent;
    color: var(--text);
    visibility: hidden;
  }

  .track:hover .num {
    visibility: hidden;
  }

  .track:hover .row-play,
  .track:hover :global(.row-action),
  .track:focus-within :global(.row-action),
  .track :global(.row-action.open) {
    visibility: visible;
  }

  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
  }

  .track :global(.row-action) {
    width: 20px;
    height: 20px;
    color: var(--muted);
    font-size: 0.8rem;
  }

  .track :global(.row-action:hover) {
    color: var(--text);
  }

  .title-cell {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .title,
  .artist,
  .album {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    color: var(--text);
  }

  .artist {
    font-size: 0.8rem;
  }

  .current .title,
  .current .num {
    color: var(--accent);
  }

  .duration {
    font-variant-numeric: tabular-nums;
  }
</style>
