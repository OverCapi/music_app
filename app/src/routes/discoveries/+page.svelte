<script lang="ts">
  import EmptyState from "#lib/components/ui/EmptyState.svelte";
  import { onMount } from "svelte";
  import { home } from "#lib/stores/home.svelte.ts";
  import { formatTotal, plural } from "#lib/format.ts";
  import { youtubeCover } from "#lib/covers.ts";
  import Button from "#lib/components/ui/Button.svelte";
  import PageHeader from "#lib/components/ui/PageHeader.svelte";
  import SuggestionTracks from "#lib/components/tracks/SuggestionTracks.svelte";

  const tracks = $derived(home.discoveries?.tracks ?? []);
  const totalDuration = $derived(tracks.reduce((sum, t) => sum + (t.duration ?? 0), 0));

  onMount(() => {
    home.loadDiscoveries();
  });
</script>

<div class="page">
  {#if tracks.length === 0 && (home.loadingDiscoveries || !home.discoveriesLoaded)}
    <p class="muted">Préparation de tes découvertes…</p>
  {:else}
    <PageHeader
      kind="Découvertes"
      title="Découvertes de la semaine"
      meta={plural(tracks.length, "titre") +
        (totalDuration > 0 ? ` · ${formatTotal(totalDuration)}` : "")}
      covers={tracks.slice(0, 4).map((t) => youtubeCover(t.youtube_id))}
      playLabel="Lire les découvertes"
      playDisabled={tracks.length === 0}
      onplay={() => home.playAll(tracks)}
    >
      {#snippet extra()}
        <Button onclick={() => home.refreshDiscoveries()} disabled={home.loadingDiscoveries}>
          {home.loadingDiscoveries ? "Génération…" : "Régénérer"}
        </Button>
      {/snippet}
    </PageHeader>

    {#if tracks.length === 0}
      <EmptyState>
        <p>Écoute quelques titres pour que je te propose des découvertes.</p>
      </EmptyState>
    {:else}
      <SuggestionTracks {tracks} onplay={(i) => home.play(tracks, i)} />
    {/if}
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }
</style>
