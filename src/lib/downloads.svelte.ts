import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { SvelteMap } from "svelte/reactivity";
import { library } from "$lib/library.svelte";

export type DownloadStatus =
  | "queued"
  | "downloading"
  | "converting"
  | "done"
  | "error"
  | "cancelled";

export interface Download {
  status: DownloadStatus;
  /** 0–1 while downloading, when the size is known. */
  progress: number | null;
  error: string | null;
}

interface DownloadProgress extends Download {
  id: string;
}

class Downloads {
  /** Keyed by YouTube video id. */
  items = new SvelteMap<string, Download>();

  get(id: string): Download | undefined {
    return this.items.get(id);
  }

  async start(id: string, url: string) {
    this.items.set(id, { status: "queued", progress: null, error: null });
    try {
      await invoke("download_track", { id, url });
    } catch (e) {
      this.items.set(id, { status: "error", progress: null, error: String(e) });
    }
  }

  async cancel(id: string) {
    await invoke("cancel_download", { id });
  }

  /** Subscribes to download and library events; returns the unsubscribe function. */
  async listen(): Promise<UnlistenFn> {
    const unlisteners = await Promise.all([
      listen<DownloadProgress>("download-progress", ({ payload }) => {
        const { id, ...download } = payload;
        if (download.status === "cancelled") this.items.delete(id);
        else this.items.set(id, download);
      }),
      listen("library-changed", () => library.refresh()),
    ]);
    return () => unlisteners.forEach((fn) => fn());
  }
}

export const downloads = new Downloads();
