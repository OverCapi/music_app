import type { UnlistenFn } from "@tauri-apps/api/event";
import { commands, events, type Playlist } from "#lib/bindings.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

class Playlists {
  items = $state<Playlist[]>([]);

  find(id: number): Playlist | undefined {
    return this.items.find((p) => p.id === id);
  }

  /** Resolves to the new playlist, or `undefined` on error. */
  create(name: string) {
    return attempt(() => commands.createPlaylist(name.trim()));
  }

  rename(id: number, name: string) {
    return attempt(() => commands.renamePlaylist(id, name.trim()));
  }

  delete(id: number) {
    return attempt(() => commands.deletePlaylist(id));
  }

  addTracks(id: number, trackIds: number[]) {
    return attempt(() => commands.addToPlaylist(id, trackIds));
  }

  removeTrack(id: number, trackId: number) {
    return attempt(() => commands.removeFromPlaylist(id, trackId));
  }

  async init(): Promise<UnlistenFn> {
    const unlisten = await events.playlistsChanged.listen(({ payload }) => {
      this.items = payload.playlists;
    });
    const items = await attempt(() => commands.getPlaylists());
    if (items) this.items = items;
    return unlisten;
  }
}

export const playlists = new Playlists();
