import { invoke } from "@tauri-apps/api/core";

export interface Playlist {
  id: string;
  name: string;
  /** Track paths, in play order. */
  tracks: string[];
  created_at: number;
}

class Playlists {
  items = $state<Playlist[]>([]);
  error = $state<string | null>(null);

  find(id: string): Playlist | undefined {
    return this.items.find((p) => p.id === id);
  }

  async refresh() {
    await this.#run(async () => {
      this.items = await invoke<Playlist[]>("get_playlists");
    });
  }

  /** Returns the new playlist, or undefined on error. */
  async create(name: string): Promise<Playlist | undefined> {
    let created: Playlist | undefined;
    await this.#run(async () => {
      const res = await invoke<{ playlist: Playlist; playlists: Playlist[] }>(
        "create_playlist",
        { name },
      );
      this.items = res.playlists;
      created = res.playlist;
    });
    return created;
  }

  async rename(id: string, name: string) {
    await this.#run(async () => {
      this.items = await invoke<Playlist[]>("rename_playlist", { id, name });
    });
  }

  async delete(id: string) {
    await this.#run(async () => {
      this.items = await invoke<Playlist[]>("delete_playlist", { id });
    });
  }

  async addTracks(id: string, paths: string[]) {
    await this.#run(async () => {
      this.items = await invoke<Playlist[]>("add_to_playlist", { id, paths });
    });
  }

  async removeTrack(id: string, path: string) {
    await this.#run(async () => {
      this.items = await invoke<Playlist[]>("remove_from_playlist", { id, path });
    });
  }

  async #run(action: () => Promise<void>) {
    this.error = null;
    try {
      await action();
    } catch (e) {
      this.error = String(e);
    }
  }
}

export const playlists = new Playlists();
