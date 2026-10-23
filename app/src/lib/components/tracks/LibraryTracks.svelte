<script lang="ts">
  import { commands, type PlaySource, type Track } from "#lib/bindings.ts";
  import { formatTime } from "#lib/format.ts";
  import { progressive } from "#lib/progressive.svelte.ts";
  import { trackCover } from "#lib/covers.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import { playlists } from "#lib/stores/playlists.svelte.ts";
  import { attempt } from "#lib/stores/toasts.svelte.ts";
  import NotDownloaded from "#lib/components/library/NotDownloaded.svelte";
  import RowMenu, { type MenuItem } from "#lib/components/ui/RowMenu.svelte";
  import TrackList from "#lib/components/ui/TrackList.svelte";
  import TrackCell from "#lib/components/ui/TrackCell.svelte";
  import TrackMain from "#lib/components/ui/TrackMain.svelte";
  import TrackRow from "#lib/components/ui/TrackRow.svelte";

  /** Rows for library tracks with the "…" menu: favorite, queue, playlists, group, remove.
   * Inside a playlist (`playlistId`) the last entry removes from the playlist instead. */
  let {
    tracks,
    source,
    onplay,
    playlistId,
    empty,
  }: {
    tracks: Track[];
    source: PlaySource;
    /** Plays the list starting at `index`. */
    onplay: (index: number) => void;
    playlistId?: number;
    empty?: string;
  } = $props();

  const visible = progressive(() => tracks.length);

  // Only files in the app's music folder (downloads) are deleted; the backend checks.
  function removeLabel(track: Track): string {
    return track.youtube_id && track.path
      ? "Supprimer (fichier mis à la corbeille)"
      : "Retirer de la bibliothèque";
  }

  async function removeTrack(track: Track) {
    if (player.state.track_id === track.id) await player.stop();
    await library.remove(track.id);
  }

  function menu(track: Track): MenuItem[] {
    const items: MenuItem[] = [
      {
        label: track.favorite ? "Retirer des favoris" : "Favori",
        active: track.favorite,
        hint: "Un favori n'est jamais supprimé par l'optimisation du stockage",
        onclick: () => attempt(() => commands.setFavorite(track.id, !track.favorite)),
      },
      { label: "Ajouter à la file", onclick: () => player.enqueue([track.id], source) },
      {
        label: "Ajouter à une playlist",
        submenu: [
          {
            label: "Nouvelle playlist",
            prompt: {
              placeholder: "Nom de la playlist",
              submit: async (name) => {
                const created = await playlists.create(name);
                if (created) await playlists.addTracks(created.id, [track.id]);
              },
            },
          },
          ...playlists.items
            .filter((p) => p.id !== playlistId)
            .map((p) => {
              const already = p.track_ids.includes(track.id);
              return {
                label: p.name,
                check: already,
                disabled: already,
                hint: already ? "Déjà dans cette playlist" : undefined,
                onclick: () => playlists.addTracks(p.id, [track.id]),
              };
            }),
        ],
      },
    ];
    if (group.isActive && track.youtube_id) {
      const youtubeId = track.youtube_id;
      items.push({
        label: "Ajouter au groupe",
        onclick: () =>
          group.add({
            youtubeId,
            title: track.title,
            artist: track.artist,
            duration: track.duration || null,
          }),
      });
    }
    items.push(
      playlistId !== undefined
        ? {
            label: "Retirer de la playlist",
            danger: true,
            onclick: () => playlists.removeTrack(playlistId, track.id),
          }
        : { label: removeLabel(track), danger: true, onclick: () => removeTrack(track) },
    );
    return items;
  }
</script>

<TrackList {empty}>
  {#each tracks.slice(0, visible.count) as track, i (track.id)}
    {@const current = player.state.track_id === track.id}
    <TrackRow
      index={i}
      cover={trackCover(track)}
      {current}
      playing={current && player.state.playing}
      cols="minmax(0,2fr) minmax(0,1.3fr) minmax(0,1.3fr) 48px 34px"
      colsSmall="minmax(0,1fr) 44px 34px"
      title={track.path ?? "Pas encore téléchargé"}
      onplay={() => onplay(i)}
    >
      <TrackMain title={track.title} subtitle={track.artist ?? "Artiste inconnu"} subtitleSmallOnly>
        {#snippet badges()}
          {#if track.favorite}<span class="fav" title="Favori">★</span>{/if}
          {#if track.path === null}<NotDownloaded youtubeId={track.youtube_id} />{/if}
        {/snippet}
      </TrackMain>
      <TrackCell hideSmall>{track.artist ?? "Artiste inconnu"}</TrackCell>
      <TrackCell hideSmall>{track.album ?? ""}</TrackCell>
      <TrackCell kind="dur">{formatTime(track.duration)}</TrackCell>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <span class="menu" onclick={(e) => e.stopPropagation()}>
        <RowMenu items={menu(track)} />
      </span>
    </TrackRow>
  {/each}
</TrackList>

<style>
  .fav {
    flex: none;
    color: var(--accent);
    font-size: 12px;
  }
  .menu {
    display: grid;
    place-items: center;
  }
</style>
