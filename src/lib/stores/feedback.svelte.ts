import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  commands,
  events,
  type FeedbackKind,
  type FeedbackState,
  type FeedbackTarget,
} from "#lib/bindings.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

/** Artist key used by the backend. Must match `normalize_artist` in Rust (trim + lowercase). */
export function normalizeArtist(name: string): string {
  return name.trim().toLowerCase();
}

class Feedback {
  state = $state<FeedbackState>({ tracks: [], artists: [] });

  #trackKinds = $derived(new Map(this.state.tracks.map((f) => [f.youtube_id, f.kind])));
  #artistKinds = $derived(new Map(this.state.artists.map((f) => [f.artist, f.kind])));

  trackKind(youtubeId: string): FeedbackKind | undefined {
    return this.#trackKinds.get(youtubeId);
  }

  artistKind(name: string): FeedbackKind | undefined {
    return this.#artistKinds.get(normalizeArtist(name));
  }

  toggleLike(track: { youtube_id: string; title: string }) {
    return this.#toggle(this.#trackTarget(track), this.trackKind(track.youtube_id), "like");
  }

  toggleTrackDislike(track: { youtube_id: string; title: string }) {
    return this.#toggle(this.#trackTarget(track), this.trackKind(track.youtube_id), "dislike");
  }

  toggleArtistDislike(name: string) {
    const target: FeedbackTarget = { type: "artist", name: name.trim() };
    return this.#toggle(target, this.artistKind(name), "dislike");
  }

  #trackTarget(track: { youtube_id: string; title: string }): FeedbackTarget {
    return { type: "track", youtube_id: track.youtube_id, label: track.title };
  }

  /** Same kind already set: clear it; otherwise set it (replacing the other kind). */
  #toggle(target: FeedbackTarget, current: FeedbackKind | undefined, kind: FeedbackKind) {
    return attempt(() =>
      current === kind ? commands.clearFeedback(target) : commands.setFeedback(target, kind),
    );
  }

  /** Clears whichever dislike (track and/or artist) exists. */
  clearDislikes(track: { youtube_id: string; title: string }, artist: string | null | undefined) {
    const ops: Promise<unknown>[] = [];
    if (this.trackKind(track.youtube_id) === "dislike") {
      ops.push(attempt(() => commands.clearFeedback(this.#trackTarget(track))));
    }
    if (artist && this.artistKind(artist) === "dislike") {
      ops.push(attempt(() => commands.clearFeedback({ type: "artist", name: artist.trim() })));
    }
    return Promise.all(ops);
  }

  async init(): Promise<UnlistenFn> {
    const unlisten = await events.feedbackChanged.listen(({ payload }) => {
      this.state = payload;
    });
    const state = await attempt(() => commands.getFeedback());
    if (state) this.state = state;
    return unlisten;
  }
}

export const feedback = new Feedback();
