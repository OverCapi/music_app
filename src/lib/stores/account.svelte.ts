import { commands, type Account } from "#lib/bindings.ts";
import { attempt } from "#lib/stores/toasts.svelte.ts";

class AccountStore {
  /** `null` until the first `getAccount` answer. */
  account = $state<Account | null>(null);
  /** The first load failed (offers a retry). */
  failed = $state(false);
  busy = $state(false);

  username = $derived(this.account?.username ?? null);
  loggedIn = $derived(this.username !== null);

  async refresh() {
    this.failed = !(await this.#run(() => commands.getAccount()));
  }

  setServerUrl(url: string) {
    return this.#run(() => commands.setServerUrl(url));
  }

  register(username: string, password: string) {
    return this.#run(() => commands.register(username, password));
  }

  login(username: string, password: string) {
    return this.#run(() => commands.login(username, password));
  }

  logout() {
    return this.#run(() => commands.logout());
  }

  /** Resolves to true on success. */
  async #run(action: () => Promise<Account>): Promise<boolean> {
    this.busy = true;
    const account = await attempt(action);
    this.busy = false;
    if (account) this.account = account;
    return account !== undefined;
  }
}

export const account = new AccountStore();
