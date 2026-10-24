<script lang="ts">
  import { lan } from "#lib/stores/lan.svelte.ts";
  import { plural } from "#lib/format.ts";
  import Field from "#lib/components/ui/Field.svelte";
  import ProgressBar from "#lib/components/ui/ProgressBar.svelte";
  import StackCard from "#lib/components/ui/StackCard.svelte";
  import PeerRow from "#lib/components/lan/PeerRow.svelte";

  // Follows the store (the name arrives after it initialized); editable until saved.
  let name = $derived(lan.deviceName);

  const progress = $derived(
    lan.progress && lan.progress.total > 0 ? lan.progress.done / lan.progress.total : null,
  );
</script>

<StackCard>
  <div class="top">
    <div class="what">
      <span class="title">Appareils à proximité</span>
      <span class="muted">
        Synchronise ta bibliothèque avec un autre appareil du même réseau où l'app est ouverte
      </span>
    </div>
  </div>
  <div class="device">
    <span class="muted">Nom de cet appareil</span>
    <Field
      variant="grow"
      bind:value={name}
      maxlength="50"
      aria-label="Nom de cet appareil"
      onchange={async () => {
        if (!(await lan.setDeviceName(name))) name = lan.deviceName;
      }}
    />
  </div>

  {#each lan.peers as peer (peer.id)}
    <PeerRow {peer} />
  {:else}
    <p class="muted small">Aucun autre appareil trouvé pour l'instant…</p>
  {/each}

  {#if lan.syncing || lan.notice || lan.done}
    <div class="status">
      {#if lan.progress}
        <ProgressBar value={progress} />
        <span class="muted small">
          {lan.progress.step}{lan.progress.total > 0
            ? ` (${lan.progress.done + 1}/${lan.progress.total})`
            : ""}
        </span>
      {:else if lan.requesting}
        <ProgressBar value={null} />
        <span class="muted small">En attente de l'autre appareil…</span>
      {/if}
      {#if lan.notice}<p class="report">{lan.notice}</p>{/if}
      {#if lan.done && !lan.syncing}
        {@const d = lan.done}
        {#if d.error}
          <p class="report error">{d.device} : {d.error}</p>
        {:else if d.report}
          {@const r = d.report}
          <p class="report">
            Synchronisé avec {d.device} : {plural(r.new_tracks, "titre")} ajouté{r.new_tracks > 1
              ? "s"
              : ""}{r.files > 0 ? ` dont ${plural(r.files, "fichier")} copié` : ""}, {plural(
              r.playlists,
              "playlist",
            )}.
            {#if r.removed_tracks + r.removed_playlists > 0}
              Supprimé : {plural(r.removed_tracks, "titre")}, {plural(
                r.removed_playlists,
                "playlist",
              )}.
            {/if}
            {#if r.skipped > 0}
              {plural(r.skipped, "titre")} ignoré{r.skipped > 1 ? "s" : ""} (ni YouTube ni fichier).
            {/if}
          </p>
          {#each r.errors as error (error)}
            <p class="report error">{error}</p>
          {/each}
        {/if}
      {/if}
    </div>
  {/if}
</StackCard>

<style>
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .what {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }
  .title {
    font-size: 16px;
    font-weight: 700;
  }
  .device {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    font-size: 14px;
  }
  .status {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .small {
    margin: 0;
    font-size: 13px;
  }
  .report {
    margin: 0;
    color: var(--mute);
    font-size: 13px;
  }
  .report.error {
    color: var(--danger);
  }
</style>
