<script lang="ts">
  import { onMount } from "svelte";
  import type { Mix, MixCategory } from "#lib/bindings.ts";
  import { home } from "#lib/stores/home.svelte.ts";
  import { plural } from "#lib/format.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import PlayHeader from "#lib/components/home/PlayHeader.svelte";
  import SuggestionList from "#lib/components/home/SuggestionList.svelte";

  const categories: { id: MixCategory; label: string }[] = [
    { id: "known", label: "Que du connu" },
    { id: "new", label: "Que du nouveau" },
    { id: "mixed", label: "Mélangé" },
  ];

  let selectedId = $state<string | null>(null);

  const groups = $derived(
    categories
      .map((c) => ({ ...c, mixes: home.mixes.filter((m) => m.category === c.id) }))
      .filter((g) => g.mixes.length > 0),
  );
  const selected = $derived(home.mixes.find((m) => m.id === selectedId) ?? null);
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

  onMount(() => {
    home.load();
    home.loadRecent();
  });

  function toggle(mix: Mix) {
    selectedId = selectedId === mix.id ? null : mix.id;
  }
</script>

<section class="view">
  <h1>Accueil</h1>

  <div class="shelf">
    <PlayHeader title="Tes mix">
      <button
        class="pill secondary"
        onclick={() => home.load(true)}
        disabled={home.loadingMixes || home.loadingDiscoveries}
      >
        Actualiser
      </button>
    </PlayHeader>

    {#if home.loadingMixes && home.mixes.length === 0}
      <p class="status">Préparation de tes mix…</p>
    {:else if groups.length === 0}
      <p class="status">Écoute quelques titres pour que je te propose des mix.</p>
    {:else}
      {#each groups as group (group.id)}
        <p class="section-title">{group.label}</p>
        <div class="cards">
          {#each group.mixes as mix (mix.id)}
            <button
              class="card mix"
              class:selected={mix.id === selectedId}
              onclick={() => toggle(mix)}
              aria-pressed={mix.id === selectedId}
            >
              <span class="cover" data-category={mix.category}><Icon name="note" size={40} /></span>
              <span class="mix-title">{mix.title}</span>
              <span class="muted count">{plural(mix.tracks.length, "titre")}</span>
            </button>
          {/each}
        </div>
      {/each}

      {#if selected}
        <div class="detail">
          <PlayHeader
            title={selected.title}
            playLabel="Lire ce mix"
            onplay={() => home.play(selected.tracks, 0)}
          />
          <SuggestionList tracks={selected.tracks} onplay={(i) => home.play(selected.tracks, i)} />
        </div>
      {/if}
    {/if}
  </div>

  <div class="shelf">
    <PlayHeader
      title="Découvertes de la semaine"
      playLabel="Lire les découvertes"
      playDisabled={discoveries.length === 0}
      onplay={() => home.play(discoveries, 0)}
    >
      <button
        class="pill secondary"
        onclick={() => home.refreshDiscoveries()}
        disabled={home.loadingDiscoveries}
      >
        Régénérer
      </button>
    </PlayHeader>
    {#if home.loadingDiscoveries && discoveries.length === 0}
      <p class="status">Préparation de tes découvertes…</p>
    {:else if discoveries.length === 0}
      <p class="status">Écoute quelques titres pour que je te propose des découvertes.</p>
    {:else}
      <SuggestionList tracks={discoveries} onplay={(i) => home.play(discoveries, i)} />
    {/if}
  </div>

  <div class="shelf">
    <PlayHeader title="Écoutés récemment" />
    {#if playable.length === 0}
      <p class="status">Rien d'écouté pour l'instant.</p>
    {:else}
      <SuggestionList tracks={playable} showKnown={false} onplay={(i) => home.play(playable, i)} />
    {/if}
  </div>
</section>

<style>
  .shelf {
    margin-top: 2rem;
  }

  .shelf:first-of-type {
    margin-top: 1.5rem;
  }

  .status {
    margin: 0;
    color: var(--muted);
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 1rem;
  }

  .mix {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 1rem;
    color: var(--text);
    text-align: left;
  }

  .mix:hover,
  .mix.selected {
    background: var(--surface-2);
  }

  .cover {
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    margin-bottom: 0.5rem;
    border-radius: 6px;
    background: linear-gradient(135deg, #450af5, #8e8ee5);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.4);
    color: rgb(255 255 255 / 0.85);
  }

  .cover[data-category="known"] {
    background: linear-gradient(135deg, #0d7377, #14a085);
  }

  .cover[data-category="new"] {
    background: linear-gradient(135deg, #c2185b, #ff7043);
  }

  .mix-title {
    overflow: hidden;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    font-size: 0.8rem;
  }

  .detail {
    margin-top: 1.5rem;
  }
</style>
