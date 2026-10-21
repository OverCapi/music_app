<script lang="ts">
  import { search } from "#lib/stores/search.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { home } from "#lib/stores/home.svelte.ts";
  import { ui } from "#lib/stores/ui.svelte.ts";
  import { progressive } from "#lib/progressive.svelte.ts";
  import { formatTime, formatViews } from "#lib/format.ts";
  import DownloadButton from "#lib/components/search/DownloadButton.svelte";
  import AddToGroup from "#lib/components/group/AddToGroup.svelte";
  import Button from "#lib/components/ui/Button.svelte";
  import Icon from "#lib/components/ui/Icon.svelte";
  import Section from "#lib/components/ui/Section.svelte";
  import Skeleton from "#lib/components/ui/Skeleton.svelte";
  import TrackList from "#lib/components/ui/TrackList.svelte";
  import TrackRow from "#lib/components/ui/TrackRow.svelte";
  import SuggestionTracks from "#lib/components/tracks/SuggestionTracks.svelte";

  const shown = progressive(() => search.results.length);
  const discoveries = $derived(home.discoveries?.tracks ?? []);
  const idle = $derived(!search.lastQuery && !search.query.trim() && !search.loading);

  // Discoveries are loaded lazily, only once the idle search page needs them.
  $effect(() => {
    if (idle) home.loadDiscoveries();
  });
</script>

<div class="page">
  {#if ui.compact}
    <!-- The top bar has no search field in narrow windows. -->
    <form
      class="compact-search"
      onsubmit={(e) => {
        e.preventDefault();
        search.search();
      }}
    >
      <input
        class="field"
        type="search"
        bind:value={search.query}
        placeholder="Rechercher"
        aria-label="Rechercher"
      />
    </form>
  {/if}
  <h1>{search.lastQuery ? `Résultats pour « ${search.lastQuery} »` : "Rechercher"}</h1>

  {#if idle}
    <Section title="Découvertes de la semaine">
      {#snippet actions()}
        {#if discoveries.length > 0}
          <Button variant="primary" onclick={() => home.playAll(discoveries)}>
            <Icon name="play" size={12} />Lire
          </Button>
        {/if}
      {/snippet}
      {#if discoveries.length > 0}
        <SuggestionTracks tracks={discoveries} onplay={(i) => home.play(discoveries, i)} />
      {:else}
        <p class="status">
          Tape ta recherche dans la barre du haut (<span class="kbd">⌘K</span>) puis Entrée.
        </p>
      {/if}
    </Section>
  {/if}

  {#if search.loading}
    <Skeleton rows={6} />
  {:else if search.lastQuery && search.results.length === 0}
    <p class="status">Aucun résultat pour « {search.lastQuery} ».</p>
  {/if}

  {#if search.results.length > 0 && !search.loading}
    <div class="results">
      <TrackList>
        {#each search.results.slice(0, shown.count) as result, i (result.id)}
          <TrackRow
            index={i}
            cover={result.thumbnail}
            cols="minmax(0,2fr) minmax(0,1.3fr) 48px auto"
            colsSmall="minmax(0,1fr) auto"
          >
            <span class="tr-main">
              <span class="tr-title">{result.title}</span>
              {#if result.details_pending}
                <span class="ph"></span>
              {:else}
                <span class="tr-sub">{result.artist ?? ""}</span>
              {/if}
            </span>
            <span class="tr-sub hide-sm">
              {#if result.details_pending}
                <span class="ph"></span>
              {:else if result.album}
                {result.album}
              {:else if result.views !== null}
                {formatViews(result.views)}
              {/if}
            </span>
            <span class="tr-dur hide-sm">
              {#if !result.details_pending && result.duration !== null}
                {formatTime(result.duration)}
              {/if}
            </span>
            <span class="actions">
              {#if group.isActive}
                <AddToGroup
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
          </TrackRow>
        {/each}
      </TrackList>
      {#if search.hasMore}
        <div class="more">
          <Button onclick={() => search.loadMore()} disabled={search.loadingMore}>
            {search.loadingMore ? "Chargement…" : "Plus de résultats"}
          </Button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 28px;
    padding-top: 20px;
  }
  .page :global(h1) {
    font-size: clamp(30px, 4.4vw, 44px) !important;
    line-height: 1.05 !important;
    text-wrap: pretty;
  }
  .compact-search .field {
    width: 100%;
    border-radius: 999px;
  }
  .status {
    margin: 0;
    color: var(--mute);
    font-size: 14px;
  }
  .results {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .more {
    display: flex;
    justify-content: center;
  }
  .actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 2px;
  }
  .ph {
    display: block;
    width: 55%;
    height: 9px;
    border-radius: 4px;
    background: var(--line);
  }
</style>
