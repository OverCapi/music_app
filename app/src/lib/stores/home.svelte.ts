import { commands, type Discovery, type Mix, type Play } from "#lib/bindings.ts";
import { player } from "#lib/stores/player.svelte.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

const RECENT_LIMIT = 20;

/**
 * Home page data. Unlike the other stores there is no `init()`: generating the mixes
 * runs several yt-dlp calls, so everything is loaded lazily the first time a page
 * needs it and cached for the session.
 */
class Home {
  mixes = $state<Mix[]>([]);
  discoveries = $state<Discovery | null>(null);
  recent = $state<Play[]>([]);
  loadingMixes = $state(false);
  loadingDiscoveries = $state(false);
  /** Mixes generated (even if empty) at least once this session. */
  mixesLoaded = $state(false);
  discoveriesLoaded = $state(false);

  /** Loads mixes and discoveries once (or again with `force`); never throws. */
  load(force = false): Promise<unknown> {
    return Promise.all([this.loadMixes(force), this.loadDiscoveries(force)]);
  }

  async loadMixes(force = false) {
    if (this.loadingMixes || (this.mixesLoaded && !force)) return;
    this.loadingMixes = true;
    const mixes = await attempt(() => commands.getMixes());
    if (mixes) {
      this.mixes = mixes;
      this.mixesLoaded = true;
    }
    this.loadingMixes = false;
  }

  async loadDiscoveries(force = false) {
    if (this.loadingDiscoveries || (this.discoveriesLoaded && !force)) return;
    this.loadingDiscoveries = true;
    const discoveries = await attempt(() => commands.getDiscoveries());
    if (discoveries) {
      this.discoveries = discoveries;
      this.discoveriesLoaded = true;
    }
    this.loadingDiscoveries = false;
  }

  /** Discards this week's discoveries and generates new ones. */
  async refreshDiscoveries() {
    if (this.loadingDiscoveries) return;
    this.loadingDiscoveries = true;
    const discoveries = await attempt(() => commands.refreshDiscoveries());
    if (discoveries) {
      this.discoveries = discoveries;
      this.discoveriesLoaded = true;
    }
    this.loadingDiscoveries = false;
  }

  /** Cheap (local database): reloaded every time the home page mounts. */
  async loadRecent() {
    const recent = await attempt(() => commands.getRecentPlays(RECENT_LIMIT));
    if (recent) this.recent = recent;
  }

  /** Plays `tracks` from `index`. */
  play(
    tracks: { youtube_id: string; title: string; artist: string | null; duration: number | null }[],
    index: number,
  ) {
    return attempt(() => commands.playSuggestions(
        tracks.map((t) => ({ ...t, duration: t.duration ?? 0 })),
        index,
      ));
  }

  /** Plays the whole list; with shuffle on, from a random track (like a playlist). */
  playAll(tracks: Parameters<Home["play"]>[0]) {
    if (tracks.length === 0) return;
    return this.play(tracks, player.state.shuffle ? Math.floor(Math.random() * tracks.length) : 0);
  }
}

export const home = new Home();
