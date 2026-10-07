const TOAST_MS = 6000;

interface Toast {
  id: number;
  message: string;
}

/** Error messages shown in the corner (the single place where errors surface). */
class Toasts {
  items = $state<Toast[]>([]);
  #next = 0;

  push(message: string) {
    const id = ++this.#next;
    this.items.push({ id, message });
    setTimeout(() => this.dismiss(id), TOAST_MS);
  }

  dismiss(id: number) {
    this.items = this.items.filter((t) => t.id !== id);
  }
}

export const toasts = new Toasts();

/** Commands throw an `AppError` `{ kind, message }` whose message is already in French. */
export function errorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String(e.message);
  return String(e);
}

/**
 * Runs a command and shows its error as a toast.
 * Resolves to `undefined` on failure (commands without a result resolve to `null` on success).
 */
export async function attempt<T>(action: () => Promise<T>): Promise<T | undefined> {
  try {
    return await action();
  } catch (e) {
    toasts.push(errorMessage(e));
    return undefined;
  }
}
