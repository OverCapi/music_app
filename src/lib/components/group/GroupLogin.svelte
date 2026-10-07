<script lang="ts">
  import { account } from "#lib/stores/account.svelte.ts";

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
      class="field"
      bind:value={serverUrl}
      oninput={() => (touchedUrl = true)}
      placeholder="https://exemple.fr"
      autocomplete="url"
      spellcheck="false"
    />
  </label>
  <label>
    <span>Pseudo</span>
    <input
      class="field"
      bind:value={username}
      autocomplete="username"
      spellcheck="false"
      maxlength="32"
    />
  </label>
  <label>
    <span>Mot de passe</span>
    <input class="field" type="password" bind:value={password} autocomplete="current-password" />
  </label>

  <div class="buttons">
    <button class="pill primary" type="submit" disabled={!valid || account.busy}>
      {account.busy ? "Connexion…" : "Se connecter"}
    </button>
    <button
      class="pill secondary"
      type="button"
      onclick={() => submit("register")}
      disabled={!valid || account.busy}
    >
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
  }

  h2 {
    font-size: 1.2rem;
  }

  .muted {
    margin: 0;
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

  .buttons {
    display: flex;
    gap: 0.75rem;
    margin-top: 0.25rem;
  }
</style>
