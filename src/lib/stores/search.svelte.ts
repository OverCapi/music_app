import type { UnlistenFn } from "@tauri-apps/api/event";
import { commands, events, type SearchResult, type SearchSource } from "#lib/bindings.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

class Search {
  query = $state("");
  source = $state<SearchSource>("music");
  /** Query and source of the displayed results. */
  lastQuery = $state("");
  lastSource = $state<SearchSource>("music");
  results = $state<SearchResult[]>([]);
  hasMore = $state(false);
  loading = $state(false);
  loadingMore = $state(false);

  #searchId: number | null = null;
  /** Incremented per new search, so a slow older request can't overwrite a newer one. */
  #request = 0;

  async search() {
    const query = this.query.trim();
    if (!query) return;
    const source = this.source;
    const request = ++this.#request;
    this.loading = true;
    this.loadingMore = false;
    const res = await attempt(() => commands.searchYoutube(query, source, 0, null));
    if (request !== this.#request) return;
    this.loading = false;
    if (!res) {
      this.results = [];
      this.hasMore = false;
      return;
    }
    this.#searchId = res.search_id;
    this.results = dedupe(res.results);
    this.hasMore = res.has_more;
    this.lastQuery = query;
    this.lastSource = source;
  }

  async loadMore() {
    if (this.loading || this.loadingMore || !this.hasMore) return;
    const request = this.#request;
    this.loadingMore = true;
    const res = await attempt(() =>
      commands.searchYoutube(this.lastQuery, this.lastSource, this.results.length, this.#searchId),
    );
    if (request !== this.#request) return;
    this.loadingMore = false;
    if (!res) return;
    this.results = dedupe([...this.results, ...res.results]);
    this.hasMore = res.has_more;
  }

  setSource(source: SearchSource) {
    if (source === this.source) return;
    this.source = source;
    if (this.query.trim()) this.search();
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
