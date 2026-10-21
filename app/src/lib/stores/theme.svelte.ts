/** Halo theme: dark/light mode + accent hue (OKLCH). Replaces the 5 named themes.
 * Applied before first paint by the inline script in `src/app.html` (same keys). */
export const ACCENTS = [65, 300, 190, 20, 145] as const;
export type Mode = "dark" | "light";

const MODE_KEY = "theme-mode";
const ACCENT_KEY = "theme-accent";

function read<T>(key: string, ok: (v: string) => boolean, fallback: T): T {
  try {
    const v = localStorage.getItem(key);
    if (v !== null && ok(v)) return v as T;
  } catch {
    // Storage unavailable.
  }
  return fallback;
}

class Theme {
  mode = $state<Mode>(read(MODE_KEY, (v) => v === "dark" || v === "light", "dark"));
  accent = $state<number>(
    Number(read(ACCENT_KEY, (v) => ACCENTS.includes(Number(v) as never), "65")),
  );

  setMode(mode: Mode) {
    this.mode = mode;
    document.documentElement.dataset.mode = mode;
    this.#save(MODE_KEY, mode);
  }

  setAccent(hue: number) {
    this.accent = hue;
    document.documentElement.style.setProperty("--accent-h", String(hue));
    this.#save(ACCENT_KEY, String(hue));
  }

  #save(key: string, value: string) {
    try {
      localStorage.setItem(key, value);
    } catch {
      // Not persisted.
    }
  }
}

export const theme = new Theme();

/* app.html — replace the theme script with:
  <script>
    try {
      const m = localStorage.getItem("theme-mode");
      if (m) document.documentElement.dataset.mode = m;
      const a = localStorage.getItem("theme-accent");
      if (a) document.documentElement.style.setProperty("--accent-h", a);
    } catch {}
  </script>
  and load the fonts:
  https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap
*/
