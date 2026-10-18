/** `m:ss`; specta sends Rust f64 as `number | null`. */
export function formatTime(secs: number | null | undefined): string {
  if (secs == null || !Number.isFinite(secs) || secs < 0) secs = 0;
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

/** Total length of a playlist: `42 min` or `1 h 5 min`. */
export function formatTotal(secs: number): string {
  const minutes = Math.round(secs / 60);
  if (minutes < 60) return `${minutes} min`;
  return `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

const viewsFormat = new Intl.NumberFormat("fr-FR", {
  notation: "compact",
  maximumFractionDigits: 1,
});

export function formatViews(views: number): string {
  return `${viewsFormat.format(views)} vues`;
}

export function plural(n: number, word: string): string {
  return `${n} ${word}${n > 1 ? "s" : ""}`;
}
