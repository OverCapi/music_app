import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type SearchSource = "music" | "youtube";

export interface SearchResult {
  id: string;
  title: string;
  url: string;
  thumbnail: string;
  /** Artist (YouTube Music) or channel (YouTube). */
  artist: string | null;
  album: string | null;
  duration: number | null;
  views: number | null;
  /** True until the `search-details` event for this result arrives (YouTube Music). */
  details_pending: boolean;
}

interface SearchResponse {
  search_id: number;
  results: SearchResult[];
  has_more: boolean;
}

interface SearchDetails {
  search_id: number;
  id: string;
  artist: string | null;
  album: string | null;
  duration: number | null;
}

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
  error = $state<string | null>(null);

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
    this.error = null;
    try {
      const res = await invoke<SearchResponse>("search_youtube", {
        query,
        source,
        offset: 0,
        searchId: null,
      });
      if (request !== this.#request) return;
      this.#searchId = res.search_id;
      this.results = dedupe(res.results);
      this.hasMore = res.has_more;
      this.lastQuery = query;
      this.lastSource = source;
    } catch (e) {
      if (request !== this.#request) return;
      this.error = String(e);
      this.results = [];
      this.hasMore = false;
    } finally {
      if (request === this.#request) this.loading = false;
    }
  }

  async loadMore() {
    if (this.loading || this.loadingMore || !this.hasMore) return;
    const request = this.#request;
    this.loadingMore = true;
    this.error = null;
    try {
      const res = await invoke<SearchResponse>("search_youtube", {
        query: this.lastQuery,
        source: this.lastSource,
        offset: this.results.length,
        searchId: this.#searchId,
      });
      if (request !== this.#request) return;
      this.results = dedupe([...this.results, ...res.results]);
      this.hasMore = res.has_more;
    } catch (e) {
      if (request !== this.#request) return;
      this.error = String(e);
    } finally {
      if (request === this.#request) this.loadingMore = false;
    }
  }

  setSource(source: SearchSource) {
    if (source === this.source) return;
    this.source = source;
    if (this.query.trim()) this.search();
  }

  /** Subscribes to detail events; returns the unsubscribe function. */
  listen(): Promise<UnlistenFn> {
    return listen<SearchDetails>("search-details", ({ payload }) => {
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
  const seen = new Set<string>();
  return results.filter((r) => !seen.has(r.id) && seen.add(r.id));
}

export const search = new Search();
