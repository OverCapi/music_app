import { commands, type LiveChannel, type LiveStream } from "#lib/bindings.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

/** A pasted YouTube link or an `@channel` is resolved; anything else is a search. */
function isLink(query: string) {
  return query.startsWith("@") || /youtu\.be|youtube\./i.test(query);
}

class Lives {
  query = $state("");
  results = $state<LiveStream[]>([]);
  /** Channels matching the search (plain searches only). */
  channels = $state<LiveChannel[]>([]);
  loading = $state(false);
  /** Query of the displayed results; `null` before the first search. */
  searched = $state<string | null>(null);
  /** Incremented per search, so a slow older request can't overwrite a newer one. */
  #request = 0;

  favorites = $state<LiveChannel[]>([]);
  /** The channel page being shown, with its running lives. */
  channel = $state<LiveChannel | null>(null);
  channelLives = $state<LiveStream[]>([]);
  channelLoading = $state(false);

  async submit() {
    const query = this.query.trim();
    if (!query) return;
    this.channel = null;
    const request = ++this.#request;
    this.loading = true;
    const link = isLink(query);
    const [res, channels] = await Promise.all([
      attempt(() => (link ? commands.resolveLiveUrl(query) : commands.searchLives(query))),
      link ? [] : attempt(() => commands.searchLiveChannels(query)),
    ]);
    if (request !== this.#request) return;
    this.loading = false;
    this.results = res ?? [];
    this.channels = channels ?? [];
    this.searched = query;
  }

  play(stream: LiveStream) {
    return attempt(() => commands.playLive(stream));
  }

  async loadFavorites() {
    this.favorites = (await attempt(() => commands.getFavoriteChannels())) ?? this.favorites;
  }

  isFavorite(id: string) {
    return this.favorites.some((c) => c.id === id);
  }

  async toggleFavorite(channel: LiveChannel) {
    const list = await attempt(() =>
      this.isFavorite(channel.id)
        ? commands.removeFavoriteChannel(channel.id)
        : commands.addFavoriteChannel(channel),
    );
    if (list) this.favorites = list;
  }

  async openChannel(channel: LiveChannel) {
    this.channel = channel;
    this.channelLives = [];
    this.channelLoading = true;
    const lives = await attempt(() => commands.getChannelLives(channel.id));
    if (this.channel?.id !== channel.id) return;
    this.channelLoading = false;
    this.channelLives = lives ?? [];
  }

  /** The live video replaces the page (see `LiveVideo.svelte`). */
  videoOpen = $state(false);
}

export const lives = new Lives();
