<script lang="ts">
  import type { TrackDownload } from "#lib/bindings.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import ProgressRing from "#lib/components/ui/ProgressRing.svelte";

  let { download }: { download: TrackDownload | undefined } = $props();

  const progress = $derived(download?.status === "downloading" ? download.progress : null);
</script>

{#if !download}
  <span class="state pending" title="Pas encore téléchargé"></span>
{:else if download.status === "ready"}
  <span class="state ready" title="Prêt"><Icon name="checkCircle" /></span>
{:else if download.status === "error"}
  <span class="state error" title={download.error ?? "Échec du téléchargement"}>
    <Icon name="errorCircle" />
  </span>
{:else}
  <span
    class="state"
    title={download.status === "queued"
      ? "En attente de téléchargement"
      : progress !== null
        ? `Téléchargement ${Math.round(progress * 100)} %`
        : "Téléchargement…"}
  >
    <ProgressRing {progress} />
    {#if progress !== null}
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

  .state > :global(*) {
    grid-area: 1 / 1;
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
    color: var(--danger);
  }

  .percent {
    color: var(--text);
    font-size: 0.5rem;
    font-variant-numeric: tabular-nums;
  }
</style>
