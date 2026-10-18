<script lang="ts">
  import { onMount } from "svelte";
  import { resolve } from "$app/paths";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { MixCategory, Playlist, Track } from "#lib/bindings.ts";
  import { home } from "#lib/stores/home.svelte.ts";
  import { playlists } from "#lib/stores/playlists.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import { plural } from "#lib/format.ts";
  import MediaCard from "#lib/components/home/MediaCard.svelte";
  import Thumb from "#lib/components/home/Thumb.svelte";

  const categories: { id: MixCategory; label: string }[] = [
    { id: "known", label: "Que du connu" },
    { id: "new", label: "Que du nouveau" },
    { id: "mixed", label: "Mélangé" },
  ];

  const groups = $derived(
    categories
      .map((c) => ({ ...c, mixes: home.mixes.filter((m) => m.category === c.id) }))
      .filter((g) => g.mixes.length > 0),
  );
  const discoveries = $derived(home.discoveries?.tracks ?? []);
  // Group tracks and tracks without a YouTube id cannot be replayed from here. A track
  // listened to several times appears once, at its most recent listen.
  const playable = $derived(
    home.recent
      .filter(
        (p, i) => p.youtube_id && home.recent.findIndex((q) => q.youtube_id === p.youtube_id) === i,
      )
      .map((p) => ({
        youtube_id: p.youtube_id!,
        title: p.title,
        artist: p.artist,
        duration: null,
        known: true,
      })),
  );

  const hour = new Date().getHours();
  const greeting = hour < 5 || hour >= 18 ? "Bonsoir" : "Bonjour";

  function tracksOf(playlist: Playlist): Track[] {
    return playlist.track_ids
      .map((id) => library.byId.get(id))
      .filter((t): t is Track => t !== undefined);
  }

  onMount(() => {
    home.load();
    home.loadRecent();
  });
</script>

<section class="view">
  <header>
    <div>
      <h1 class="greeting">{greeting}</h1>
      <p class="muted">Tes playlists, tes mix et ce que tu as écouté récemment.</p>
    </div>
    <button
      class="pill secondary"
      onclick={() => home.load(true)}
      disabled={home.loadingMixes || home.loadingDiscoveries}
    >
      Actualiser
    </button>
  </header>

  <h2>Tes playlists</h2>
  {#if playlists.items.length === 0}
    <p class="status">Crée ta première playlist avec le bouton + du menu de gauche.</p>
  {:else}
    <div class="grid">
      {#each playlists.items as playlist (playlist.id)}
        {@const tracks = tracksOf(playlist)}
        <MediaCard
          title={playlist.name}
          subtitle={plural(playlist.track_ids.length, "titre")}
          href={resolve("/playlist/[id]", { id: String(playlist.id) })}
          playLabel={`Lire ${playlist.name}`}
          onplay={() => tracks.length > 0 && player.playFrom(tracks, 0, "playlist")}
        >
          {#snippet cover()}
            <span class="mosaic" class:single={tracks.length < 4}>
              {#each tracks.slice(0, tracks.length < 4 ? 1 : 4) as track (track.id)}
                <img
                  src={convertFileSrc(String(track.id), "cover")}
                  alt=""
                  loading="lazy"
                  onerror={(e) => ((e.currentTarget as HTMLElement).style.visibility = "hidden")}
                />
              {/each}
            </span>
          {/snippet}
        </MediaCard>
      {/each}
    </div>
  {/if}

  <h2>Tes mix</h2>
  {#if home.loadingMixes && home.mixes.length === 0}
    <p class="status">Préparation de tes mix…</p>
  {:else if groups.length === 0}
    <p class="status">Écoute quelques titres pour que je te propose des mix.</p>
  {:else}
    {#each groups as group (group.id)}
      <h3>{group.label}</h3>
      <div class="grid">
        {#each group.mixes as mix (mix.id)}
          <MediaCard
            title={mix.title}
            subtitle={`${plural(mix.tracks.length, "titre")} · ${group.label}`}
            tracks={mix.tracks}
            category={mix.category}
            href={resolve("/mix/[id]", { id: mix.id })}
            playLabel={`Lire ${mix.title}`}
            onplay={() => home.playAll(mix.tracks)}
          />
        {/each}
      </div>
    {/each}
  {/if}

  <h2>Découvertes de la semaine</h2>
  {#if home.loadingDiscoveries && discoveries.length === 0}
    <p class="status">Préparation de tes découvertes…</p>
  {:else if discoveries.length === 0}
    <p class="status">Écoute quelques titres pour que je te propose des découvertes.</p>
  {:else}
    <div class="grid">
      <MediaCard
        title="Découvertes"
        subtitle={`${plural(discoveries.length, "titre")} · De la semaine`}
        tracks={discoveries}
        href={resolve("/discoveries")}
        playLabel="Lire les découvertes"
        onplay={() => home.playAll(discoveries)}
      />
    </div>
  {/if}

  <h2>Écoutés récemment</h2>
  {#if playable.length === 0}
    <p class="status">Rien d'écouté pour l'instant.</p>
  {:else}
    <div class="grid small">
      {#each playable as track, i (track.youtube_id)}
        <button class="recent" onclick={() => home.play(playable, i)} title="Lire">
          <span class="img"><Thumb id={track.youtube_id} /></span>
          <span class="meta">
            <span class="name">{track.title}</span>
            <span class="artist">{track.artist ?? "Artiste inconnu"}</span>
          </span>
        </button>
      {/each}
    </div>
  {/if}
</section>

<style>
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.5rem;
  }

  header p {
    margin: 0.4rem 0 0;
  }

  header :global(.pill) {
    height: 40px;
  }

  .view h1.greeting {
    font-size: 2.5rem;
    letter-spacing: -0.02em;
    line-height: 1.1;
  }

  h2 {
    margin: 2.5rem 0 0.25rem;
    font-size: 1.1rem;
    font-weight: 700;
  }

  h3 {
    margin: 1.25rem 0 0.75rem;
    color: var(--muted);
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .mosaic {
    display: grid;
    grid-template: 1fr 1fr / 1fr 1fr;
    width: 100%;
    height: 100%;
  }

  .mosaic.single {
    grid-template: 1fr / 1fr;
  }

  .mosaic img {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    object-fit: cover;
  }

  .status {
    margin: 0.75rem 0 0;
    color: var(--muted);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(176px, 1fr));
    gap: 1rem;
  }

  h2 + .grid {
    margin-top: 0.75rem;
  }

  .grid.small {
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 0.5rem;
  }

  .recent {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 0;
    padding: 0.5rem;
    border-radius: 12px;
    background: var(--surface);
    color: var(--text);
    text-align: left;
  }

  .recent:hover,
  .recent:focus-visible {
    background: var(--surface-2);
    outline: none;
  }

  .img {
    flex: none;
    width: 48px;
    height: 48px;
    overflow: hidden;
    border-radius: 8px;
  }

  .img :global(.thumb) {
    width: 100%;
    height: 100%;
  }

  .meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name,
  .artist {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name {
    font-size: 0.9rem;
    font-weight: 600;
  }

  .artist {
    color: var(--muted);
    font-size: 0.8rem;
  }
</style>
