import type { UnlistenFn } from "@tauri-apps/api/event";
import { commands, events, type Track } from "#lib/bindings.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

class Library {
  tracks = $state<Track[]>([]);
  adding = $state(false);

  byId = $derived(new Map(this.tracks.map((t) => [t.id, t])));
  #youtubeIds = $derived(new Set(this.tracks.map((t) => t.youtube_id)));

  hasYoutubeId(id: string): boolean {
    return this.#youtubeIds.has(id);
  }

  async add(paths: string[]) {
    this.adding = true;
    await attempt(() => commands.addTracks(paths));
    this.adding = false;
  }

  /** Also removes it from the playlists and the queue (backend). */
  remove(id: number) {
    return attempt(() => commands.removeTrack(id));
  }

  async init(): Promise<UnlistenFn> {
    const unlisten = await events.libraryChanged.listen(({ payload }) => {
      this.tracks = payload.tracks;
    });
    const tracks = await attempt(() => commands.getLibrary());
    if (tracks) this.tracks = tracks;
    return unlisten;
  }
}

export const library = new Library();
