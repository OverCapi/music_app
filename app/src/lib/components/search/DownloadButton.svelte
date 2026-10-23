<script lang="ts">
  /** Download state of a YouTube video: arrow → progress ring (click cancels) → check. */
  import { downloads } from "#lib/stores/downloads.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import IconButton from "#lib/components/ui/IconButton.svelte";
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
  <IconButton
    label="Annuler le téléchargement"
    size={34}
    onclick={() => downloads.cancel(id)}
    title={download.status === "queued"
      ? "En attente — cliquer pour annuler"
      : download.status === "converting"
        ? "Conversion… — cliquer pour annuler"
        : `Téléchargement ${Math.round(progress * 100)} % — cliquer pour annuler`}
  >
    <ProgressRing progress={download.status === "downloading" ? progress : null} stop />
  </IconButton>
{:else if download?.status === "error"}
  <IconButton
    class="error"
    icon="errorCircle"
    iconSize={20}
    label="Réessayer le téléchargement"
    size={34}
    onclick={() => downloads.start(id)}
    title={`${download.error ?? "Erreur"} — cliquer pour réessayer`}
  />
{:else}
  <IconButton
    icon="download"
    label="Télécharger"
    size={34}
    onclick={() => downloads.start(id)}
    title="Télécharger dans la bibliothèque"
  />
{/if}

<style>
  .state {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    color: var(--accent);
    animation: pop 0.45s var(--ease-spring);
  }
  :global(.halo-btn.error) {
    color: var(--danger);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.5) rotate(-30deg);
    }
  }
</style>
