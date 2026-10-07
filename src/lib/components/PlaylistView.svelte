<script lang="ts">
  import { tick } from "svelte";
  import { library, type Track } from "$lib/library.svelte";
  import { nav } from "$lib/navigation.svelte";
  import { player } from "$lib/player.svelte";
  import { playlists } from "$lib/playlists.svelte";
  import AddToPlaylist from "$lib/components/AddToPlaylist.svelte";
  import TrackTable from "$lib/components/TrackTable.svelte";

  let { id }: { id: string } = $props();

  const playlist = $derived(playlists.find(id));
  /** Playlist entries resolved against the library, in playlist order. */
  const tracks = $derived(
    (playlist?.tracks ?? [])
      .map((path) => library.find(path))
      .filter((t): t is Track => t !== undefined),
  );
  const totalDuration = $derived(tracks.reduce((sum, t) => sum + t.duration, 0));
  const error = $derived(playlists.error ?? player.error ?? player.state.error);

  let renaming = $state(false);
  let nameDraft = $state("");
  let nameInput = $state<HTMLInputElement>();
  let confirmDelete = $state(false);

  // Reset local UI state when switching to another playlist.
  $effect(() => {
    void id;
    renaming = false;
    confirmDelete = false;
  });

  function playFrom(index: number) {
    player.playFrom(
      tracks.map((t) => t.path),
      index,
    );
  }

  async function startRename() {
    if (!playlist) return;
    nameDraft = playlist.name;
    renaming = true;
    await tick();
    nameInput?.select();
  }

  async function saveRename(e?: SubmitEvent) {
    e?.preventDefault();
    if (!renaming) return;
    renaming = false;
    if (playlist && nameDraft.trim() && nameDraft.trim() !== playlist.name) {
      await playlists.rename(id, nameDraft);
    }
  }

  async function deletePlaylist() {
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    await playlists.delete(id);
    nav.go({ kind: "library" });
  }

  function formatTotal(secs: number): string {
    const minutes = Math.round(secs / 60);
    if (minutes < 60) return `${minutes} min`;
    return `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
  }
</script>

<section class="playlist">
  {#if !playlist}
    <p class="muted">Cette playlist n'existe plus.</p>
  {:else}
    <header>
      <div class="cover" aria-hidden="true">
        <svg viewBox="0 0 24 24"><path d="M9 18a3 3 0 1 1-2-2.8V5l12-2v11a3 3 0 1 1-2-2.8V5.4l-8 1.3z" /></svg>
      </div>
      <div class="info">
        <p class="kind">Playlist</p>
        {#if renaming}
          <form onsubmit={saveRename}>
            <input
              bind:this={nameInput}
              bind:value={nameDraft}
              maxlength="100"
              onblur={() => saveRename()}
              onkeydown={(e) => e.key === "Escape" && (renaming = false)}
              aria-label="Nom de la playlist"
            />
          </form>
        {:else}
          <h1>
            <button class="name" onclick={startRename} title="Renommer">{playlist.name}</button>
          </h1>
        {/if}
        <p class="meta">
          {tracks.length} titre{tracks.length > 1 ? "s" : ""}
          {#if tracks.length > 0}· {formatTotal(totalDuration)}{/if}
        </p>
      </div>
    </header>

    <div class="toolbar">
      <button
        class="play"
        onclick={() => playFrom(0)}
        disabled={tracks.length === 0}
        aria-label="Lire la playlist"
        title="Lire la playlist"
      >
        <svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>
      </button>
      <button class="secondary" onclick={startRename}>Renommer</button>
      <button
        class="secondary"
        class:danger={confirmDelete}
        onclick={deletePlaylist}
        onblur={() => (confirmDelete = false)}
      >
        {confirmDelete ? "Confirmer la suppression" : "Supprimer"}
      </button>
    </div>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    {#if tracks.length === 0}
      <div class="empty">
        <p>Cette playlist est vide.</p>
        <p class="muted">
          Ajoute des titres depuis la bibliothèque avec le bouton « + » de chaque ligne.
        </p>
      </div>
    {:else}
      <TrackTable {tracks} onplay={playFrom}>
        {#snippet actions(track)}
          <button
            class="row-action"
            onclick={() => player.enqueue([track.path])}
            aria-label="Ajouter à la file d'attente"
            title="Ajouter à la file d'attente"
          >
            <svg viewBox="0 0 24 24"><path d="M3 6h12v2H3zm0 5h12v2H3zm0 5h8v2H3zm14-2v-3h2v3h3v2h-3v3h-2v-3h-3v-2z" /></svg>
          </button>
          <AddToPlaylist path={track.path} exclude={id} />
          <button
            class="row-action"
            onclick={() => playlists.removeTrack(id, track.path)}
            aria-label="Retirer de la playlist"
            title="Retirer de la playlist"
          >
            ✕
          </button>
        {/snippet}
      </TrackTable>
    {/if}
  {/if}
</section>

<style>
  .playlist {
    padding: 1.5rem 2rem;
    overflow-y: auto;
    min-height: 0;
  }

  header {
    display: flex;
    align-items: flex-end;
    gap: 1.5rem;
    margin-bottom: 1.5rem;
  }

  .cover {
    display: grid;
    flex: none;
    place-items: center;
    width: 160px;
    height: 160px;
    border-radius: 6px;
    background: linear-gradient(135deg, #450af5, #8e8ee5);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.5);
  }

  .cover svg {
    width: 64px;
    height: 64px;
    fill: rgb(255 255 255 / 0.85);
  }

  .info {
    min-width: 0;
  }

  .kind {
    margin: 0;
    font-size: 0.8rem;
    font-weight: 600;
  }

  h1 {
    margin: 0.3rem 0;
    font-size: 2.5rem;
  }

  .name {
    max-width: 100%;
    padding: 0;
    overflow: hidden;
    background: transparent;
    color: var(--text);
    font-size: inherit;
    font-weight: inherit;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  input {
    width: min(100%, 480px);
    margin: 0.3rem 0;
    padding: 0.2rem 0.5rem;
    border: 1px solid var(--muted);
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: 2rem;
    font-weight: 700;
    outline: none;
  }

  .meta {
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .play {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    padding: 0;
    border-radius: 50%;
    background: var(--accent);
    color: #000;
  }

  .play:not(:disabled):hover {
    transform: scale(1.05);
  }

  .play svg {
    width: 26px;
    height: 26px;
    fill: currentColor;
  }

  .secondary {
    padding: 0.45rem 1rem;
    border: 1px solid var(--track);
    border-radius: 999px;
    background: transparent;
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 600;
  }

  .secondary:hover {
    border-color: var(--text);
    color: var(--text);
  }

  .secondary.danger {
    border-color: #f15e6c;
    color: #f15e6c;
  }

  .error {
    margin: 0 0 1rem;
    color: #f15e6c;
  }

  .empty {
    padding: 3rem 0;
    text-align: center;
  }

  .empty p {
    margin: 0.3rem 0;
  }

  .muted {
    color: var(--muted);
  }
</style>
