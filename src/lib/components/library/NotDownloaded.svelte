<script lang="ts">
  import { downloads } from "#lib/stores/downloads.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import ProgressRing from "#lib/components/ui/ProgressRing.svelte";

  /** Marker for a library track without a file; clicking downloads it now. */
  let { youtubeId }: { youtubeId: string | null } = $props();

  const download = $derived(youtubeId ? downloads.get(youtubeId) : undefined);
  const active = $derived(
    download?.status === "queued" ||
      download?.status === "downloading" ||
      download?.status === "converting",
  );
</script>

{#if active}
  <span class="marker" title="Téléchargement en cours…">
    <ProgressRing progress={download?.status === "downloading" ? download.progress : null} />
  </span>
{:else if youtubeId}
  <button
    class="marker"
    class:error={download?.status === "error"}
    onclick={(e) => {
      e.stopPropagation();
      downloads.start(youtubeId);
    }}
    ondblclick={(e) => e.stopPropagation()}
    title={download?.status === "error"
      ? `${download.error ?? "Erreur"} — cliquer pour réessayer`
      : "Pas encore téléchargé : il le sera à l'écoute. Cliquer pour le télécharger maintenant"}
    aria-label="Télécharger maintenant"
  >
    <Icon name="download" size={14} />
  </button>
{:else}
  <span class="marker" title="Aucune vidéo YouTube associée pour l'instant">
    <Icon name="download" size={14} />
  </span>
{/if}

<style>
  .marker {
    display: inline-grid;
    flex: none;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    background: transparent;
    color: var(--muted);
  }

  .marker :global(svg) {
    width: 14px;
    height: 14px;
  }

  button.marker:hover {
    color: var(--text);
  }

  .error {
    color: var(--danger);
  }
</style>
