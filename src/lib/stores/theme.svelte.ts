/** Predefined color themes; their colors live in `src/app.css` under `[data-theme="<id>"]`. */
export const THEMES = [
  { id: "nuit", label: "Nuit" },
  { id: "aube", label: "Aube" },
  { id: "braise", label: "Braise" },
  { id: "ocean", label: "Océan" },
  { id: "rose", label: "Rose" },
] as const;

export type ThemeId = (typeof THEMES)[number]["id"];

// Also read by the inline script in `src/app.html`, which applies the theme before first paint.
const KEY = "theme";

function stored(): ThemeId {
  try {
    const id = localStorage.getItem(KEY);
    if (THEMES.some((t) => t.id === id)) return id as ThemeId;
  } catch {
    // Storage unavailable: keep the default.
  }
  return "nuit";
}

class Theme {
  current = $state<ThemeId>(stored());

  set(id: ThemeId) {
    this.current = id;
    document.documentElement.dataset.theme = id;
    try {
      localStorage.setItem(KEY, id);
    } catch {
      // Not persisted; the theme still applies for this session.
    }
  }
}

export const theme = new Theme();
