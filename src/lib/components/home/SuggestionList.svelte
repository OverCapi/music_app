<script lang="ts">
  import type { MixTrack } from "#lib/bindings.ts";
  import { formatTime } from "#lib/format.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  let {
    tracks,
    onplay,
    showKnown = true,
  }: {
    tracks: MixTrack[];
    /** Plays the list starting at `index`. */
    onplay: (index: number) => void;
    /** Marks tracks the user already knows. */
    showKnown?: boolean;
  } = $props();
</script>

<ul>
  {#each tracks as track, i (`${i}-${track.youtube_id}`)}
    <li>
      <button class="row" onclick={() => onplay(i)} title="Lire">
        <span class="index">
          <span class="num">{i + 1}</span>
          <span class="icon"><Icon name="play" size={16} /></span>
        </span>
        <span class="meta">
          <span class="title">{track.title}</span>
          <span class="artist">{track.artist ?? "Artiste inconnu"}</span>
        </span>
        {#if showKnown && track.known}
          <span class="badge">Déjà connu</span>
        {/if}
        <span class="duration">{track.duration ? formatTime(track.duration) : ""}</span>
      </button>
    </li>
  {/each}
</ul>

<style>
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: grid;
    grid-template-columns: 2.5rem minmax(0, 1fr) auto 4rem;
    align-items: center;
    gap: 1rem;
    width: 100%;
    padding: 0.4rem 0.75rem;
    border-radius: 4px;
    background: transparent;
    color: var(--muted);
    font-size: 0.9rem;
    text-align: left;
  }

  .row:hover,
  .row:focus-visible {
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

  .row:hover .num {
    visibility: hidden;
  }

  .row:hover .icon,
  .row:focus-visible .icon {
    visibility: visible;
  }

  .meta {
    display: flex;
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
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    background: var(--surface-2);
    font-size: 0.7rem;
  }

  .duration {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
