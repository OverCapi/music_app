export function formatTime(secs: number): string {
  if (!Number.isFinite(secs) || secs < 0) secs = 0;
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

const viewsFormat = new Intl.NumberFormat("fr-FR", { notation: "compact", maximumFractionDigits: 1 });

export function formatViews(views: number): string {
  return `${viewsFormat.format(views)} vues`;
}
