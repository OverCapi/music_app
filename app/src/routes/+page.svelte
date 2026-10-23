<script lang="ts">
  import CardGrid from "#lib/components/ui/CardGrid.svelte";
  import { onMount } from "svelte";
  import { resolve } from "$app/paths";
  import type { MixCategory } from "#lib/bindings.ts";
  import { home } from "#lib/stores/home.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";
  import { plural } from "#lib/format.ts";
  import { trackCover, youtubeCover } from "#lib/covers.ts";
  import Card from "#lib/components/ui/Card.svelte";
  import Cover from "#lib/components/ui/Cover.svelte";
  import IconButton from "#lib/components/ui/IconButton.svelte";
  import MediaCard from "#lib/components/ui/MediaCard.svelte";
  import MiniCard from "#lib/components/ui/MiniCard.svelte";
  import Panel from "#lib/components/ui/Panel.svelte";
  import PlayButton from "#lib/components/ui/PlayButton.svelte";
  import Section from "#lib/components/ui/Section.svelte";

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

  // "Reprendre": what is loaded in the player, otherwise the last thing played.
  const loaded = $derived(
    player.state.track_id !== null ? library.byId.get(player.state.track_id) : undefined,
  );
  const resume = $derived(
    loaded
      ? { title: loaded.title, artist: loaded.artist, cover: trackCover(loaded) }
      : playable[0]
        ? {
            title: playable[0].title,
            artist: playable[0].artist,
            cover: youtubeCover(playable[0].youtube_id),
          }
        : null,
  );

  function toggleResume() {
    if (loaded) player.togglePlay();
    else home.play(playable, 0);
  }

  const covers = (tracks: { youtube_id: string }[]) =>
    tracks.slice(0, 4).map((t) => youtubeCover(t.youtube_id));

  onMount(() => {
    home.load();
    home.loadRecent();
  });
</script>

<div class="page">
  <header>
    <h1 class="greeting">{greeting}</h1>
    <IconButton
      icon="refresh"
      label="Actualiser"
      size={40}
      disabled={home.loadingMixes || home.loadingDiscoveries}
      onclick={() => home.load(true)}
    />
  </header>

  {#if resume}
    <Panel radius={20} pad={20} class="resume">
      <div class="art" data-art><Cover src={resume.cover} /></div>
      <div class="info">
        <span class="kind">Reprendre</span>
        <div class="name">{resume.title}</div>
        <div class="by">{resume.artist ?? "Artiste inconnu"}</div>
      </div>
      <PlayButton
        size={76}
        playing={!!loaded && player.state.playing}
        label={loaded && player.state.playing ? "Pause" : "Lire"}
        onclick={toggleResume}
      />
    </Panel>
  {/if}

  <Section title="Tes mix">
    {#if home.loadingMixes && home.mixes.length === 0}
      <p class="status">Préparation de tes mix…</p>
    {:else if groups.length === 0}
      <p class="status">Écoute quelques titres pour que je te propose des mix.</p>
    {:else}
      {#each groups as group (group.id)}
        <Section label={group.label}>
          <CardGrid>
            {#each group.mixes as mix (mix.id)}
              <MediaCard
                title={mix.title}
                subtitle={`${plural(mix.tracks.length, "titre")} · ${group.label}`}
                covers={covers(mix.tracks)}
                mosaic
                href={resolve("/mix/[id]", { id: mix.id })}
                playLabel={`Lire ${mix.title}`}
                onplay={() => home.playAll(mix.tracks)}
              />
            {/each}
          </CardGrid>
        </Section>
      {/each}
    {/if}
  </Section>

  <Section title="Découvertes de la semaine">
    {#if home.loadingDiscoveries && discoveries.length === 0}
      <p class="status">Préparation de tes découvertes…</p>
    {:else if discoveries.length === 0}
      <p class="status">Écoute quelques titres pour que je te propose des découvertes.</p>
    {:else}
      <Card href={resolve("/discoveries")} radius={14} pad={14} class="wide">
        <span class="wide-art"><Cover src={covers(discoveries)} mosaic /></span>
        <span class="wide-info">
          <span class="wide-title">Découvertes</span>
          <span class="wide-meta">{plural(discoveries.length, "titre")} · De la semaine</span>
        </span>
        <PlayButton
          size={56}
          label="Lire les découvertes"
          onclick={(e: MouseEvent) => {
            e.preventDefault();
            e.stopPropagation();
            home.playAll(discoveries);
          }}
        />
      </Card>
    {/if}
  </Section>

  <Section title="Écoutés récemment">
    {#if playable.length === 0}
      <p class="status">Rien d'écouté pour l'instant.</p>
    {:else}
      <CardGrid min={240} gap={10}>
        {#each playable as track, i (track.youtube_id)}
          <MiniCard
            cover={youtubeCover(track.youtube_id)}
            title={track.title}
            subtitle={track.artist ?? "Artiste inconnu"}
            onclick={() => home.play(playable, i)}
          />
        {/each}
      </CardGrid>
    {/if}
  </Section>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 44px;
  }
  header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    padding-top: 20px;
  }
  .greeting {
    font-size: clamp(38px, 6vw, 60px) !important;
    letter-spacing: -0.045em !important;
  }
  .status {
    margin: 0;
    color: var(--mute);
    font-size: 14px;
  }

  /* Reprendre */
  .page :global(.resume) {
    display: grid;
    grid-template-columns: 180px minmax(0, 1fr) auto;
    gap: 32px;
    align-items: center;
  }
  .art {
    position: relative;
    aspect-ratio: 1;
    overflow: hidden;
    border-radius: var(--rs);
    box-shadow: 0 24px 50px -16px var(--shadow);
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }
  .kind {
    color: var(--mute);
    font-size: 13px;
    font-weight: 600;
  }
  .name {
    font-size: clamp(26px, 3.4vw, 40px);
    font-weight: 700;
    letter-spacing: -0.035em;
    line-height: 1.05;
    text-wrap: pretty;
  }
  .by {
    color: var(--mute);
    font-size: 16px;
  }
  .page :global(.resume > .play-btn) {
    margin-right: 16px;
  }

  /* Découvertes */
  .page :global(.wide) {
    display: grid;
    grid-template-columns: 120px minmax(0, 1fr) auto;
    gap: 24px;
    align-items: center;
    flex-direction: row;
  }
  .page :global(.wide > .play-btn) {
    margin-right: 10px;
  }
  .wide-art {
    position: relative;
    display: block;
    width: 120px;
    height: 120px;
    overflow: hidden;
    border-radius: var(--rs);
    box-shadow: 0 20px 40px -16px var(--shadow);
  }
  .wide-info {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .wide-title {
    font-size: 26px;
    font-weight: 700;
    letter-spacing: -0.03em;
  }
  .wide-meta {
    color: var(--mute);
    font-size: 13px;
  }

  @container page (max-width: 640px) {
    .page {
      gap: 32px;
    }
    .page :global(.resume) {
      grid-template-columns: 96px minmax(0, 1fr) auto;
      gap: 16px;
    }
    .page :global(.resume > .play-btn) {
      width: 56px !important;
      height: 56px !important;
      margin-right: 0;
    }
    .page :global(.wide) {
      grid-template-columns: 80px minmax(0, 1fr) auto;
      gap: 14px;
    }
    .wide-art {
      width: 80px;
      height: 80px;
    }
    .wide-title {
      font-size: 20px;
    }
    .by {
      font-size: 14px;
    }
  }
</style>
