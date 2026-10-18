import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  commands,
  events,
  type SyncProgress,
  type SyncReport,
  type SpotifyImportReport,
  type SyncSettings,
} from "#lib/bindings.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

class Sync {
  /** `null` until loaded. */
  settings = $state<SyncSettings | null>(null);
  running = $state(false);
  progress = $state<SyncProgress | null>(null);
  /** Result of the last sync run in this session. */
  report = $state<SyncReport | null>(null);

  async setBrowser(browser: string) {
    const settings = await attempt(() => commands.setYtmusicBrowser(browser));
    if (settings) this.settings = settings;
  }

  async syncYoutubeMusic() {
    this.running = true;
    this.progress = null;
    const report = await attempt(() => commands.syncYoutubeMusic());
    this.running = false;
    this.progress = null;
    if (report) {
      this.report = report;
      // Refresh the last sync date.
      const settings = await attempt(() => commands.getSyncSettings());
      if (settings) this.settings = settings;
    }
  }

  spotifyReport = $state<SpotifyImportReport | null>(null);

  async importSpotify(path: string) {
    this.running = true;
    this.progress = null;
    const report = await attempt(() => commands.importSpotifyTsv(path));
    this.running = false;
    this.progress = null;
    if (report) this.spotifyReport = report;
  }

  async init(): Promise<UnlistenFn> {
    const unlisten = await events.syncProgress.listen(({ payload }) => {
      if (this.running) this.progress = payload;
    });
    this.settings = await commands.getSyncSettings();
    return unlisten;
  }
}

export const sync = new Sync();
