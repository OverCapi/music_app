<script lang="ts">
  import { onMount } from "svelte";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import { home } from "#lib/stores/home.svelte.ts";
  import { formatTotal, plural } from "#lib/format.ts";
  import { youtubeCover } from "#lib/covers.ts";
  import PageHeader from "#lib/components/ui/PageHeader.svelte";
  import SuggestionTracks from "#lib/components/tracks/SuggestionTracks.svelte";

  const mix = $derived(home.mixes.find((m) => m.id === page.params.id));
  const totalDuration = $derived(
    (mix?.tracks ?? []).reduce((sum, t) => sum + (t.duration ?? 0), 0),
  );

  // Mixes are not stored: opening the page directly (or after a reload) regenerates them.
  onMount(() => {
    home.loadMixes();
  });
</script>

<div class="page">
  {#if mix}
    <PageHeader
      kind="Mix"
      title={mix.title}
      meta={plural(mix.tracks.length, "titre") +
        (totalDuration > 0 ? ` · ${formatTotal(totalDuration)}` : "")}
      covers={mix.tracks.slice(0, 4).map((t) => youtubeCover(t.youtube_id))}
      playLabel="Lire le mix"
      playDisabled={mix.tracks.length === 0}
      onplay={() => home.playAll(mix.tracks)}
    />
    <SuggestionTracks tracks={mix.tracks} onplay={(i) => home.play(mix.tracks, i)} />
  {:else if !home.mixesLoaded || home.loadingMixes}
    <p class="muted">Préparation du mix…</p>
  {:else}
    <div class="empty">
      <p>Ce mix n'existe plus : les mix sont recréés à chaque session.</p>
      <p><a class="link" href={resolve("/")}>Retour à l'accueil</a></p>
    </div>
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }
  .empty {
    padding: 3rem 0;
    text-align: center;
  }
  .link {
    color: var(--accent);
    font-weight: 600;
  }
</style>
