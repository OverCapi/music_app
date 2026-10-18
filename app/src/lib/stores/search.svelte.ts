import type { UnlistenFn } from "@tauri-apps/api/event";
import { commands, events, type SearchResult } from "#lib/bindings.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

class Search {
  query = $state("");
  /** Query of the displayed results. */
  lastQuery = $state("");
  results = $state<SearchResult[]>([]);
  hasMore = $state(false);
  loading = $state(false);
  loadingMore = $state(false);

  #searchId: number | null = null;
  #nextOffset = 0;
  /** Incremented per new search, so a slow older request can't overwrite a newer one. */
  #request = 0;

  async search() {
    const query = this.query.trim();
    if (!query) return;
    const request = ++this.#request;
    this.loading = true;
    this.loadingMore = false;
    const res = await attempt(() => commands.searchYoutube(query, 0, null));
    if (request !== this.#request) return;
    this.loading = false;
    if (!res) {
      this.results = [];
      this.hasMore = false;
      return;
    }
    this.#searchId = res.search_id;
    this.#nextOffset = res.next_offset;
    this.results = dedupe(res.results);
    this.hasMore = res.has_more;
    this.lastQuery = query;
  }

  async loadMore() {
    if (this.loading || this.loadingMore || !this.hasMore) return;
    const request = this.#request;
    this.loadingMore = true;
    const res = await attempt(() =>
      commands.searchYoutube(this.lastQuery, this.#nextOffset, this.#searchId),
    );
    if (request !== this.#request) return;
    this.loadingMore = false;
    if (!res) return;
    this.#nextOffset = res.next_offset;
    this.results = dedupe([...this.results, ...res.results]);
    this.hasMore = res.has_more;
  }

  /** Background details of YouTube Music results. */
  init(): Promise<UnlistenFn> {
    return events.searchDetails.listen(({ payload }) => {
      if (payload.search_id !== this.#searchId) return;
      const result = this.results.find((r) => r.id === payload.id);
      if (!result) return;
      result.artist = payload.artist;
      result.album = payload.album;
      result.duration = payload.duration;
      result.details_pending = false;
    });
  }
}

/** Pages can overlap; keep the first occurrence of each video. */
function dedupe(results: SearchResult[]): SearchResult[] {
  // ponytail: O(n²), fine for a few hundred results.
  return results.filter((r, i) => results.findIndex((other) => other.id === r.id) === i);
}

export const search = new Search();
