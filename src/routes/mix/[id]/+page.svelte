<script lang="ts">
  import { onMount } from "svelte";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import { home } from "#lib/stores/home.svelte.ts";
  import { formatTotal, plural } from "#lib/format.ts";
  import TrackCollection from "#lib/components/ui/TrackCollection.svelte";
  import SuggestionTable from "#lib/components/home/SuggestionTable.svelte";

  const mix = $derived(home.mixes.find((m) => m.id === page.params.id));
  const totalDuration = $derived(
    (mix?.tracks ?? []).reduce((sum, t) => sum + (t.duration ?? 0), 0),
  );

  // Mixes are not stored: opening the page directly (or after a reload) regenerates them.
  onMount(() => {
    home.loadMixes();
  });
</script>

<section class="view">
  {#if mix}
    <TrackCollection
      kind="Mix"
      name={mix.title}
      meta={plural(mix.tracks.length, "titre") +
        (totalDuration > 0 ? ` · ${formatTotal(totalDuration)}` : "")}
      icon="note"
      playLabel="Lire le mix"
      playDisabled={mix.tracks.length === 0}
      onplay={() => home.playAll(mix.tracks)}
    >
      <SuggestionTable tracks={mix.tracks} onplay={(i) => home.play(mix.tracks, i)} />
    </TrackCollection>
  {:else if !home.mixesLoaded || home.loadingMixes}
    <p class="muted">Préparation du mix…</p>
  {:else}
    <div class="empty">
      <p>Ce mix n'existe plus : les mix sont recréés à chaque session.</p>
      <p><a class="link" href={resolve("/")}>Retour à l'accueil</a></p>
    </div>
  {/if}
</section>

<style>
  .link {
    color: var(--accent);
    font-weight: 600;
  }

  .link:hover {
    text-decoration: underline;
  }
</style>
