<script lang="ts">
  import { search } from "#lib/stores/search.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { home } from "#lib/stores/home.svelte.ts";
  import { formatTime, formatViews } from "#lib/format.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
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

  function onsubmit(e: SubmitEvent) {
    e.preventDefault();
    search.search();
  }
</script>

<section class="view">
  <header>
    <h1>Rechercher</h1>
    <form {onsubmit}>
      <Icon name="search" size={18} />
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="search"
        placeholder="Titre, artiste… sur YouTube et YouTube Music"
        bind:value={search.query}
        autofocus
      />
    </form>
  </header>

  {#if search.loading}
    <p class="status">Recherche en cours…</p>
  {:else if search.lastQuery && search.results.length === 0}
    <p class="status">Aucun résultat pour « {search.lastQuery} ».</p>
  {:else if !search.lastQuery}
    <p class="status">Appuie sur Entrée pour chercher sur YouTube et YouTube Music.</p>
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
    <div class="results" class:stale={search.loading} class:in-group={group.isActive}>
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
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 1.5rem;
    margin-bottom: 1.5rem;
  }

  form {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 0.6rem;
    max-width: 420px;
    padding: 0.6rem 1rem;
    border: 1px solid transparent;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--muted);
  }

  form:focus-within {
    border-color: var(--muted);
  }

  input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 0.9rem;
    outline: none;
  }

  .more {
    display: flex;
    justify-content: center;
    padding: 1.5rem 0 0.5rem;
  }

  .more button {
    padding: 0.55rem 1.4rem;
    border: 1px solid var(--muted);
    border-radius: 999px;
    background: transparent;
    color: var(--text);
    font-size: 0.85rem;
    font-weight: 700;
  }

  .more button:not(:disabled):hover {
    border-color: var(--text);
    transform: scale(1.04);
  }

  .discoveries {
    margin-top: 2rem;
  }

  .status {
    margin: 0 0 1rem;
    color: var(--muted);
  }

  .results.stale {
    opacity: 0.5;
  }

  .row {
    display: grid;
    grid-template-columns: 2.5rem minmax(0, 3fr) minmax(0, 2fr) 4rem 2.5rem;
    align-items: center;
    gap: 1rem;
    padding: 0.4rem 0.75rem;
    border-radius: 4px;
  }

  .in-group .row {
    grid-template-columns: 2.5rem minmax(0, 3fr) minmax(0, 2fr) 4rem 4.5rem;
  }

  .head {
    margin-bottom: 0.5rem;
    border-bottom: 1px solid var(--surface-2);
    border-radius: 0;
    color: var(--muted);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
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
    width: 40px;
    height: 40px;
    border-radius: 4px;
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
