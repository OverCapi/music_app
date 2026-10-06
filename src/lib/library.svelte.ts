import { invoke } from "@tauri-apps/api/core";

export interface Track {
  path: string;
  title: string;
  artist: string | null;
  album: string | null;
  duration: number;
  added_at: number;
  /** Set for tracks downloaded from YouTube / YouTube Music. */
  youtube_id: string | null;
}

class Library {
  tracks = $state<Track[]>([]);
  error = $state<string | null>(null);
  adding = $state(false);

  async refresh() {
    try {
      this.tracks = await invoke<Track[]>("get_library");
    } catch (e) {
      this.error = String(e);
    }
  }

  async add(paths: string[]) {
    this.error = null;
    this.adding = true;
    try {
      this.tracks = await invoke<Track[]>("add_tracks", { paths });
    } catch (e) {
      // Some files may still have been added: resync the list.
      this.error = String(e);
      await this.refresh();
    } finally {
      this.adding = false;
    }
  }

  async remove(path: string) {
    this.error = null;
    try {
      this.tracks = await invoke<Track[]>("remove_track", { path });
    } catch (e) {
      this.error = String(e);
    }
  }

  hasYoutubeId(id: string): boolean {
    return this.tracks.some((t) => t.youtube_id === id);
  }

  find(path: string | null): Track | undefined {
    return path ? this.tracks.find((t) => t.path === path) : undefined;
  }
}

export const library = new Library();
