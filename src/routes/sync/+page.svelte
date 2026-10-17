<script lang="ts">
  import { sync } from "#lib/stores/sync.svelte.ts";
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
    <h1>Synchronisation</h1>
    <p class="subtitle">
      Importe tes titres likés et tes playlists. Les titres sont ajoutés sans être téléchargés : ils
      le seront au moment de l'écoute. Un titre déjà présent n'est jamais ajouté deux fois.
    </p>
  </header>

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
    margin-bottom: 1.5rem;
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
