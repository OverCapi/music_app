<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { library, type Track } from "$lib/library.svelte";
  import { player } from "$lib/player.svelte";
  import { playlists } from "$lib/playlists.svelte";
  import TrackTable from "$lib/components/TrackTable.svelte";
  import AddToPlaylist from "$lib/components/AddToPlaylist.svelte";
  import AddToGroup from "$lib/components/AddToGroup.svelte";
  import { group } from "$lib/group.svelte";

  let query = $state("");

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return library.tracks;
    return library.tracks.filter((t) =>
      [t.title, t.artist, t.album].some((field) => field?.toLowerCase().includes(q)),
    );
  });

  const error = $derived(
    library.error ?? playlists.error ?? player.error ?? player.state.error,
  );

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

  // Only files in the app's music folder (downloads) are deleted; the backend checks.
  function removeLabel(track: Track): string {
    return track.youtube_id
      ? "Supprimer (le fichier est mis à la corbeille)"
      : "Retirer de la bibliothèque";
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
    <TrackTable tracks={filtered} onplay={playFrom} actionsWidth={group.isActive ? "7.5rem" : undefined}>
      {#snippet actions(track)}
        {#if group.isActive && track.youtube_id}
          <AddToGroup
            entry={{
              youtubeId: track.youtube_id,
              title: track.title,
              artist: track.artist,
              duration: track.duration,
            }}
          />
        {/if}
        <button
          class="row-action"
          onclick={() => player.enqueue([track.path])}
          aria-label="Ajouter à la file d'attente"
          title="Ajouter à la file d'attente"
        >
          <svg viewBox="0 0 24 24"><path d="M3 6h12v2H3zm0 5h12v2H3zm0 5h8v2H3zm14-2v-3h2v3h3v2h-3v3h-2v-3h-3v-2z" /></svg>
        </button>
        <AddToPlaylist path={track.path} />
        <button
          class="row-action"
          onclick={() => removeTrack(track)}
          aria-label={removeLabel(track)}
          title={removeLabel(track)}
        >
          ✕
        </button>
      {/snippet}
    </TrackTable>
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
</style>
