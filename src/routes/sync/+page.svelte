<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type { SyncReport } from "#lib/bindings.ts";
  import { sync, type SyncService } from "#lib/stores/sync.svelte.ts";
  import { attempt } from "#lib/stores/toasts.svelte.ts";
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
  const busy = $derived(sync.running !== null);
  const lastSync = $derived(formatDate(settings?.ytmusic_synced_at ?? null));
  const lastImport = $derived(formatDate(settings?.spotify_imported_at ?? null));

  function formatDate(secs: number | null) {
    return secs
      ? new Date(secs * 1000).toLocaleString("fr-FR", { dateStyle: "long", timeStyle: "short" })
      : null;
  }

  function followed(n: number) {
    return `${plural(n, "artiste")} suivi${n > 1 ? "s" : ""}`;
  }

  async function importSpotify() {
    const paths = await attempt(() =>
      open({
        multiple: true,
        filters: [{ name: "Export Spotify", extensions: ["zip", "json"] }],
      }),
    );
    if (paths && paths.length > 0) await sync.importSpotify(paths);
  }
</script>

{#snippet status(service: SyncService, last: string | null, lastLabel: string)}
  {#if sync.running === service && sync.progress}
    <span class="muted">
      {sync.progress.step}{sync.progress.total > 0
        ? ` (${sync.progress.done + 1}/${sync.progress.total})`
        : ""}
    </span>
  {:else if last}
    <span class="muted">{lastLabel} : {last}</span>
  {/if}
{/snippet}

{#snippet report(r: SyncReport | undefined, service: SyncService)}
  {#if r && sync.running !== service}
    <p class="report">
      {plural(r.playlists, "playlist")} importée{r.playlists > 1 ? "s" : ""}, {plural(
        r.tracks,
        "titre",
      )}
      dont {r.new_tracks} nouveau{r.new_tracks > 1 ? "x" : ""} dans la bibliothèque{r.artists > 0
        ? `, ${followed(r.artists)}`
        : ""}.
    </p>
    {#if r.errors.length > 0}
      <ul class="errors">
        {#each r.errors as error (error)}
          <li class="error">{error}</li>
        {/each}
      </ul>
    {/if}
  {/if}
{/snippet}

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
          disabled={busy}
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
          disabled={!settings.ytmusic_browser || busy}
          onclick={() => sync.syncYoutubeMusic()}
        >
          {sync.running === "youtube_music" ? "Synchronisation…" : "Synchroniser"}
        </button>
        {@render status("youtube_music", lastSync, "Dernière synchronisation")}
      </div>

      {@render report(sync.reports.youtube_music, "youtube_music")}
    {:else}
      <p class="muted">Chargement…</p>
    {/if}
  </div>

  <div class="card">
    <h2>Spotify</h2>
    <p class="muted">
      Sans Spotify Premium, l'app ne peut pas lire ta bibliothèque directement : elle importe
      l'export de tes données. Demande-le sur spotify.com, dans Compte → Confidentialité → «
      Télécharger vos données », en cochant « Données du compte ». Spotify l'envoie par e-mail sous
      quelques jours.
    </p>
    <p class="muted">
      Choisis ensuite le fichier <code>.zip</code> reçu (ou <code>YourLibrary.json</code> et les
      <code>Playlist1.json</code>… qu'il contient). Les titres déjà présents, par exemple venant de
      YouTube Music, sont réutilisés.
    </p>

    <div class="row">
      <button class="pill primary" disabled={busy} onclick={importSpotify}>
        {sync.running === "spotify" ? "Import…" : "Importer l'export"}
      </button>
      {@render status("spotify", lastImport, "Dernier import")}
    </div>
    {@render report(sync.reports.spotify, "spotify")}
    {#if settings && settings.spotify_artists.length > 0}
      <details class="artists">
        <summary class="muted">
          {followed(settings.spotify_artists.length)}
        </summary>
        <p>{settings.spotify_artists.join(", ")}</p>
      </details>
    {/if}
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

  .artists {
    margin-top: 1rem;
    font-size: 0.85rem;
  }

  .artists summary {
    cursor: pointer;
  }

  .errors {
    margin: 0.5rem 0 0;
    padding-left: 1.2rem;
    font-size: 0.85rem;
  }
</style>
