<script lang="ts">
  import { search } from "#lib/stores/search.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { home } from "#lib/stores/home.svelte.ts";
  import { formatTime, formatViews } from "#lib/format.ts";
  import DownloadButton from "#lib/components/search/DownloadButton.svelte";
  import AddToGroup from "#lib/components/group/AddToGroup.svelte";
  import PlayHeader from "#lib/components/home/PlayHeader.svelte";
  import SuggestionList from "#lib/components/home/SuggestionList.svelte";

  const discoveries = $derived(home.discoveries?.tracks ?? []);
  const showDiscoveries = $derived(!search.lastQuery && !search.query.trim());

  // Discoveries are loaded lazily, only once the idle search page needs them.
  $effect(() => {
    if (showDiscoveries) home.loadDiscoveries();
  });
</script>

<section class="view">
  <header>
    <h1>{search.lastQuery ? `Résultats pour « ${search.lastQuery} »` : "Rechercher"}</h1>
  </header>

  {#if search.loading}
    <p class="status">Recherche en cours…</p>
  {:else if search.lastQuery && search.results.length === 0}
    <p class="status">Aucun résultat pour « {search.lastQuery} ».</p>
  {:else if !search.lastQuery}
    <p class="status">
      Tape ta recherche dans la barre du haut (<span class="kbd">⌘K</span>) puis Entrée.
    </p>
  {/if}

  {#if showDiscoveries && discoveries.length > 0}
    <div class="discoveries">
      <PlayHeader
        title="Découvertes de la semaine"
        playLabel="Lire les découvertes"
        onplay={() => home.play(discoveries, 0)}
      />
      <SuggestionList tracks={discoveries} onplay={(i) => home.play(discoveries, i)} />
    </div>
  {/if}

  {#if search.results.length > 0}
    <div class="results card" class:stale={search.loading} class:in-group={group.isActive}>
      <div class="row head">
        <span>#</span>
        <span>Titre</span>
        <span>Album / Vues</span>
        <span class="right">Durée</span>
        <span></span>
      </div>
      {#each search.results as result, i (result.id)}
        <div class="row result">
          <span class="index">{i + 1}</span>
          <span class="title-cell">
            <img src={result.thumbnail} alt="" loading="lazy" />
            <span class="meta">
              <span class="title">{result.title}</span>
              {#if result.details_pending}
                <span class="placeholder"></span>
              {:else}
                <span class="artist">{result.artist ?? ""}</span>
              {/if}
            </span>
          </span>
          <span class="album">
            {#if result.details_pending}
              <span class="placeholder"></span>
            {:else if result.album}
              {result.album}
            {:else if result.views !== null}
              {formatViews(result.views)}
            {/if}
          </span>
          <span class="duration right">
            {#if result.details_pending}
              <span class="placeholder short"></span>
            {:else if result.duration !== null}
              {formatTime(result.duration)}
            {/if}
          </span>
          <span class="action">
            {#if group.isActive}
              <AddToGroup
                size={22}
                entry={{
                  youtubeId: result.id,
                  title: result.title,
                  artist: result.artist,
                  duration: result.duration,
                }}
              />
            {/if}
            <DownloadButton id={result.id} />
          </span>
        </div>
      {/each}
    </div>
    {#if search.hasMore && !search.loading}
      <div class="more">
        <button onclick={() => search.loadMore()} disabled={search.loadingMore}>
          {search.loadingMore ? "Chargement…" : "Plus de résultats"}
        </button>
      </div>
    {/if}
  {/if}
</section>

<style>
  header {
    margin-bottom: 1.25rem;
  }

  .more {
    display: flex;
    justify-content: center;
    padding: 1.5rem 0 0.5rem;
  }

  .more button {
    height: 40px;
    padding: 0 1.25rem;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.9rem;
    font-weight: 700;
  }

  .more button:not(:disabled):hover {
    background: var(--surface-3);
  }

  .discoveries {
    margin-top: 2rem;
  }

  .status {
    margin: 0 0 1rem;
    color: var(--muted);
  }

  .results {
    padding: 8px;
  }

  .results.stale {
    opacity: 0.5;
  }

  .row {
    display: grid;
    grid-template-columns: 2.5rem minmax(0, 3fr) minmax(0, 2fr) 4rem 2.5rem;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 0.75rem;
    border-radius: 12px;
  }

  .in-group .row {
    grid-template-columns: 2.5rem minmax(0, 3fr) minmax(0, 2fr) 4rem 4.5rem;
  }

  .head {
    margin-bottom: 0.5rem;
    border-bottom: 1px solid var(--line);
    border-radius: 0;
    color: var(--muted);
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .result {
    color: var(--muted);
    font-size: 0.9rem;
  }

  .result:hover {
    background: var(--surface-2);
  }

  .index {
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  .right {
    text-align: right;
  }

  .title-cell {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 0;
  }

  img {
    flex: none;
    width: 44px;
    height: 44px;
    border-radius: 8px;
    object-fit: cover;
    background: var(--surface-2);
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
  }

  .title,
  .artist,
  .album {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    color: var(--text);
    font-weight: 600;
  }

  .artist {
    font-size: 0.8rem;
  }

  .action {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  /* Same look as DownloadButton. */
  .action :global(.add-group) {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--muted);
  }

  .action :global(.add-group:hover) {
    color: var(--text);
    transform: scale(1.08);
  }

  .duration {
    font-variant-numeric: tabular-nums;
  }

  .placeholder {
    display: inline-block;
    width: 7rem;
    max-width: 100%;
    height: 0.7rem;
    border-radius: 3px;
    background: var(--surface-2);
    animation: pulse 1.2s ease-in-out infinite;
  }

  .placeholder.short {
    width: 2.5rem;
  }

  @keyframes pulse {
    50% {
      opacity: 0.4;
    }
  }
</style>
