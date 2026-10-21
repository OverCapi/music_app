<script lang="ts">
  /** Download state of a YouTube video: arrow → progress ring (click cancels) → check. */
  import { downloads } from "#lib/stores/downloads.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { liquid } from "#lib/motion.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import ProgressRing from "#lib/components/ui/ProgressRing.svelte";

  let { id }: { id: string } = $props();

  const download = $derived(downloads.get(id));
  const inLibrary = $derived(library.hasYoutubeId(id));
  const progress = $derived(download?.progress ?? 0);
  const busy = $derived(
    download?.status === "queued" ||
      download?.status === "downloading" ||
      download?.status === "converting",
  );
</script>

{#if inLibrary}
  <span class="state done" title="Dans la bibliothèque"><Icon name="check" size={18} /></span>
{:else if busy && download}
  <button
    class="halo-btn"
    use:liquid
    onclick={() => downloads.cancel(id)}
    title={download.status === "queued"
      ? "En attente — cliquer pour annuler"
      : download.status === "converting"
        ? "Conversion… — cliquer pour annuler"
        : `Téléchargement ${Math.round(progress * 100)} % — cliquer pour annuler`}
    aria-label="Annuler le téléchargement"
  >
    <ProgressRing progress={download.status === "downloading" ? progress : null} stop />
  </button>
{:else if download?.status === "error"}
  <button
    class="halo-btn error"
    use:liquid
    onclick={() => downloads.start(id)}
    title={`${download.error ?? "Erreur"} — cliquer pour réessayer`}
    aria-label="Réessayer le téléchargement"
  >
    <Icon name="errorCircle" size={20} />
  </button>
{:else}
  <button
    class="halo-btn"
    use:liquid
    onclick={() => downloads.start(id)}
    title="Télécharger dans la bibliothèque"
    aria-label="Télécharger"
  >
    <Icon name="download" size={18} />
  </button>
{/if}

<style>
  .halo-btn {
    width: 34px;
    height: 34px;
  }
  .state {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    color: var(--accent);
    animation: pop 0.45s var(--ease-spring);
  }
  .error {
    color: var(--danger);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.5) rotate(-30deg);
    }
  }
</style>
