<script lang="ts">
  import Field from "#lib/components/ui/Field.svelte";
  import { account } from "#lib/stores/account.svelte.ts";
  import Button from "#lib/components/ui/Button.svelte";
  import Panel from "#lib/components/ui/Panel.svelte";

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

<form {onsubmit}>
  <Panel radius={18} pad={32} class="login">
    <div class="head">
      <h1>Groupe d'écoute</h1>
      <span class="muted">Écoutez ensemble, chacun depuis ses fichiers.</span>
    </div>
    <div class="fields">
      <Field
        variant="mono"
        bind:value={serverUrl}
        oninput={() => (touchedUrl = true)}
        placeholder="Serveur"
        aria-label="Serveur"
        autocomplete="url"
        spellcheck="false"
      />
      <Field
        bind:value={username}
        placeholder="Nom d'utilisateur"
        aria-label="Nom d'utilisateur"
        autocomplete="username"
        spellcheck="false"
        maxlength="32"
      />
      <Field
        type="password"
        bind:value={password}
        placeholder="Mot de passe"
        aria-label="Mot de passe"
        autocomplete="current-password"
      />
    </div>
    <div class="buttons">
      <Button variant="primary" size={44} type="submit" disabled={!valid || account.busy}>
        {account.busy ? "Connexion…" : "Se connecter"}
      </Button>
      <Button size={44} onclick={() => submit("register")} disabled={!valid || account.busy}>
        Créer un compte
      </Button>
    </div>
  </Panel>
</form>

<style>
  form {
    width: 100%;
    max-width: 440px;
    margin: 40px auto 0;
  }
  form :global(.login) {
    display: flex;
    flex-direction: column;
    gap: 22px;
    box-sizing: border-box;
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .head :global(h1) {
    font-size: 32px !important;
    letter-spacing: -0.035em !important;
  }
  .head span {
    font-size: 14px;
  }
  .fields {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .buttons :global(.primary) {
    flex: 1;
    justify-content: center;
  }
</style>
