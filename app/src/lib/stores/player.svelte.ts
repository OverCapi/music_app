import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  commands,
  events,
  type PlayerState,
  type PlaySource,
  type RepeatMode,
  type QueueState,
  type Track,
} from "#lib/bindings.ts";
import { attempt, toasts } from "#lib/stores/toasts.svelte.ts";

class Player {
  state = $state<PlayerState>({
    track_id: null,
    path: null,
    playing: false,
    loaded: false,
    loading: false,
    position: 0,
    duration: 0,
    volume: 1,
    current: null,
    error: null,
    group_mode: false,
    shuffle: false,
    repeat: "off",
    live: null,
  });
  queue = $state<QueueState>({ items: [], current: null });
  /** `Date.now()` when `state.position` was received, to interpolate between events. */
  #receivedAt = Date.now();

  duration = $derived(this.state.duration ?? 0);
  volume = $derived(this.state.volume ?? 1);
  /** Tracks after the current one, with their queue index. */
  upcoming = $derived(
    this.queue.items
      .map((item, index) => ({ ...item, index }))
      .slice(this.queue.current === null ? 0 : this.queue.current + 1),
  );

  /** Position at `now` (ms), extrapolated from the last event while playing. */
  positionAt(now: number): number {
    const position = this.state.position ?? 0;
    if (!this.state.playing) return position;
    return position + Math.max(0, now - this.#receivedAt) / 1000;
  }

  /** Replaces the queue with `trackIds` and plays `trackIds[start]`. */
  /** Plays `tracks` from `tracks[start]`. Tracks that can't be played yet (no file
   * and no YouTube video to download) are left out; the start one is always sent,
   * so the backend explains why it can't play. */
  playFrom(tracks: Track[], start: number, source: PlaySource) {
    const first = tracks[start];
    const list = tracks.filter((t) => t === first || t.path !== null || t.youtube_id !== null);
    return attempt(() =>
      commands.playQueue(
        list.map((t) => t.id),
        list.indexOf(first),
        source,
      ),
    );
  }

  enqueue(trackIds: number[], source: PlaySource) {
    return attempt(() => commands.enqueue(trackIds, source));
  }

  playIndex(index: number) {
    return attempt(() => commands.playIndex(index));
  }

  removeFromQueue(index: number) {
    return attempt(() => commands.removeFromQueue(index));
  }

  moveInQueue(from: number, to: number) {
    return attempt(() => commands.moveInQueue(from, to));
  }

  clearQueue() {
    return attempt(() => commands.clearQueue());
  }

  togglePlay() {
    return attempt(() => (this.state.playing ? commands.pause() : commands.play()));
  }

  next() {
    return attempt(() => commands.next());
  }

  previous() {
    return attempt(() => commands.previous());
  }

  stop() {
    return attempt(() => commands.stop());
  }

  toggleShuffle() {
    return attempt(() => commands.setShuffle(!this.state.shuffle));
  }

  /** Off → whole queue → current track → off. */
  cycleRepeat() {
    const order: RepeatMode[] = ["off", "all", "one"];
    const next = order[(order.indexOf(this.state.repeat) + 1) % order.length];
    return attempt(() => commands.setRepeat(next));
  }

  seek(secs: number) {
    this.#setState({ ...this.state, position: secs });
    return attempt(() => commands.seek(secs));
  }

  setVolume(volume: number) {
    this.state.volume = volume;
    return attempt(() => commands.setVolume(volume));
  }

  /** Subscribes to engine events, then loads the current state. */
  async init(): Promise<UnlistenFn> {
    const unlisteners = await Promise.all([
      events.playerChanged.listen(({ payload }) => this.#setState(payload)),
      events.queueChanged.listen(({ payload }) => (this.queue = payload)),
    ]);
    const [state, queue] = await Promise.all([
      attempt(() => commands.getPlayerState()),
      attempt(() => commands.getQueue()),
    ]);
    if (state) this.#setState(state);
    if (queue) this.queue = queue;
    return () => unlisteners.forEach((fn) => fn());
  }

  #setState(state: PlayerState) {
    if (state.error && state.error !== this.state.error) toasts.push(state.error);
    this.state = state;
    this.#receivedAt = Date.now();
  }
}

export const player = new Player();
