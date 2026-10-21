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
  import { trackCover } from "#lib/covers.ts";
  import Button from "#lib/components/ui/Button.svelte";
  import PageHeader from "#lib/components/ui/PageHeader.svelte";
  import LibraryTracks from "#lib/components/tracks/LibraryTracks.svelte";

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
    player.playFrom(tracks, index, "playlist");
  }

  /** Plays the whole playlist; with shuffle on, from a random track. */
  function playAll() {
    playFrom(player.state.shuffle ? Math.floor(Math.random() * tracks.length) : 0);
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

<div class="page">
  {#if !playlist}
    <p class="muted">Cette playlist n'existe plus.</p>
  {:else}
    <PageHeader
      kind="Playlist"
      title={playlist.name}
      meta={plural(tracks.length, "titre") +
        (tracks.length > 0 ? ` · ${formatTotal(totalDuration)}` : "")}
      covers={tracks.slice(0, 4).map((t) => trackCover(t.id))}
      playLabel="Lire la playlist"
      playDisabled={tracks.length === 0}
      onplay={playAll}
    >
      {#snippet titleSlot()}
        {#if renaming}
          <form onsubmit={saveRename}>
            <input
              class="rename"
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
      {#snippet extra()}
        <Button onclick={startRename}>Renommer</Button>
        <Button variant="outline" onclick={deletePlaylist} onblur={() => (confirmDelete = false)}>
          {confirmDelete ? "Confirmer la suppression" : "Supprimer"}
        </Button>
      {/snippet}
    </PageHeader>

    <LibraryTracks
      {tracks}
      source="playlist"
      playlistId={id}
      onplay={playFrom}
      empty={tracks.length === 0 ? "Ajoute des titres depuis la bibliothèque." : undefined}
    />
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }
  .name {
    padding: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    letter-spacing: inherit;
    text-align: left;
    cursor: text;
  }
  .rename {
    width: 100%;
    box-sizing: border-box;
    border: 0;
    border-bottom: 2px solid var(--accent);
    background: transparent;
    color: var(--fg);
    font: inherit;
    font-size: clamp(30px, 5vw, 52px);
    font-weight: 700;
    letter-spacing: -0.04em;
    outline: none;
    user-select: text;
  }
</style>
