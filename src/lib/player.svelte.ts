import { invoke } from "@tauri-apps/api/core";

export interface PlayerState {
  path: string | null;
  playing: boolean;
  loaded: boolean;
  position: number;
  duration: number;
  volume: number;
  /** Index of the current track in the queue. */
  current: number | null;
  /** Last playback error reported by the audio engine. */
  error: string | null;
}

interface QueueState {
  tracks: string[];
  current: number | null;
}

const POLL_MS = 250;

class Player {
  state = $state<PlayerState>({
    path: null,
    playing: false,
    loaded: false,
    position: 0,
    duration: 0,
    volume: 1,
    current: null,
    error: null,
  });
  /** Paths in play order. The current index comes from `state.current` (kept fresh by polling). */
  queue = $state<string[]>([]);
  error = $state<string | null>(null);

  #timer: ReturnType<typeof setInterval> | null = null;

  /** Tracks after the current one, with their queue index. */
  upcoming = $derived(
    this.queue
      .map((path, index) => ({ path, index }))
      .slice(this.state.current === null ? 0 : this.state.current + 1),
  );

  /** Replaces the queue with `paths` and plays `paths[start]`. */
  async playFrom(paths: string[], start: number) {
    await this.#run(async () => {
      this.#setQueue(await invoke<QueueState>("play_queue", { tracks: paths, start }));
    });
  }

  async enqueue(paths: string[]) {
    await this.#run(async () => {
      this.#setQueue(await invoke<QueueState>("enqueue", { tracks: paths }));
    });
  }

  async playIndex(index: number) {
    await this.#run(() => invoke("play_index", { index }));
  }

  async removeFromQueue(index: number) {
    await this.#run(async () => {
      this.#setQueue(await invoke<QueueState>("remove_from_queue", { index }));
    });
  }

  async clearQueue() {
    await this.#run(async () => {
      this.#setQueue(await invoke<QueueState>("clear_queue"));
    });
  }

  async togglePlay() {
    await this.#run(() => invoke(this.state.playing ? "pause" : "play"));
  }

  async next() {
    await this.#run(() => invoke("next"));
  }

  async previous() {
    await this.#run(() => invoke("previous"));
  }

  async stop() {
    await this.#run(() => invoke("stop"));
  }

  async seek(secs: number) {
    this.state.position = secs;
    await this.#run(() => invoke("seek", { secs }));
  }

  async setVolume(volume: number) {
    this.state.volume = volume;
    await this.#run(() => invoke("set_volume", { volume }));
  }

  async init() {
    await this.#run(async () => {
      this.#setQueue(await invoke<QueueState>("get_queue"));
    });
    this.startPolling();
  }

  startPolling() {
    this.stopPolling();
    this.#timer = setInterval(() => this.refresh(), POLL_MS);
  }

  stopPolling() {
    if (this.#timer) clearInterval(this.#timer);
    this.#timer = null;
  }

  async refresh() {
    try {
      this.state = await invoke<PlayerState>("get_state");
    } catch (e) {
      this.error = String(e);
    }
  }

  #setQueue(queue: QueueState) {
    this.queue = queue.tracks;
    this.state.current = queue.current;
  }

  async #run(action: () => Promise<unknown>) {
    this.error = null;
    try {
      await action();
    } catch (e) {
      this.error = String(e);
    }
    await this.refresh();
  }
}

export const player = new Player();
