import { convertFileSrc } from "@tauri-apps/api/core";

/** YouTube thumbnail of a video id. */
export const youtubeCover = (id: string) => `https://i.ytimg.com/vi/${id}/mqdefault.jpg`;

/** Cover of a library track: embedded art (`cover` protocol), or the YouTube thumbnail while not downloaded. */
export const trackCover = (t: { id: number; path: string | null; youtube_id: string | null }) =>
  !t.path && t.youtube_id ? youtubeCover(t.youtube_id) : localCover(t.id);

/** Embedded art of a library track via the `cover` protocol. */
export const localCover = (trackId: number) => convertFileSrc(String(trackId), "cover");
