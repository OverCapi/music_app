/** Shared guard for drag-to-reorder lists (`Sidebar` playlists, `QueueView`): a swap happens
 * only once the pointer crosses the target's middle, and not within 90 ms of the last swap
 * (no jitter while rows animate). */
export class ReorderGuard {
  #lock = 0;

  get locked(): boolean {
    return performance.now() < this.#lock;
  }

  /** Call right after a swap. */
  arm() {
    this.#lock = performance.now() + 90;
  }

  /** The dragged row moves from index `from` to the row under the pointer at index `to`. */
  shouldSwap(e: DragEvent, from: number, to: number): boolean {
    if (this.locked) return false;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const middle = r.top + r.height / 2;
    return from < to ? e.clientY >= middle : e.clientY <= middle;
  }
}
