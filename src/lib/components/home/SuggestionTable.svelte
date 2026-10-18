<script lang="ts">
  import type { MixTrack } from "#lib/bindings.ts";
  import { formatTime } from "#lib/format.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import Thumb from "#lib/components/home/Thumb.svelte";

  let {
    tracks,
    onplay,
  }: {
    tracks: MixTrack[];
    /** Plays the list starting at `index`. */
    onplay: (index: number) => void;
  } = $props();
</script>

<div class="table">
  <div class="row head">
    <span>#</span>
    <span>Titre</span>
    <span class="right">Durée</span>
  </div>
  {#each tracks as track, i (`${i}-${track.youtube_id}`)}
    <button class="row item" onclick={() => onplay(i)} title="Lire">
      <span class="index">
        <span class="num">{i + 1}</span>
        <span class="icon"><Icon name="play" size={16} /></span>
      </span>
      <span class="title-cell">
        <span class="img"><Thumb id={track.youtube_id} /></span>
        <span class="meta">
          <span class="title">{track.title}</span>
          <span class="artist">{track.artist ?? "Artiste inconnu"}</span>
        </span>
        {#if track.known}
          <span class="badge">Déjà connu</span>
        {/if}
      </span>
      <span class="duration">{track.duration ? formatTime(track.duration) : ""}</span>
    </button>
  {/each}
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 2.5rem minmax(0, 1fr) 4rem;
    align-items: center;
    gap: 1rem;
    box-sizing: border-box;
    width: 100%;
    padding: 0.4rem 0.75rem;
    color: var(--muted);
    font-size: 0.9rem;
    text-align: left;
  }

  .head {
    margin-bottom: 0.5rem;
    padding-block: 0.5rem;
    border-bottom: 1px solid var(--surface-2);
    font-size: 0.75rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .right {
    text-align: right;
  }

  .item {
    border-radius: 4px;
    background: transparent;
  }

  .item:hover,
  .item:focus-visible {
    background: var(--surface-2);
    outline: none;
  }

  .index {
    display: grid;
    place-items: center;
    font-variant-numeric: tabular-nums;
  }

  .index > * {
    grid-area: 1 / 1;
  }

  .icon {
    display: grid;
    color: var(--text);
    visibility: hidden;
  }

  .item:hover .num {
    visibility: hidden;
  }

  .item:hover .icon,
  .item:focus-visible .icon {
    visibility: visible;
  }

  .title-cell {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    min-width: 0;
  }

  .img {
    flex: none;
    width: 56px;
    height: 32px;
    overflow: hidden;
    border-radius: 3px;
  }

  .img :global(.thumb) {
    width: 100%;
    height: 100%;
  }

  .meta {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .title,
  .artist {
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

  .badge {
    flex: none;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    background: var(--surface-3);
    font-size: 0.7rem;
  }

  .duration {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
