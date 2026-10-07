<script lang="ts">
  import { account } from "$lib/account.svelte";

  let serverUrl = $state(account.account?.server_url ?? "");
  let username = $state("");
  let password = $state("");
  let touchedUrl = false;

  // Prefill once the account is loaded, unless the user already typed something.
  $effect(() => {
    const url = account.account?.server_url;
    if (url !== undefined && !touchedUrl) serverUrl = url;
  });

  const valid = $derived(serverUrl.trim() !== "" && username.trim() !== "" && password !== "");

  async function submit(mode: "login" | "register") {
    if (!valid || account.busy) return;
    const url = serverUrl.trim();
    if (url !== account.account?.server_url && !(await account.setServerUrl(url))) return;
    const ok =
      mode === "login"
        ? await account.login(username.trim(), password)
        : await account.register(username.trim(), password);
    if (ok) password = "";
  }

  function onsubmit(e: SubmitEvent) {
    e.preventDefault();
    submit("login");
  }
</script>

<form class="card" {onsubmit}>
  <h2>Connexion</h2>
  <p class="muted">Connecte-toi au serveur pour écouter de la musique avec tes amis.</p>

  <label>
    <span>Serveur</span>
    <input
      bind:value={serverUrl}
      oninput={() => (touchedUrl = true)}
      placeholder="https://exemple.fr"
      autocomplete="url"
      spellcheck="false"
    />
  </label>
  <label>
    <span>Pseudo</span>
    <input bind:value={username} autocomplete="username" spellcheck="false" maxlength="32" />
  </label>
  <label>
    <span>Mot de passe</span>
    <input type="password" bind:value={password} autocomplete="current-password" />
  </label>

  {#if account.error}
    <p class="error">{account.error}</p>
  {/if}

  <div class="buttons">
    <button class="primary" type="submit" disabled={!valid || account.busy}>
      {account.busy ? "Connexion…" : "Se connecter"}
    </button>
    <button class="secondary" type="button" onclick={() => submit("register")} disabled={!valid || account.busy}>
      Créer un compte
    </button>
  </div>
</form>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    max-width: 380px;
    padding: 1.5rem;
    border-radius: 8px;
    background: var(--surface);
  }

  h2 {
    margin: 0;
    font-size: 1.2rem;
  }

  .muted {
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 600;
  }

  input {
    padding: 0.6rem 0.8rem;
    border: 1px solid transparent;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: 0.9rem;
    font-weight: 400;
    outline: none;
    user-select: text;
  }

  input:focus {
    border-color: var(--muted);
  }

  .error {
    margin: 0;
    color: #f15e6c;
    font-size: 0.85rem;
  }

  .buttons {
    display: flex;
    gap: 0.75rem;
    margin-top: 0.25rem;
  }

  .primary,
  .secondary {
    padding: 0.6rem 1.4rem;
    border-radius: 999px;
    font-weight: 700;
  }

  .primary {
    background: var(--accent);
    color: #000;
  }

  .secondary {
    border: 1px solid var(--muted);
    background: transparent;
    color: var(--text);
  }

  .primary:not(:disabled):hover,
  .secondary:not(:disabled):hover {
    transform: scale(1.04);
  }

  .secondary:not(:disabled):hover {
    border-color: var(--text);
  }
</style>
