import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// Mirrors protocol/src/lib.rs. Times are server clock ms, positions are seconds.

export interface QueueEntry {
  entry_id: number;
  youtube_id: string;
  title: string;
  artist: string | null;
  duration: number | null;
  added_by: string;
}

export interface Member {
  username: string;
  /** Has downloaded the current track. */
  ready: boolean;
}

export type Playback =
  | { status: "idle" }
  | { status: "preparing"; deadline_ms: number }
  | { status: "playing"; position: number; at_ms: number }
  | { status: "paused"; position: number };

export interface GroupState {
  code: string;
  members: Member[];
  queue: QueueEntry[];
  /** Index in `queue` of the current track. */
  current: number | null;
  playback: Playback;
}

/** Local download of a group track (temp folder, not the library). */
export interface TrackDownload {
  status: "queued" | "downloading" | "ready" | "error";
  /** 0–1 while downloading, when the size is known. */
  progress: number | null;
  error: string | null;
}

export interface GroupView {
  code: string;
  link: string;
  /** Our username. */
  me: string;
  /** False while the connection to the server is being re-established. */
  connected: boolean;
  state: GroupState;
  /** Keyed by YouTube video id. */
  downloads: Record<string, TrackDownload>;
}

export interface NewEntry {
  youtubeId: string;
  title: string;
  artist: string | null;
  duration: number | null;
}

class Group {
  view = $state<GroupView | null>(null);
  error = $state<string | null>(null);
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
        return p.position + Math.max(0, now - p.at_ms) / 1000;
      case "paused":
        return p.position;
      default:
        return 0;
    }
  }

  async refresh() {
    try {
      this.view = await invoke<GroupView | null>("get_group");
    } catch (e) {
      this.error = String(e);
    }
  }

  create() {
    return this.#join(() => invoke<GroupView>("group_create"));
  }

  join(invite: string) {
    return this.#join(() => invoke<GroupView>("group_join", { invite: invite.trim() }));
  }

  async leave() {
    this.busy = true;
    const ok = await this.#run(() => invoke("group_leave"));
    if (ok) this.view = null;
    this.busy = false;
  }

  add(entry: NewEntry) {
    return this.#run(() => invoke("group_add", { ...entry }));
  }

  remove(entryId: number) {
    return this.#run(() => invoke("group_remove", { entryId }));
  }

  playEntry(entryId: number) {
    return this.#run(() => invoke("group_play_entry", { entryId }));
  }

  play() {
    return this.#run(() => invoke("group_play"));
  }

  pause() {
    return this.#run(() => invoke("group_pause"));
  }

  /** Play/pause button: pauses an active track, otherwise resumes or starts the queue. */
  togglePlay() {
    const status = this.playback.status;
    if (status === "playing" || status === "preparing") return this.pause();
    if (status === "idle") {
      const entry = this.currentEntry ?? this.state?.queue[0];
      return entry ? this.playEntry(entry.entry_id) : Promise.resolve(false);
    }
    return this.play();
  }

  seek(position: number) {
    return this.#run(() => invoke("group_seek", { position }));
  }

  skip() {
    return this.#run(() => invoke("group_skip"));
  }

  /** Subscribes to group events; returns the unsubscribe function. */
  async listen(): Promise<UnlistenFn> {
    const unlisteners = await Promise.all([
      listen<GroupView | null>("group-state", ({ payload }) => {
        this.view = payload;
      }),
      listen<string>("group-error", ({ payload }) => {
        this.error = payload;
      }),
    ]);
    return () => unlisteners.forEach((fn) => fn());
  }

  async #join(action: () => Promise<GroupView>): Promise<boolean> {
    this.busy = true;
    const ok = await this.#run(async () => {
      this.view = await action();
    });
    this.busy = false;
    return ok;
  }

  /** Returns true on success. */
  async #run(action: () => Promise<unknown>): Promise<boolean> {
    this.error = null;
    try {
      await action();
      return true;
    } catch (e) {
      this.error = String(e);
      return false;
    }
  }
}

export const group = new Group();
