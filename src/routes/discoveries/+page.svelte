<script lang="ts">
  import { onMount } from "svelte";
  import { home } from "#lib/stores/home.svelte.ts";
  import { formatTotal, plural } from "#lib/format.ts";
  import TrackCollection from "#lib/components/ui/TrackCollection.svelte";
  import SuggestionTable from "#lib/components/home/SuggestionTable.svelte";

  const tracks = $derived(home.discoveries?.tracks ?? []);
  const totalDuration = $derived(tracks.reduce((sum, t) => sum + (t.duration ?? 0), 0));

  onMount(() => {
    home.loadDiscoveries();
  });
</script>

<section class="view">
  {#if tracks.length === 0 && (home.loadingDiscoveries || !home.discoveriesLoaded)}
    <p class="muted">Préparation de tes découvertes…</p>
  {:else}
    <TrackCollection
      kind="Découvertes"
      name="Découvertes de la semaine"
      meta={plural(tracks.length, "titre") +
        (totalDuration > 0 ? ` · ${formatTotal(totalDuration)}` : "")}
      icon="note"
      playLabel="Lire les découvertes"
      playDisabled={tracks.length === 0}
      onplay={() => home.playAll(tracks)}
    >
      {#snippet toolbar()}
        <button
          class="pill secondary"
          onclick={() => home.refreshDiscoveries()}
          disabled={home.loadingDiscoveries}
        >
          {home.loadingDiscoveries ? "Génération…" : "Régénérer"}
        </button>
      {/snippet}

      {#if tracks.length === 0}
        <div class="empty">
          <p>Écoute quelques titres pour que je te propose des découvertes.</p>
        </div>
      {:else}
        <SuggestionTable {tracks} onplay={(i) => home.play(tracks, i)} />
      {/if}
    </TrackCollection>
  {/if}
</section>
