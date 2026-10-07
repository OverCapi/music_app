export type View =
  | { kind: "library" }
  | { kind: "search" }
  | { kind: "group" }
  | { kind: "playlist"; id: string };

class Navigation {
  view = $state<View>({ kind: "library" });

  go(view: View) {
    this.view = view;
  }

  isPlaylist(id: string): boolean {
    return this.view.kind === "playlist" && this.view.id === id;
  }
}

export const nav = new Navigation();
