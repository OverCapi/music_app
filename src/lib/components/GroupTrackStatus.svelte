<script lang="ts">
  import type { TrackDownload } from "$lib/group.svelte";

  let { download }: { download: TrackDownload | undefined } = $props();

  // Progress ring (same look as DownloadButton): circumference of r=9.
  const CIRCUMFERENCE = 2 * Math.PI * 9;
  const progress = $derived(download?.progress ?? 0);
  const determinate = $derived(download?.status === "downloading" && download.progress !== null);
</script>

{#if !download}
  <span class="state pending" title="Pas encore téléchargé"></span>
{:else if download.status === "ready"}
  <span class="state ready" title="Prêt">
    <svg viewBox="0 0 24 24"><path d="M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20zm-1.4 13.2-3.3-3.3-1.4 1.4 4.7 4.7 7.7-7.7-1.4-1.4z" /></svg>
  </span>
{:else if download.status === "error"}
  <span class="state error" title={download.error ?? "Échec du téléchargement"}>
    <svg viewBox="0 0 24 24"><path d="M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20zm-1 5v7h2V7zm0 9v2h2v-2z" /></svg>
  </span>
{:else}
  <span
    class="state active"
    title={download.status === "queued"
      ? "En attente de téléchargement"
      : determinate
        ? `Téléchargement ${Math.round(progress * 100)} %`
        : "Téléchargement…"}
  >
    <svg viewBox="0 0 24 24" class:spin={!determinate}>
      <circle class="track" cx="12" cy="12" r="9" />
      <circle
        class="bar"
        cx="12"
        cy="12"
        r="9"
        stroke-dasharray={CIRCUMFERENCE}
        stroke-dashoffset={determinate ? CIRCUMFERENCE * (1 - progress) : CIRCUMFERENCE * 0.75}
      />
    </svg>
    {#if determinate}
      <span class="percent">{Math.round(progress * 100)}</span>
    {/if}
  </span>
{/if}

<style>
  .state {
    display: grid;
    flex: none;
    place-items: center;
    width: 22px;
    height: 22px;
    color: var(--muted);
  }

  .state > * {
    grid-area: 1 / 1;
  }

  svg {
    width: 20px;
    height: 20px;
    fill: currentColor;
  }

  .pending::before {
    content: "";
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--track);
  }

  .ready {
    color: var(--accent);
  }

  .error {
    color: #f15e6c;
  }

  .active svg {
    width: 22px;
    height: 22px;
  }

  .track,
  .bar {
    fill: none;
    stroke-width: 2.5;
  }

  .track {
    stroke: var(--track);
  }

  .bar {
    stroke: var(--accent);
    stroke-linecap: round;
    transform: rotate(-90deg);
    transform-origin: center;
    transition: stroke-dashoffset 0.2s linear;
  }

  .percent {
    color: var(--text);
    font-size: 0.5rem;
    font-variant-numeric: tabular-nums;
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
