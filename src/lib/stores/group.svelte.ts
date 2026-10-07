import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  commands,
  events,
  type GroupView,
  type Playback,
  type QueueEntry,
  type TrackDownload,
} from "#lib/bindings.ts";
import { attempt, toasts } from "#lib/stores/toasts.svelte.ts";

// Times are server clock ms, positions are seconds.

export interface NewEntry {
  youtubeId: string;
  title: string;
  artist: string | null;
  duration: number | null;
}

class Group {
  view = $state<GroupView | null>(null);
  /** True while creating / joining / leaving. */
  busy = $state(false);

  isActive = $derived(this.view !== null);
  state = $derived(this.view?.state ?? null);
  playback = $derived<Playback>(this.view?.state.playback ?? { status: "idle" });
  currentEntry = $derived.by((): QueueEntry | undefined => {
    const state = this.view?.state;
    return state && state.current !== null ? state.queue[state.current] : undefined;
  });
  /** Members that haven't downloaded the current track yet. */
  waitingFor = $derived(this.view?.state.members.filter((m) => !m.ready) ?? []);

  download(youtubeId: string): TrackDownload | undefined {
    return this.view?.downloads[youtubeId];
  }

  /**
   * Estimated position of the current track, from the shared playback state.
   * Assumes the local clock is close to the server's (the engine's own position is
   * more accurate once the track is loaded locally).
   */
  positionAt(now: number): number {
    const p = this.playback;
    switch (p.status) {
      case "playing":
        return (p.position ?? 0) + Math.max(0, now - p.at_ms) / 1000;
      case "paused":
        return p.position ?? 0;
      default:
        return 0;
    }
  }

  create() {
    return this.#busy(() => commands.groupCreate());
  }

  join(invite: string) {
    return this.#busy(() => commands.groupJoin(invite.trim()));
  }

  leave() {
    return this.#busy(() => commands.groupLeave());
  }

  /** Resolves to true on success. */
  async add({ youtubeId, title, artist, duration }: NewEntry): Promise<boolean> {
    return (
      (await attempt(() => commands.groupAdd(youtubeId, title, artist, duration))) !== undefined
    );
  }

  remove(entryId: number) {
    return attempt(() => commands.groupRemove(entryId));
  }

  playEntry(entryId: number) {
    return attempt(() => commands.groupPlayEntry(entryId));
  }

  /** Play/pause button: pauses an active track, otherwise resumes or starts the queue. */
  togglePlay() {
    const status = this.playback.status;
    if (status === "playing" || status === "preparing") return attempt(() => commands.groupPause());
    if (status === "paused") return attempt(() => commands.groupPlay());
    const entry = this.currentEntry ?? this.state?.queue[0];
    return entry ? this.playEntry(entry.entry_id) : undefined;
  }

  seek(position: number) {
    return attempt(() => commands.groupSeek(position));
  }

  skip() {
    return attempt(() => commands.groupSkip());
  }

  /** Subscribes to group events, then loads the current group. */
  async init(): Promise<UnlistenFn> {
    const unlisteners = await Promise.all([
      events.groupChanged.listen(({ payload }) => (this.view = payload)),
      events.groupError.listen(({ payload }) => toasts.push(payload.message)),
    ]);
    const view = await attempt(() => commands.getGroup());
    if (view !== undefined) this.view = view;
    return () => unlisteners.forEach((fn) => fn());
  }

  async #busy(action: () => Promise<GroupView | void>) {
    this.busy = true;
    const view = await attempt(action);
    // `groupChanged` brings the same state; set it now so the page switches immediately.
    if (view) this.view = view;
    this.busy = false;
  }
}

export const group = new Group();
