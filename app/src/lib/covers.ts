import { convertFileSrc } from "@tauri-apps/api/core";

/** Cover of a library track (served by the app's `cover` protocol). */
export const trackCover = (id: number) => convertFileSrc(String(id), "cover");

/** YouTube thumbnail of a video id. */
export const youtubeCover = (id: string) => `https://i.ytimg.com/vi/${id}/mqdefault.jpg`;
