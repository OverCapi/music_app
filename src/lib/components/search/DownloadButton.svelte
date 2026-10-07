<script lang="ts">
  import { downloads } from "#lib/stores/downloads.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import ProgressRing from "#lib/components/ui/ProgressRing.svelte";

  let { id }: { id: string } = $props();

  const download = $derived(downloads.get(id));
  const inLibrary = $derived(library.hasYoutubeId(id));
  const progress = $derived(download?.progress ?? 0);
</script>

{#if inLibrary}
  <span class="state done" title="Dans la bibliothèque"><Icon name="checkCircle" size={22} /></span>
{:else if download?.status === "queued" || download?.status === "downloading" || download?.status === "converting"}
  <button
    class="state"
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
    class="state error"
    onclick={() => downloads.start(id)}
    title={`${download.error ?? "Erreur"} — cliquer pour réessayer`}
    aria-label="Réessayer le téléchargement"
  >
    <Icon name="errorCircle" size={22} />
  </button>
{:else}
  <button
    class="state idle"
    onclick={() => downloads.start(id)}
    title="Télécharger dans la bibliothèque"
    aria-label="Télécharger"
  >
    <Icon name="download" size={22} />
  </button>
{/if}

<style>
  .state {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--muted);
  }

  .idle:hover {
    color: var(--text);
    transform: scale(1.08);
  }

  .done {
    color: var(--accent);
  }

  .error {
    color: var(--danger);
  }
</style>
