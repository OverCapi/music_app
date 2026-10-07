<script lang="ts">
  import { resolve } from "$app/paths";
  import type { GroupView } from "#lib/bindings.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { formatTime } from "#lib/format.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import GroupTrackStatus from "#lib/components/group/GroupTrackStatus.svelte";

  let { view }: { view: GroupView } = $props();

  const queue = $derived(view.state.queue);
  const playing = $derived(view.state.playback.status === "playing");
</script>

{#if queue.length === 0}
  <div class="empty">
    <p>La file du groupe est vide.</p>
    <p class="muted">
      Ajoute des titres depuis
      <a href={resolve("/search")}>Rechercher</a>
      ou depuis les titres de ta
      <a href={resolve("/")}>bibliothèque</a>
      téléchargés depuis YouTube.
    </p>
  </div>
{:else}
  <div class="table" role="table" aria-label="File du groupe">
    <div class="row head" role="row">
      <span role="columnheader">#</span>
      <span role="columnheader">Titre</span>
      <span role="columnheader">Ajouté par</span>
      <span role="columnheader" class="right">Durée</span>
      <span role="columnheader" class="center" title="Téléchargement local">État</span>
      <span></span>
    </div>
    {#each queue as entry, i (entry.entry_id)}
      {@const current = view.state.current === i}
      <div
        class="row entry"
        class:current
        class:past={view.state.current !== null && i < view.state.current}
        role="row"
        tabindex="0"
        ondblclick={() => group.playEntry(entry.entry_id)}
        onkeydown={(e) => e.key === "Enter" && group.playEntry(entry.entry_id)}
      >
        <span class="index">{current && playing ? "▶" : i + 1}</span>
        <span class="title-cell">
          <span class="title">{entry.title}</span>
          <span class="artist">{entry.artist ?? "Artiste inconnu"}</span>
        </span>
        <span class="added-by">{entry.added_by === view.me ? "toi" : entry.added_by}</span>
        <span class="duration right">
          {entry.duration !== null ? formatTime(entry.duration) : ""}
        </span>
        <span class="center">
          <GroupTrackStatus download={view.downloads[entry.youtube_id]} />
        </span>
        <span class="row-actions">
          <button
            class="row-action"
            onclick={() => group.playEntry(entry.entry_id)}
            aria-label="Lire ce titre pour le groupe"
            title="Lire ce titre pour le groupe"
          >
            <Icon name="play" size={16} />
          </button>
          <button
            class="row-action"
            onclick={() => group.remove(entry.entry_id)}
            aria-label="Retirer de la file du groupe"
            title="Retirer de la file du groupe"
          >
            ✕
          </button>
        </span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .empty a {
    color: var(--text);
    text-decoration: underline;
  }

  .row {
    display: grid;
    grid-template-columns: 2.5rem minmax(0, 3fr) minmax(0, 1fr) 4rem 2.5rem 3.5rem;
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

  .entry {
    color: var(--muted);
    font-size: 0.9rem;
    cursor: default;
    outline: none;
  }

  .entry:hover,
  .entry:focus-visible,
  .entry:focus-within {
    background: var(--surface-2);
  }

  .entry.past {
    opacity: 0.6;
  }

  .right {
    text-align: right;
  }

  .center {
    display: flex;
    justify-content: center;
  }

  .index {
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  .title-cell {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .title,
  .artist,
  .added-by {
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
  .current .index {
    color: var(--accent);
  }

  .duration {
    font-variant-numeric: tabular-nums;
  }

  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
  }

  .row-action {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    background: transparent;
    color: var(--muted);
    font-size: 0.8rem;
    visibility: hidden;
  }

  .row-action:hover {
    color: var(--text);
  }

  .entry:hover .row-action,
  .entry:focus-within .row-action {
    visibility: visible;
  }
</style>
