import { convertFileSrc } from "@tauri-apps/api/core";

/** YouTube thumbnail of a video id. */
export const youtubeCover = (id: string) => `https://i.ytimg.com/vi/${id}/mqdefault.jpg`;

/** Cover of a library track: embedded art (`cover` protocol), or the YouTube thumbnail while not downloaded. */
export const trackCover = (t: { id: number; path: string | null; youtube_id: string | null }) =>
  !t.path && t.youtube_id ? youtubeCover(t.youtube_id) : convertFileSrc(String(t.id), "cover");
