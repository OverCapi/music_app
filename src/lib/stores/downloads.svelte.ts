import type { UnlistenFn } from "@tauri-apps/api/event";
import { SvelteMap } from "svelte/reactivity";
import { commands, events, type DownloadProgress } from "#lib/bindings.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

export type Download = Omit<DownloadProgress, "id">;

class Downloads {
  /** Keyed by YouTube video id. */
  items = new SvelteMap<string, Download>();

  get(id: string): Download | undefined {
    return this.items.get(id);
  }

  async start(id: string) {
    this.items.set(id, { status: "queued", progress: null, error: null });
    if ((await attempt(() => commands.downloadTrack(id))) === undefined) {
      this.items.set(id, { status: "error", progress: null, error: null });
    }
  }

  cancel(id: string) {
    return attempt(() => commands.cancelDownload(id));
  }

  /** The finished file arrives through `libraryChanged` (library store). */
  init(): Promise<UnlistenFn> {
    return events.downloadProgress.listen(({ payload: { id, ...download } }) => {
      if (download.status === "cancelled") this.items.delete(id);
      else this.items.set(id, download);
    });
  }
}

export const downloads = new Downloads();
