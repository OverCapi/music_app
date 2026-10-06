<script lang="ts">
  import { downloads } from "$lib/downloads.svelte";
  import { library } from "$lib/library.svelte";

  let { id, url }: { id: string; url: string } = $props();

  const download = $derived(downloads.get(id));
  const inLibrary = $derived(library.hasYoutubeId(id));
  const active = $derived(
    download?.status === "queued" ||
      download?.status === "downloading" ||
      download?.status === "converting",
  );

  // Progress ring: circumference of r=9.
  const CIRCUMFERENCE = 2 * Math.PI * 9;
  const progress = $derived(download?.progress ?? 0);
</script>

{#if inLibrary}
  <span class="state done" title="Dans la bibliothèque">
    <svg viewBox="0 0 24 24"><path d="M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20zm-1.4 13.2-3.3-3.3-1.4 1.4 4.7 4.7 7.7-7.7-1.4-1.4z" /></svg>
  </span>
{:else if active && download}
  <button
    class="state active"
    onclick={() => downloads.cancel(id)}
    title={download.status === "queued"
      ? "En attente — cliquer pour annuler"
      : download.status === "converting"
        ? "Conversion… — cliquer pour annuler"
        : `Téléchargement ${Math.round(progress * 100)} % — cliquer pour annuler`}
    aria-label="Annuler le téléchargement"
  >
    <svg viewBox="0 0 24 24" class:spin={download.status !== "downloading"}>
      <circle class="track" cx="12" cy="12" r="9" />
      <circle
        class="bar"
        cx="12"
        cy="12"
        r="9"
        stroke-dasharray={CIRCUMFERENCE}
        stroke-dashoffset={download.status === "downloading"
          ? CIRCUMFERENCE * (1 - progress)
          : CIRCUMFERENCE * 0.75}
      />
      <rect class="stop" x="9" y="9" width="6" height="6" rx="1" />
    </svg>
  </button>
{:else if download?.status === "error"}
  <button
    class="state error"
    onclick={() => downloads.start(id, url)}
    title={`${download.error ?? "Erreur"} — cliquer pour réessayer`}
    aria-label="Réessayer le téléchargement"
  >
    <svg viewBox="0 0 24 24"><path d="M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20zm-1 5v7h2V7zm0 9v2h2v-2z" /></svg>
  </button>
{:else}
  <button
    class="state idle"
    onclick={() => downloads.start(id, url)}
    title="Télécharger dans la bibliothèque"
    aria-label="Télécharger"
  >
    <svg viewBox="0 0 24 24"><path d="M11 4h2v9.2l3.3-3.3 1.4 1.4L12 17l-5.7-5.7 1.4-1.4 3.3 3.3zM5 19h14v2H5z" /></svg>
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

  svg {
    width: 22px;
    height: 22px;
    fill: currentColor;
  }

  .idle:hover {
    color: var(--text);
    transform: scale(1.08);
  }

  .done {
    color: var(--accent);
  }

  .error {
    color: #f15e6c;
  }

  .active .track,
  .active .bar {
    fill: none;
    stroke-width: 2.5;
  }

  .active .track {
    stroke: var(--track);
  }

  .active .bar {
    stroke: var(--accent);
    stroke-linecap: round;
    transform: rotate(-90deg);
    transform-origin: center;
    transition: stroke-dashoffset 0.2s linear;
  }

  .active .stop {
    fill: var(--text);
    opacity: 0;
  }

  .active:hover .stop {
    opacity: 1;
  }

  .spin {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
