import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  commands,
  events,
  type SyncProgress,
  type SyncReport,
  type SyncSettings,
} from "#lib/bindings.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

export type SyncService = "youtube_music" | "spotify";

class Sync {
  /** `null` until loaded. */
  settings = $state<SyncSettings | null>(null);
  /** The service being synced (only one at a time). */
  running = $state<SyncService | null>(null);
  progress = $state<SyncProgress | null>(null);
  /** Result of the last run of each service in this session. */
  reports = $state<Partial<Record<SyncService, SyncReport>>>({});

  async setBrowser(browser: string) {
    const settings = await attempt(() => commands.setYtmusicBrowser(browser));
    if (settings) this.settings = settings;
  }

  syncYoutubeMusic() {
    return this.run("youtube_music", () => commands.syncYoutubeMusic());
  }

  /** `paths`: the Spotify export `.zip`, or JSON files extracted from it. */
  importSpotify(paths: string[]) {
    return this.run("spotify", () => commands.importSpotify(paths));
  }

  private async run(service: SyncService, start: () => Promise<SyncReport>) {
    this.running = service;
    this.progress = null;
    const report = await attempt(start);
    this.running = null;
    this.progress = null;
    if (report) {
      this.reports[service] = report;
      // Refresh the last sync date (and the followed artists).
      const settings = await attempt(() => commands.getSyncSettings());
      if (settings) this.settings = settings;
    }
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
