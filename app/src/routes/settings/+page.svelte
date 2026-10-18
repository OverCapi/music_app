<script lang="ts">
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
    <p class="muted">Bientôt : import de l'export de données Spotify.</p>
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

  .report {
    margin: 1rem 0 0;
  }

  .errors {
    margin: 0.5rem 0 0;
    padding-left: 1.2rem;
    font-size: 0.85rem;
  }
</style>
