<script lang="ts">
  import { account } from "#lib/stores/account.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";

  let invite = $state("");

  function onsubmit(e: SubmitEvent) {
    e.preventDefault();
    if (invite.trim()) group.join(invite);
  }
</script>

<p class="who">
  Connecté en tant que <strong>{account.username}</strong>
  <button class="link" onclick={() => account.logout()} disabled={account.busy}>
    Se déconnecter
  </button>
</p>

<div class="cards">
  <div class="card">
    <h2>Nouveau groupe</h2>
    <p class="muted">Crée un groupe et partage le code avec tes amis.</p>
    <button class="pill primary" onclick={() => group.create()} disabled={group.busy}>
      Créer un groupe
    </button>
  </div>

  <form class="card" {onsubmit}>
    <h2>Rejoindre un groupe</h2>
    <p class="muted">Colle le code (ex. ABC123) ou le lien d'invitation.</p>
    <div class="join">
      <input
        class="field"
        bind:value={invite}
        placeholder="Code ou musicapp://join/…"
        spellcheck="false"
        aria-label="Code ou lien d'invitation"
      />
      <button class="pill secondary" type="submit" disabled={!invite.trim() || group.busy}>
        Rejoindre
      </button>
    </div>
  </form>
</div>

<style>
  .who {
    margin: 0 0 1.25rem;
    color: var(--muted);
    font-size: 0.9rem;
  }

  strong {
    color: var(--text);
  }

  .link {
    margin-left: 0.75rem;
    padding: 0;
    background: transparent;
    color: var(--muted);
    font-size: 0.85rem;
    text-decoration: underline;
  }

  .link:not(:disabled):hover {
    color: var(--text);
  }

  .cards {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
  }

  .card {
    display: flex;
    flex: 1 1 280px;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.75rem;
    max-width: 400px;
  }

  .muted {
    margin: 0;
    font-size: 0.85rem;
  }

  .join {
    display: flex;
    gap: 0.6rem;
    width: 100%;
  }

  .field {
    flex: 1;
    min-width: 0;
  }
</style>
