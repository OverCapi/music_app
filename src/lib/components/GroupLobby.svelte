<script lang="ts">
  import { account } from "$lib/account.svelte";
  import { group } from "$lib/group.svelte";

  let invite = $state("");

  function onsubmit(e: SubmitEvent) {
    e.preventDefault();
    if (invite.trim()) group.join(invite);
  }
</script>

<div class="lobby">
  <p class="who">
    Connecté en tant que <strong>{account.username}</strong>
    <button class="link" onclick={() => account.logout()} disabled={account.busy}>Se déconnecter</button>
  </p>

  {#if group.error}
    <p class="error">{group.error}</p>
  {/if}

  <div class="cards">
    <div class="card">
      <h2>Nouveau groupe</h2>
      <p class="muted">Crée un groupe et partage le code avec tes amis.</p>
      <button class="primary" onclick={() => group.create()} disabled={group.busy}>Créer un groupe</button>
    </div>

    <form class="card" {onsubmit}>
      <h2>Rejoindre un groupe</h2>
      <p class="muted">Colle le code (ex. ABC123) ou le lien d'invitation.</p>
      <div class="join">
        <input
          bind:value={invite}
          placeholder="Code ou musicapp://join/…"
          spellcheck="false"
          aria-label="Code ou lien d'invitation"
        />
        <button class="secondary" type="submit" disabled={!invite.trim() || group.busy}>Rejoindre</button>
      </div>
    </form>
  </div>
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

  .error {
    margin: 0 0 1rem;
    color: #f15e6c;
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
    padding: 1.5rem;
    border-radius: 8px;
    background: var(--surface);
  }

  h2 {
    margin: 0;
    font-size: 1.1rem;
  }

  .muted {
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .join {
    display: flex;
    gap: 0.6rem;
    width: 100%;
  }

  input {
    flex: 1;
    min-width: 0;
    padding: 0.6rem 0.8rem;
    border: 1px solid transparent;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: 0.9rem;
    outline: none;
    user-select: text;
  }

  input:focus {
    border-color: var(--muted);
  }

  .primary,
  .secondary {
    flex: none;
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
