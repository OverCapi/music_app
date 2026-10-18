<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { storage } from "#lib/stores/storage.svelte.ts";
  import { formatSize } from "#lib/format.ts";
  import { sync } from "#lib/stores/sync.svelte.ts";
  import { THEMES, theme } from "#lib/stores/theme.svelte.ts";
  import { plural } from "#lib/format.ts";

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

  async function pickSpotify() {
    const path = await open({ filters: [{ name: "TSV", extensions: ["tsv"] }] });
    if (typeof path === "string") await sync.importSpotify(path);
  }

  const freeable = $derived(
    storage.tracks.filter((t) => storage.selected.has(t.track_id)).reduce((n, t) => n + t.size, 0),
  );

  const settings = $derived(sync.settings);
  const lastSync = $derived(
    settings?.ytmusic_synced_at
      ? new Date(settings.ytmusic_synced_at * 1000).toLocaleString("fr-FR", {
          dateStyle: "long",
          timeStyle: "short",
        })
      : null,
  );
</script>

<section class="view">
  <header>
    <h1>Paramètres</h1>
  </header>

  <h2 class="section">Thème</h2>
  <div class="themes" role="radiogroup" aria-label="Thème de couleur">
    {#each THEMES as t (t.id)}
      <button
        class="theme"
        class:selected={theme.current === t.id}
        role="radio"
        aria-checked={theme.current === t.id}
        onclick={() => theme.set(t.id)}
      >
        <!-- The preview carries the theme's own variables. -->
        <span class="preview" data-theme={t.id} aria-hidden="true">
          <span class="surface"><span class="a"></span><span class="b"></span></span>
          <span class="line"></span>
          <span class="line short"></span>
        </span>
        {t.label}
      </button>
    {/each}
  </div>

  <h2 class="section">Synchronisation</h2>
  <div class="sync-intro">
    <p class="subtitle">
      Importe tes titres likés et tes playlists. Les titres sont ajoutés sans être téléchargés : ils
      le seront au moment de l'écoute. Un titre déjà présent n'est jamais ajouté deux fois.
    </p>
  </div>

  <div class="card">
    <h2>YouTube Music</h2>
    <p class="muted">
      L'app lit ta bibliothèque avec la connexion YouTube de ton navigateur : il suffit d'y être
      connecté à YouTube Music. Les cookies ne sont jamais enregistrés par l'app.
    </p>

    {#if settings}
      <label class="row">
        <span>Navigateur</span>
        <select
          class="field"
          value={settings.ytmusic_browser ?? ""}
          disabled={sync.running}
          onchange={(e) => sync.setBrowser(e.currentTarget.value)}
        >
          {#if !settings.ytmusic_browser}
            <option value="" disabled>Choisir…</option>
          {/if}
          {#each settings.browsers as browser (browser)}
            <option value={browser}>{browserNames[browser] ?? browser}</option>
          {/each}
        </select>
      </label>

      <div class="row">
        <button
          class="pill primary"
          disabled={!settings.ytmusic_browser || sync.running}
          onclick={() => sync.syncYoutubeMusic()}
        >
          {sync.running ? "Synchronisation…" : "Synchroniser"}
        </button>
        {#if sync.running && sync.progress}
          <span class="muted">
            {sync.progress.step}{sync.progress.total > 0
              ? ` (${sync.progress.done + 1}/${sync.progress.total})`
              : ""}
          </span>
        {:else if lastSync}
          <span class="muted">Dernière synchronisation : {lastSync}</span>
        {/if}
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
        {#if r.errors.length > 0}
          <ul class="errors">
            {#each r.errors as error (error)}
              <li class="error">{error}</li>
            {/each}
          </ul>
        {/if}
      {/if}
    {:else}
      <p class="muted">Chargement…</p>
    {/if}
  </div>

  <div class="card">
    <h2>Spotify</h2>
    <p class="muted">
      Importe une liste de titres (fichier .tsv : n, titre, artiste, durée). Les titres déjà dans la
      bibliothèque sont ignorés, les autres sont cherchés sur YouTube. Ceux qui ne sont pas trouvés
      sont écrits dans un fichier « _non_trouves.tsv » à côté de la liste.
    </p>
    <div class="row">
      <button class="pill primary" disabled={sync.running} onclick={pickSpotify}>
        {sync.running ? "Import…" : "Choisir un fichier"}
      </button>
      {#if sync.running && sync.progress}
        <span class="muted">{sync.progress.step} ({sync.progress.done}/{sync.progress.total})</span>
      {/if}
    </div>
    {#if sync.spotifyReport && !sync.running}
      {@const r = sync.spotifyReport}
      <p class="report">
        {plural(r.total, "titre")} : {r.already_owned} déjà présent{r.already_owned > 1 ? "s" : ""},
        {r.added} ajouté{r.added > 1 ? "s" : ""}, {r.not_found} introuvable{r.not_found > 1
          ? "s"
          : ""}.
        {#if r.not_found_path}Liste : {r.not_found_path}{/if}
      </p>
    {/if}
  </div>

  <h2 class="section">Stockage</h2>
  <div class="card">
    <h2>Optimiseur de stockage</h2>
    <p class="muted">
      Supprime le fichier audio des sons pas écoutés depuis longtemps. Les sons restent dans la
      bibliothèque et sont retéléchargés quand tu les écoutes. Les favoris (★) sont conservés.
    </p>
    {#if storage.settings}
      {@const s = storage.settings}
      <label class="row">
        <input
          type="checkbox"
          checked={s.auto}
          onchange={(e) => storage.save({ ...s, auto: e.currentTarget.checked })}
        />
        <span>Optimiser automatiquement à chaque ouverture de l'app</span>
      </label>
      <label class="row">
        <span>Supprimer après</span>
        <input
          class="field days"
          type="number"
          min="1"
          value={s.days}
          onchange={(e) =>
            storage.save({
              ...s,
              days: Math.max(1, Math.floor(Number(e.currentTarget.value)) || 1),
            })}
        />
        <span>jours sans écoute</span>
      </label>
      <div class="row">
        <button class="pill primary" disabled={storage.busy} onclick={() => storage.optimize()}>
          Optimiser maintenant
        </button>
        <button class="pill" disabled={storage.busy} onclick={() => storage.load()}>
          Afficher les fichiers
        </button>
      </div>
      {#if storage.tracks.length > 0}
        <ul class="files">
          {#each storage.tracks as t (t.track_id)}
            <li>
              <label>
                <input
                  type="checkbox"
                  checked={storage.selected.has(t.track_id)}
                  onchange={() => storage.toggle(t.track_id)}
                />
                <span class="name">{t.title}{t.artist ? ` — ${t.artist}` : ""}</span>
                <span class="muted">
                  {new Date(t.last_played * 1000).toLocaleDateString("fr-FR")} · {formatSize(
                    t.size,
                  )}
                </span>
              </label>
            </li>
          {/each}
        </ul>
        <div class="row">
          <button
            class="pill primary"
            disabled={storage.busy || storage.selected.size === 0}
            onclick={() => storage.deleteSelected()}
          >
            Supprimer la sélection ({formatSize(freeable)})
          </button>
        </div>
      {/if}
    {:else}
      <p class="muted">Chargement…</p>
    {/if}
  </div>
</section>

<style>
  header {
    margin-bottom: 0.5rem;
  }

  .section {
    margin: 2rem 0 1rem;
    font-size: 1.1rem;
    font-weight: 700;
  }

  .sync-intro {
    margin-bottom: 1rem;
  }

  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 1rem;
    max-width: 52rem;
  }

  .theme {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.6rem 0.6rem 0.8rem;
    border: 2px solid transparent;
    border-radius: 16px;
    background: var(--surface);
    color: var(--text);
    font-weight: 700;
    text-align: left;
  }

  .theme:hover {
    background: var(--surface-2);
  }

  .theme.selected {
    border-color: var(--accent);
  }

  .preview {
    display: flex;
    flex-direction: column;
    gap: 6px;
    height: 72px;
    box-sizing: border-box;
    padding: 10px;
    border-radius: 12px;
    background: var(--bg);
  }

  .preview .surface {
    display: flex;
    gap: 6px;
    padding: 6px;
    border-radius: 8px;
    background: var(--surface);
  }

  .preview .a,
  .preview .b {
    width: 14px;
    height: 14px;
    border-radius: 50%;
  }

  .preview .a {
    background: var(--accent);
  }

  .preview .b {
    background: var(--coral);
  }

  .preview .line {
    height: 6px;
    border-radius: 3px;
    background: var(--text);
    opacity: 0.85;
  }

  .preview .line.short {
    width: 60%;
    background: var(--muted);
  }

  .subtitle {
    max-width: 40rem;
    margin: 0.25rem 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .card {
    max-width: 40rem;
    margin-bottom: 1rem;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-top: 1rem;
  }

  select {
    width: auto;
    min-width: 10rem;
  }

  .days {
    width: 5rem;
  }

  .files {
    list-style: none;
    margin: 1rem 0 0;
    padding: 0;
    max-height: 18rem;
    overflow-y: auto;
  }

  .files label {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.3rem 0;
  }

  .files .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .report {
    margin: 1rem 0 0;
  }

  .errors {
    margin: 0.5rem 0 0;
    padding-left: 1.2rem;
    font-size: 0.85rem;
  }
</style>
