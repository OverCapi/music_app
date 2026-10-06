<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { library, type Track } from "$lib/library.svelte";
  import { player } from "$lib/player.svelte";
  import { formatTime } from "$lib/utils";

  let query = $state("");

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return library.tracks;
    return library.tracks.filter((t) =>
      [t.title, t.artist, t.album].some((field) => field?.toLowerCase().includes(q)),
    );
  });

  const error = $derived(library.error ?? player.error ?? player.state.error);

  async function addFiles() {
    const paths = await open({
      multiple: true,
      directory: false,
      filters: [{ name: "Audio", extensions: ["mp3", "opus"] }],
    });
    if (paths && paths.length > 0) await library.add(paths);
  }

  /** Plays the visible list (like a Spotify playlist), starting at `index`. */
  function playFrom(index: number) {
    player.playFrom(
      filtered.map((t) => t.path),
      index,
    );
  }

  async function removeTrack(track: Track) {
    if (player.state.path === track.path) await player.stop();
    await library.remove(track.path);
  }
</script>

<section class="library">
  <header>
    <div>
      <h1>Bibliothèque</h1>
      <p class="count">
        {library.tracks.length} titre{library.tracks.length > 1 ? "s" : ""}
      </p>
    </div>
    <div class="actions">
      {#if library.tracks.length > 0}
        <input class="search" type="search" placeholder="Rechercher" bind:value={query} />
      {/if}
      <button class="add" onclick={addFiles} disabled={library.adding}>
        {library.adding ? "Ajout…" : "Ajouter des titres"}
      </button>
    </div>
  </header>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if library.tracks.length === 0}
    <div class="empty">
      <p>Ta bibliothèque est vide.</p>
      <p class="muted">Ajoute des fichiers mp3 pour commencer.</p>
    </div>
  {:else if filtered.length === 0}
    <div class="empty">
      <p class="muted">Aucun résultat pour « {query} ».</p>
    </div>
  {:else}
    <div class="table" role="table">
      <div class="row head" role="row">
        <span role="columnheader">#</span>
        <span role="columnheader">Titre</span>
        <span role="columnheader">Album</span>
        <span role="columnheader" class="right">Durée</span>
        <span></span>
      </div>
      {#each filtered as track, i (track.path)}
        {@const current = player.state.path === track.path}
        {@const playing = current && player.state.playing}
        <div
          class="row track"
          class:current
          role="row"
          tabindex="0"
          ondblclick={() => playFrom(i)}
          onkeydown={(e) => e.key === "Enter" && playFrom(i)}
          title={track.path}
        >
          <span class="index">
            <span class="num">{playing ? "▶" : i + 1}</span>
            <button
              class="row-play"
              onclick={() => (current ? player.togglePlay() : playFrom(i))}
              aria-label={playing ? "Pause" : "Lire"}
            >
              {#if playing}
                <svg viewBox="0 0 24 24"><rect x="6" y="5" width="4" height="14" rx="1" /><rect x="14" y="5" width="4" height="14" rx="1" /></svg>
              {:else}
                <svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>
              {/if}
            </button>
          </span>
          <span class="title-cell">
            <span class="title">{track.title}</span>
            <span class="artist">{track.artist ?? "Artiste inconnu"}</span>
          </span>
          <span class="album">{track.album ?? ""}</span>
          <span class="duration right">{formatTime(track.duration)}</span>
          <span class="row-actions">
            <button
              class="row-action"
              onclick={() => player.enqueue([track.path])}
              aria-label="Ajouter à la file d'attente"
              title="Ajouter à la file d'attente"
            >
              <svg viewBox="0 0 24 24"><path d="M3 6h12v2H3zm0 5h12v2H3zm0 5h8v2H3zm14-2v-3h2v3h3v2h-3v3h-2v-3h-3v-2z" /></svg>
            </button>
            <button
              class="row-action"
              onclick={() => removeTrack(track)}
              aria-label="Retirer de la bibliothèque"
              title="Retirer de la bibliothèque"
            >
              ✕
            </button>
          </span>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .library {
    padding: 1.5rem 2rem;
    overflow-y: auto;
    min-height: 0;
  }

  header {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  h1 {
    margin: 0;
    font-size: 2rem;
  }

  .count {
    margin: 0.25rem 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .search {
    width: 200px;
    padding: 0.5rem 0.9rem;
    border: 1px solid transparent;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: 0.85rem;
    outline: none;
  }

  .search:focus {
    border-color: var(--muted);
  }

  .add {
    padding: 0.6rem 1.4rem;
    border-radius: 999px;
    background: var(--accent);
    color: #000;
    font-weight: 700;
  }

  .add:not(:disabled):hover {
    transform: scale(1.04);
  }

  .error {
    margin: 0 0 1rem;
    color: #f15e6c;
    white-space: pre-line;
  }

  .empty {
    padding: 4rem 0;
    text-align: center;
  }

  .empty p {
    margin: 0.3rem 0;
  }

  .muted {
    color: var(--muted);
  }

  .row {
    display: grid;
    grid-template-columns: 2.5rem minmax(0, 3fr) minmax(0, 2fr) 4rem 4rem;
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
  .track:focus-visible {
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
  .row-action {
    display: grid;
    place-items: center;
    padding: 0;
    background: transparent;
    color: var(--text);
    visibility: hidden;
  }

  .row-play svg,
  .row-action svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
  }

  .track:hover .num {
    visibility: hidden;
  }

  .track:hover .row-play,
  .track:hover .row-action {
    visibility: visible;
  }

  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
  }

  .row-action {
    width: 20px;
    height: 20px;
    color: var(--muted);
    font-size: 0.8rem;
  }

  .row-action:hover {
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
