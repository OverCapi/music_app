<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { commands, type Track } from "#lib/bindings.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { attempt } from "#lib/stores/toasts.svelte.ts";
  import { formatTotal, plural } from "#lib/format.ts";
  import TrackCollection from "#lib/components/ui/TrackCollection.svelte";
  import TrackTable from "#lib/components/ui/TrackTable.svelte";
  import EnqueueButton from "#lib/components/library/EnqueueButton.svelte";
  import AddToPlaylist from "#lib/components/playlists/AddToPlaylist.svelte";
  import AddToGroup from "#lib/components/group/AddToGroup.svelte";

  let query = $state("");

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return library.tracks;
    return library.tracks.filter((t) =>
      [t.title, t.artist, t.album].some((field) => field?.toLowerCase().includes(q)),
    );
  });

  const totalDuration = $derived(library.tracks.reduce((sum, t) => sum + (t.duration ?? 0), 0));
  const notDownloaded = $derived(library.tracks.filter((t) => t.path === null).length);

  async function addFiles() {
    const paths = await attempt(() =>
      open({ multiple: true, filters: [{ name: "Audio", extensions: ["mp3", "opus"] }] }),
    );
    if (paths && paths.length > 0) await library.add(paths);
  }

  /** Plays the visible list (like a Spotify playlist), starting at `index`. */
  function playFrom(index: number) {
    player.playFrom(filtered, index, "library");
  }

  /** Plays the whole visible list; with shuffle on, from a random track. */
  function playAll() {
    playFrom(player.state.shuffle ? Math.floor(Math.random() * filtered.length) : 0);
  }

  // Only files in the app's music folder (downloads) are deleted; the backend checks.
  function removeLabel(track: Track): string {
    return track.youtube_id && track.path
      ? "Supprimer (le fichier est mis à la corbeille)"
      : "Retirer de la bibliothèque";
  }

  async function removeTrack(track: Track) {
    if (player.state.track_id === track.id) await player.stop();
    await library.remove(track.id);
  }
</script>

<section class="view">
  <TrackCollection
    kind="Bibliothèque"
    name="Bibliothèque"
    meta={plural(library.tracks.length, "titre") +
      (library.tracks.length > 0 ? ` · ${formatTotal(totalDuration)}` : "") +
      (notDownloaded > 0 ? ` · ${notDownloaded} à télécharger à l'écoute` : "")}
    icon="library"
    playLabel="Lire la bibliothèque"
    playDisabled={filtered.length === 0}
    onplay={playAll}
  >
    {#snippet toolbar()}
      {#if library.tracks.length > 0}
        <input class="search" type="search" placeholder="Rechercher" bind:value={query} />
      {/if}
      <button class="pill primary" onclick={addFiles} disabled={library.adding}>
        {library.adding ? "Ajout…" : "Ajouter des titres"}
      </button>
    {/snippet}

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
      <TrackTable
        tracks={filtered}
        onplay={playFrom}
        actionsWidth={group.isActive ? "9.5rem" : "7.5rem"}
      >
        {#snippet actions(track)}
          {#if group.isActive && track.youtube_id}
            <AddToGroup
              entry={{
                youtubeId: track.youtube_id,
                title: track.title,
                artist: track.artist,
                duration: track.duration || null,
              }}
            />
          {/if}
          <button
            class="row-action"
            class:favorite={track.favorite}
            onclick={() => attempt(() => commands.setFavorite(track.id, !track.favorite))}
            aria-label={track.favorite ? "Retirer des favoris" : "Ajouter aux favoris"}
            title={track.favorite
              ? "Favori : le fichier n'est jamais supprimé"
              : "Marquer comme favori (fichier conservé)"}
          >
            {track.favorite ? "★" : "☆"}
          </button>
          <EnqueueButton trackId={track.id} source="library" />
          <AddToPlaylist trackId={track.id} />
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
  </TrackCollection>
</section>

<style>
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

  .empty {
    padding: 4rem 0;
  }
</style>
