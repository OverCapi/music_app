import { SvelteSet } from "svelte/reactivity";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  commands,
  events,
  type StorageReport,
  type StorageSettings,
  type StorageTrack,
} from "#lib/bindings.ts";
import { formatSize } from "#lib/format.ts";
import { attempt, toasts } from "#lib/stores/toasts.svelte.ts";

class Storage {
  /** `null` until loaded. */
  settings = $state<StorageSettings | null>(null);
  tracks = $state<StorageTrack[]>([]);
  /** Ids ticked in the list. */
  selected = new SvelteSet<number>();
  busy = $state(false);

  async load() {
    const tracks = await attempt(() => commands.listStorageTracks());
    if (!tracks) return;
    this.tracks = tracks;
    this.selected.clear();
    for (const t of tracks) if (t.expired) this.selected.add(t.track_id);
  }

  async save(settings: StorageSettings) {
    const saved = await attempt(() => commands.setStorageSettings(settings));
    if (!saved) return;
    this.settings = saved;
    await this.load();
  }

  toggle(id: number) {
    if (!this.selected.delete(id)) this.selected.add(id);
  }

  async optimize() {
    await this.#run(() => commands.optimizeStorage());
  }

  async deleteSelected() {
    const ids = [...this.selected];
    await this.#run(() => commands.deleteTrackFiles(ids));
  }

  async #run(action: () => Promise<StorageReport>) {
    this.busy = true;
    const report = await attempt(action);
    this.busy = false;
    if (report?.deleted === 0) toasts.push("Aucun fichier à supprimer");
    await this.load();
  }

  async init(): Promise<UnlistenFn> {
    // Background runs (at startup) and manual ones both end with this event.
    const unlisten = await events.storageOptimized.listen(({ payload }) => {
      const { deleted, bytes_freed } = payload;
      const sons = `${deleted} son${deleted > 1 ? "s" : ""} supprimé${deleted > 1 ? "s" : ""}`;
      toasts.push(`${sons} · ${formatSize(bytes_freed)} libérés`);
    });
    this.settings = await commands.getStorageSettings();
    return unlisten;
  }
}

export const storage = new Storage();
