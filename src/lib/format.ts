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

/** `850 Ko`, `12,4 Mo`, `1,2 Go`. */
export function formatSize(bytes: number): string {
  const units = ["o", "Ko", "Mo", "Go"];
  let i = 0;
  while (bytes >= 1024 && i < units.length - 1) {
    bytes /= 1024;
    i++;
  }
  return `${i === 0 ? bytes : bytes.toLocaleString("fr-FR", { maximumFractionDigits: 1 })} ${units[i]}`;
}
