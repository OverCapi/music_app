<script lang="ts">
  import { tick } from "svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import type { Track } from "#lib/bindings.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import { playlists } from "#lib/stores/playlists.svelte.ts";
  import { formatTotal, plural } from "#lib/format.ts";
  import TrackCollection from "#lib/components/ui/TrackCollection.svelte";
  import TrackTable from "#lib/components/ui/TrackTable.svelte";
  import EnqueueButton from "#lib/components/library/EnqueueButton.svelte";
  import AddToPlaylist from "#lib/components/playlists/AddToPlaylist.svelte";

  const id = $derived(Number(page.params.id));
  const playlist = $derived(playlists.find(id));
  /** Playlist entries resolved against the library, in playlist order. */
  const tracks = $derived(
    (playlist?.track_ids ?? [])
      .map((trackId) => library.byId.get(trackId))
      .filter((t): t is Track => t !== undefined),
  );
  const totalDuration = $derived(tracks.reduce((sum, t) => sum + (t.duration ?? 0), 0));

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
      tracks.map((t) => t.id),
      index,
      "playlist",
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
    if ((await playlists.delete(id)) !== undefined) goto(resolve("/"));
  }
</script>

<section class="view">
  {#if !playlist}
    <p class="muted">Cette playlist n'existe plus.</p>
  {:else}
    <TrackCollection
      kind="Playlist"
      name={playlist.name}
      meta={plural(tracks.length, "titre") +
        (tracks.length > 0 ? ` · ${formatTotal(totalDuration)}` : "")}
      icon="note"
      playLabel="Lire la playlist"
      playDisabled={tracks.length === 0}
      onplay={() => playFrom(0)}
    >
      {#snippet title()}
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
      {/snippet}
      {#snippet toolbar()}
        <button class="secondary" onclick={startRename}>Renommer</button>
        <button
          class="secondary"
          class:danger={confirmDelete}
          onclick={deletePlaylist}
          onblur={() => (confirmDelete = false)}
        >
          {confirmDelete ? "Confirmer la suppression" : "Supprimer"}
        </button>
      {/snippet}

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
            <EnqueueButton trackId={track.id} source="playlist" />
            <AddToPlaylist trackId={track.id} exclude={id} />
            <button
              class="row-action"
              onclick={() => playlists.removeTrack(id, track.id)}
              aria-label="Retirer de la playlist"
              title="Retirer de la playlist"
            >
              ✕
            </button>
          {/snippet}
        </TrackTable>
      {/if}
    </TrackCollection>
  {/if}
</section>

<style>
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
    border-color: var(--danger);
    color: var(--danger);
  }
</style>
