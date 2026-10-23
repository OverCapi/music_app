<script lang="ts">
  import CardGrid from "#lib/components/ui/CardGrid.svelte";
  import Field from "#lib/components/ui/Field.svelte";
  import EmptyState from "#lib/components/ui/EmptyState.svelte";
  import { tick } from "svelte";
  import { SvelteMap } from "svelte/reactivity";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { Track } from "#lib/bindings.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import { attempt } from "#lib/stores/toasts.svelte.ts";
  import { formatTotal, plural } from "#lib/format.ts";
  import { trackCover } from "#lib/covers.ts";
  import { enterPage, liquid } from "#lib/motion.ts";
  import { progressive } from "#lib/progressive.svelte.ts";
  import Button from "#lib/components/ui/Button.svelte";
  import Cover from "#lib/components/ui/Cover.svelte";
  import Icon from "#lib/components/ui/Icon.svelte";
  import MediaCard from "#lib/components/ui/MediaCard.svelte";
  import SegmentedPills from "#lib/components/ui/SegmentedPills.svelte";
  import LibraryTracks from "#lib/components/tracks/LibraryTracks.svelte";

  type Tab = "albums" | "artists" | "tracks";
  const tabs: { id: Tab; label: string }[] = [
    { id: "tracks", label: "Titres" },
    { id: "albums", label: "Albums" },
    { id: "artists", label: "Artistes" },
  ];

  let tab = $state<Tab>("tracks");
  let body = $state<HTMLElement>();
  let query = $state("");

  const norm = (s: string | null) =>
    (s ?? "").normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
  const needle = $derived(norm(query.trim()));
  const filtered = $derived(
    needle
      ? library.tracks.filter((t) =>
          norm(`${t.title} ${t.artist ?? ""} ${t.album ?? ""}`).includes(needle),
        )
      : library.tracks,
  );

  const totalDuration = $derived(library.tracks.reduce((sum, t) => sum + (t.duration ?? 0), 0));
  const notDownloaded = $derived(library.tracks.filter((t) => t.path === null).length);
  const meta = $derived(
    plural(library.tracks.length, "titre") +
      (library.tracks.length > 0 ? ` · ${formatTotal(totalDuration)}` : "") +
      (notDownloaded > 0 ? ` · ${notDownloaded} à télécharger à l'écoute` : ""),
  );

  type Group = { key: string; title: string; artist: string | null; tracks: Track[] };

  /** Groups tracks by `key` (library order is kept inside each group). */
  function groupBy(keyOf: (t: Track) => string, titleOf: (t: Track) => string): Group[] {
    const map = new SvelteMap<string, Group>();
    for (const t of filtered) {
      const key = keyOf(t);
      let g = map.get(key);
      if (!g) map.set(key, (g = { key, title: titleOf(t), artist: t.artist, tracks: [] }));
      g.tracks.push(t);
    }
    return [...map.values()];
  }

  // Albums and artists come from the tracks' metadata (there is no album table).
  const albums = $derived(
    groupBy(
      (t) => `${t.album ?? ""}\u0000${t.artist ?? ""}`,
      (t) => t.album ?? "Sans album",
    ).sort((a, b) => a.title.localeCompare(b.title, "fr")),
  );
  const artists = $derived(
    groupBy(
      (t) => t.artist ?? "",
      (t) => t.artist ?? "Artiste inconnu",
    ).sort((a, b) => a.title.localeCompare(b.title, "fr")),
  );

  const shownAlbums = progressive(() => albums.length);
  const shownArtists = progressive(() => artists.length);

  const coverOf = (tracks: Track[]) => trackCover(tracks[0]);

  async function selectTab(id: string) {
    if (id === tab) return;
    tab = id as Tab;
    await tick();
    if (body) enterPage(body);
  }

  async function addFiles() {
    const paths = await attempt(() =>
      open({ multiple: true, filters: [{ name: "Audio", extensions: ["mp3", "opus"] }] }),
    );
    if (paths && paths.length > 0) await library.add(paths);
  }

  function playList(tracks: Track[], index: number) {
    player.playFrom(tracks, index, "library");
  }
</script>

<div class="page">
  <header>
    <div class="title">
      <h1>Bibliothèque</h1>
      <span class="meta">{meta}</span>
    </div>
    <div class="tools">
      <Field
        variant="search"
        type="search"
        placeholder="Rechercher…"
        aria-label="Rechercher dans la bibliothèque"
        bind:value={query}
      />
      <SegmentedPills items={tabs} value={tab} onselect={selectTab} />
      <Button onclick={addFiles} disabled={library.adding}>
        <Icon name="plus" size={14} />{library.adding ? "Ajout…" : "Ajouter"}
      </Button>
    </div>
  </header>

  <div bind:this={body}>
    {#if library.tracks.length === 0}
      <EmptyState>
        <p>Ta bibliothèque est vide.</p>
        <p class="muted">Ajoute des fichiers mp3 ou opus pour commencer.</p>
      </EmptyState>
    {:else if filtered.length === 0}
      <EmptyState><p class="muted">Aucun résultat pour « {query} ».</p></EmptyState>
    {:else if tab === "albums"}
      <CardGrid min={170} gap={20} minSmall={140} gapSmall={14}>
        {#each albums.slice(0, shownAlbums.count) as album (album.key)}
          <MediaCard
            title={album.title}
            subtitle={album.artist ?? "Artiste inconnu"}
            covers={coverOf(album.tracks)}
            onplay={() => playList(album.tracks, 0)}
            playLabel={`Lire ${album.title}`}
          />
        {/each}
      </CardGrid>
    {:else if tab === "artists"}
      <CardGrid min={150} gap={28} minSmall={110} gapSmall={18}>
        {#each artists.slice(0, shownArtists.count) as artist (artist.key)}
          <button
            class="artist"
            type="button"
            data-liquid
            use:liquid
            onclick={() => playList(artist.tracks, 0)}
            title={`Lire ${artist.title}`}
          >
            <span class="avatar"><Cover src={coverOf(artist.tracks)} /></span>
            <span class="name">{artist.title}</span>
            <span class="count">{plural(artist.tracks.length, "titre")}</span>
          </button>
        {/each}
      </CardGrid>
    {:else}
      <LibraryTracks tracks={filtered} source="library" onplay={(i) => playList(filtered, i)} />
    {/if}
  </div>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 16px 24px;
    padding-top: 20px;
  }
  .title {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .meta {
    color: var(--mute);
    font-size: 13px;
  }
  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    max-width: 100%;
  }
  .artist {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    background: transparent;
    color: var(--fg);
    cursor: pointer;
    transition: transform 0.45s var(--ease-spring);
  }
  .artist:hover {
    transform: translateY(-5px);
  }
  .avatar {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 50%;
    box-shadow:
      inset 0 1px 0 var(--hi),
      0 20px 40px -18px var(--shadow);
  }
  .name {
    font-size: 15px;
    font-weight: 600;
  }
  .count {
    margin-top: -6px;
    color: var(--mute);
    font-size: 12px;
  }
</style>
