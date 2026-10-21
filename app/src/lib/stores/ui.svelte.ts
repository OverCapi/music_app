/** Window widths. Under `COLLAPSE` both panels fold once; under `COMPACT` they become drawers. */
const COLLAPSE = 1200;
const COMPACT = 768;

const INITIAL_WIDTH = typeof window === "undefined" ? 1440 : window.innerWidth;

/** Shell UI state shared by the layout, panels, top bar and player. */
class Ui {
  /** Playlists panel (left) and queue panel (right). 264/300 px open, 66 px collapsed. */
  sideOpen = $state(INITIAL_WIDTH >= COLLAPSE);
  queueOpen = $state(INITIAL_WIDTH >= COLLAPSE);
  fullscreen = $state(false);
  /** Last measured window width. */
  width = $state(INITIAL_WIDTH);

  /** Narrow window: panels float over the page as drawers (closed by default). */
  compact = $derived(this.width < COMPACT);
  sideWidth = $derived(this.compact ? Math.min(264, this.width - 32) : this.sideOpen ? 264 : 66);
  queueWidth = $derived(this.compact ? Math.min(300, this.width - 32) : this.queueOpen ? 300 : 66);
  /** Left / right offsets of the page, top bar and player dock. */
  left = $derived(this.compact ? 0 : 16 + this.sideWidth);
  right = $derived(this.compact ? 0 : 16 + this.queueWidth);
  /** Hide the volume slider when the dock gets narrower than this. */
  showVolume = $derived(this.width - this.left - this.right > 760);
  /** Whether each panel shows its full content (drawers always do when visible). */
  sideExpanded = $derived(this.compact || this.sideOpen);
  queueExpanded = $derived(this.compact || this.queueOpen);
  /** A drawer is open over the page (compact mode). */
  drawerOpen = $derived(this.compact && (this.sideOpen || this.queueOpen));

  onResize(w: number) {
    const was = this.width;
    this.width = w;
    if (w < COMPACT && was >= COMPACT) {
      this.sideOpen = false;
      this.queueOpen = false;
    } else if (w < COLLAPSE && was >= COLLAPSE) {
      this.sideOpen = false;
      this.queueOpen = false;
    } else if (w >= COMPACT && was < COMPACT && w < COLLAPSE) {
      // Back to the panel layout: keep them folded.
      this.sideOpen = false;
      this.queueOpen = false;
    }
  }

  closeDrawers() {
    this.sideOpen = false;
    this.queueOpen = false;
  }
}

export const ui = new Ui();
