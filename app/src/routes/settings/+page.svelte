<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { storage } from "#lib/stores/storage.svelte.ts";
  import { sync } from "#lib/stores/sync.svelte.ts";
  import { ACCENTS, theme, type Mode } from "#lib/stores/theme.svelte.ts";
  import { progressive } from "#lib/progressive.svelte.ts";
  import { formatSize, plural } from "#lib/format.ts";
  import { liquid } from "#lib/motion.ts";
  import Button from "#lib/components/ui/Button.svelte";
  import Checkbox from "#lib/components/ui/Checkbox.svelte";
  import Chip from "#lib/components/ui/Chip.svelte";
  import Panel from "#lib/components/ui/Panel.svelte";
  import ProgressBar from "#lib/components/ui/ProgressBar.svelte";
  import Section from "#lib/components/ui/Section.svelte";
  import Toggle from "#lib/components/ui/Toggle.svelte";

  const browserNames: Record<string, string> = {
    brave: "Brave",
    chrome: "Chrome",
    chromium: "Chromium",
    edge: "Edge",
    firefox: "Firefox",
    opera: "Opera",
    safari: "Safari",
    vivaldi: "Vivaldi",
    whale: "Whale",
  };

  const modes: { id: Mode; label: string }[] = [
    { id: "dark", label: "Sombre" },
    { id: "light", label: "Clair" },
  ];

  async function pickSpotify() {
    const path = await open({ filters: [{ name: "TSV", extensions: ["tsv"] }] });
    if (typeof path === "string") await sync.importSpotify(path);
  }

  const freeable = $derived(
    storage.tracks.filter((t) => storage.selected.has(t.track_id)).reduce((n, t) => n + t.size, 0),
  );

  let showFiles = $state(false);
  const shownFiles = progressive(() => storage.tracks.length);

  async function toggleFiles() {
    showFiles = !showFiles;
    if (showFiles) await storage.load();
  }

  const settings = $derived(sync.settings);
  const lastSync = $derived(
    settings?.ytmusic_synced_at
      ? new Date(settings.ytmusic_synced_at * 1000).toLocaleString("fr-FR", {
          dateStyle: "long",
          timeStyle: "short",
        })
      : null,
  );
  const syncProgress = $derived(
    sync.running && sync.progress && sync.progress.total > 0
      ? sync.progress.done / sync.progress.total
      : null,
  );
</script>

<div class="page">
  <h1>Paramètres</h1>

  <Section title="Apparence">
    <div class="modes" role="radiogroup" aria-label="Mode">
      {#each modes as m (m.id)}
        <button
          class="glass mode"
          class:selected={theme.mode === m.id}
          type="button"
          role="radio"
          aria-checked={theme.mode === m.id}
          data-liquid
          use:liquid
          onclick={() => theme.setMode(m.id)}
        >
          <span class="preview" class:light={m.id === "light"}>
            <span class="p-bar"></span>
            <span class="p-row"><span class="p-dot"></span><span class="p-line"></span></span>
            <span class="p-line short"></span>
          </span>
          <span class="mode-name">{m.label}</span>
        </button>
      {/each}
    </div>
    <Panel radius={10} class="accent-row">
      <span class="lbl">Accent</span>
      <div class="dots">
        {#each ACCENTS as hue (hue)}
          <button
            class="dot"
            class:selected={theme.accent === hue}
            type="button"
            aria-label={`Accent ${hue}`}
            aria-pressed={theme.accent === hue}
            style:--h={hue}
            use:liquid
            onclick={() => theme.setAccent(hue)}
          ></button>
        {/each}
      </div>
    </Panel>
  </Section>

  <Section title="Synchronisation">
    <Panel radius={12} class="card-s">
      <div class="top">
        <div class="what">
          <span class="name">YouTube Music</span>
          <span class="muted">Titres likés et playlists, via ton navigateur</span>
        </div>
        <Button
          variant="primary"
          disabled={!settings?.ytmusic_browser || sync.running}
          onclick={() => sync.syncYoutubeMusic()}
        >
          {sync.running ? "Synchronisation…" : "Synchroniser"}
        </Button>
      </div>
      {#if settings}
        <div class="chips">
          {#each settings.browsers as browser (browser)}
            <Chip
              selected={settings.ytmusic_browser === browser}
              disabled={sync.running}
              onclick={() => sync.setBrowser(browser)}
            >
              {browserNames[browser] ?? browser}
            </Chip>
          {/each}
        </div>
      {/if}
      <div class="status">
        {#if sync.running}<ProgressBar value={syncProgress} />{/if}
        <span class="muted small">
          {#if sync.running && sync.progress}
            {sync.progress.step}{sync.progress.total > 0
              ? ` (${sync.progress.done + 1}/${sync.progress.total})`
              : ""}
          {:else if lastSync}
            Dernière synchronisation : {lastSync}
          {:else if !settings?.ytmusic_browser}
            Choisis le navigateur où tu es connecté à YouTube Music.
          {/if}
        </span>
      </div>
      {#if sync.report && !sync.running}
        {@const r = sync.report}
        <p class="report">
          {plural(r.playlists, "playlist")} importée{r.playlists > 1 ? "s" : ""}, {plural(
            r.tracks,
            "titre",
          )}
          dont {r.new_tracks} nouveau{r.new_tracks > 1 ? "x" : ""} dans la bibliothèque.
        </p>
        {#each r.errors as error (error)}
          <p class="report error">{error}</p>
        {/each}
      {/if}
    </Panel>

    <Panel radius={12} class="card-s">
      <div class="top">
        <div class="what">
          <span class="name">Spotify</span>
          <span class="muted">Importer une liste .tsv, cherchée sur YouTube</span>
        </div>
        <Button disabled={sync.running} onclick={pickSpotify}>Choisir un fichier</Button>
      </div>
      {#if sync.spotifyReport && !sync.running}
        {@const r = sync.spotifyReport}
        <p class="report">
          {plural(r.total, "titre")} : {r.already_owned} déjà présent{r.already_owned > 1
            ? "s"
            : ""}, {r.added} ajouté{r.added > 1 ? "s" : ""}, {r.not_found} introuvable{r.not_found >
          1
            ? "s"
            : ""}.
          {#if r.not_found_path}Liste : {r.not_found_path}{/if}
        </p>
      {/if}
    </Panel>
  </Section>

  <Section title="Stockage">
    <Panel radius={12} class="card-s">
      {#if storage.settings}
        {@const s = storage.settings}
        <p class="muted small">
          Supprime le fichier audio des sons pas écoutés depuis longtemps. Les sons restent dans la
          bibliothèque et sont retéléchargés à l'écoute. Les favoris (★) sont conservés.
        </p>
        <div class="line">
          <span class="name sm">Optimiser à chaque ouverture</span>
          <Toggle
            checked={s.auto}
            label="Optimiser à chaque ouverture"
            onchange={(auto) => storage.save({ ...s, auto })}
          />
        </div>
        <div class="line days">
          <span class="muted">Supprimer les fichiers après</span>
          <input
            class="field num"
            type="number"
            min="1"
            value={s.days}
            aria-label="Nombre de jours"
            onchange={(e) =>
              storage.save({
                ...s,
                days: Math.max(1, Math.floor(Number(e.currentTarget.value)) || 1),
              })}
          />
          <span class="muted">jours sans écoute</span>
        </div>
        <div class="buttons">
          <Button variant="primary" disabled={storage.busy} onclick={() => storage.optimize()}>
            Optimiser maintenant
          </Button>
          <Button disabled={storage.busy} onclick={toggleFiles}>
            {showFiles ? "Masquer les fichiers" : "Afficher les fichiers"}
          </Button>
        </div>
        {#if showFiles}
          <div class="files">
            {#each storage.tracks.slice(0, shownFiles.count) as t (t.track_id)}
              <div
                class="file"
                role="button"
                tabindex="0"
                onclick={() => storage.toggle(t.track_id)}
                onkeydown={(e) => e.key === "Enter" && storage.toggle(t.track_id)}
              >
                <Checkbox
                  checked={storage.selected.has(t.track_id)}
                  label={t.title}
                  onchange={() => storage.toggle(t.track_id)}
                />
                <span class="fname">{t.title}{t.artist ? ` — ${t.artist}` : ""}</span>
                <span class="fmeta">
                  {new Date(t.last_played * 1000).toLocaleDateString("fr-FR")} · {formatSize(
                    t.size,
                  )}
                </span>
              </div>
            {:else}
              <p class="muted small">Aucun fichier téléchargé.</p>
            {/each}
            {#if storage.tracks.length > 0}
              <div class="del">
                <Button
                  disabled={storage.busy || storage.selected.size === 0}
                  onclick={() => storage.deleteSelected()}
                >
                  Supprimer la sélection ({formatSize(freeable)})
                </Button>
              </div>
            {/if}
          </div>
        {/if}
      {:else}
        <p class="muted">Chargement…</p>
      {/if}
    </Panel>
  </Section>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 40px;
    max-width: 780px;
    padding-top: 20px;
  }
  .small {
    margin: 0;
    font-size: 13px;
  }

  /* Appearance */
  .modes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
  .mode {
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 10px;
    border-radius: calc(var(--rs) + 10px);
    color: var(--fg);
    text-align: left;
    cursor: pointer;
    transition: box-shadow 0.3s;
  }
  .mode.selected {
    box-shadow:
      0 0 0 2px var(--accent),
      0 20px 50px -18px var(--shadow);
  }
  .mode-name {
    padding: 0 4px 2px;
    font-size: 14px;
    font-weight: 700;
  }
  .preview {
    display: flex;
    flex-direction: column;
    gap: 8px;
    box-sizing: border-box;
    height: 96px;
    padding: 12px;
    overflow: hidden;
    border-radius: var(--rs);
    background: oklch(0.13 0.008 250);
  }
  .p-bar {
    width: 70%;
    height: 22px;
    border: 1px solid oklch(1 0 0 / 0.1);
    border-radius: 6px;
    background: oklch(1 0 0 / 0.09);
  }
  .p-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .p-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--accent);
  }
  .p-line {
    width: 50%;
    height: 6px;
    border-radius: 3px;
    background: oklch(0.96 0.004 250);
  }
  .p-line.short {
    width: 35%;
    background: oklch(0.72 0.01 250);
  }
  .preview.light {
    background: oklch(0.955 0.004 250);
  }
  .light .p-bar {
    border-color: #fff;
    background: oklch(1 0 0 / 0.8);
  }
  .light .p-line {
    background: oklch(0.2 0.01 250);
  }
  .light .p-line.short {
    background: oklch(0.47 0.01 250);
  }
  .page :global(.accent-row) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 20px;
  }
  .lbl {
    font-size: 14px;
    font-weight: 600;
  }
  .dots {
    display: flex;
    gap: 12px;
  }
  .dot {
    position: relative;
    overflow: hidden;
    width: 28px;
    height: 28px;
    padding: 0;
    border-radius: 50%;
    background: oklch(0.8 0.13 var(--h));
    transition:
      transform 0.4s var(--ease-spring),
      box-shadow 0.3s;
  }
  .dot:hover {
    transform: scale(1.12);
  }
  .dot.selected {
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px oklch(0.8 0.13 var(--h));
  }

  /* Cards */
  .page :global(.card-s) {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 22px 24px;
  }
  .top {
    display: flex;
    flex-wrap: wrap;
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
  .name {
    font-size: 16px;
    font-weight: 700;
  }
  .name.sm {
    font-size: 14px;
    font-weight: 600;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .status {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .report {
    margin: 0;
    color: var(--mute);
    font-size: 13px;
  }
  .report.error {
    color: var(--danger);
  }

  /* Storage */
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .line.days {
    justify-content: flex-start;
    flex-wrap: wrap;
    gap: 12px;
    font-size: 14px;
  }
  .num {
    width: 76px;
    height: 38px;
    text-align: center;
    font-family: var(--mono);
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .files {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }
  .file {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 6px;
    border-radius: var(--rs);
    cursor: pointer;
    transition: background 0.2s;
  }
  .file:hover {
    background: var(--glass2);
  }
  .fname {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 14px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fmeta {
    color: var(--mute);
    font-family: var(--mono);
    font-size: 12px;
    white-space: nowrap;
  }
  .del {
    display: flex;
    justify-content: flex-end;
    padding-top: 10px;
  }
  @container page (max-width: 520px) {
    .fmeta {
      display: none;
    }
    .modes {
      gap: 10px;
    }
  }
</style>
