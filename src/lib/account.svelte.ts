import { invoke } from "@tauri-apps/api/core";

export interface Account {
  server_url: string;
  /** Null when logged out. */
  username: string | null;
}

class AccountStore {
  /** Null until the first `get_account` answer. */
  account = $state<Account | null>(null);
  error = $state<string | null>(null);
  busy = $state(false);

  username = $derived(this.account?.username ?? null);
  loggedIn = $derived(this.username !== null);

  refresh(): Promise<boolean> {
    return this.#run(() => invoke<Account>("get_account"));
  }

  setServerUrl(url: string): Promise<boolean> {
    return this.#run(() => invoke<Account>("set_server_url", { url }));
  }

  register(username: string, password: string): Promise<boolean> {
    return this.#run(() => invoke<Account>("register", { username, password }));
  }

  login(username: string, password: string): Promise<boolean> {
    return this.#run(() => invoke<Account>("login", { username, password }));
  }

  logout(): Promise<boolean> {
    return this.#run(() => invoke<Account>("logout"));
  }

  /** Returns true on success. */
  async #run(action: () => Promise<Account>): Promise<boolean> {
    this.error = null;
    this.busy = true;
    try {
      this.account = await action();
      return true;
    } catch (e) {
      this.error = String(e);
      return false;
    } finally {
      this.busy = false;
    }
  }
}

export const account = new AccountStore();
