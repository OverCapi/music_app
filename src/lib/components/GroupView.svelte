<script lang="ts">
  import { account } from "$lib/account.svelte";
  import { group } from "$lib/group.svelte";
  import GroupLogin from "$lib/components/GroupLogin.svelte";
  import GroupLobby from "$lib/components/GroupLobby.svelte";
  import GroupSession from "$lib/components/GroupSession.svelte";
</script>

<section class="group">
  <header>
    <h1>Groupe d'écoute</h1>
    <p class="subtitle">Écoute la même musique en même temps que tes amis.</p>
  </header>

  {#if group.view}
    <GroupSession view={group.view} />
  {:else if account.account === null}
    {#if account.error}
      <p class="error">{account.error}</p>
      <button class="retry" onclick={() => account.refresh()}>Réessayer</button>
    {:else}
      <p class="subtitle">Chargement…</p>
    {/if}
  {:else if !account.loggedIn}
    <GroupLogin />
  {:else}
    <GroupLobby />
  {/if}
</section>

<style>
  .group {
    padding: 1.5rem 2rem;
    overflow-y: auto;
    min-height: 0;
  }

  header {
    margin-bottom: 1.5rem;
  }

  h1 {
    margin: 0;
    font-size: 2rem;
  }

  .subtitle {
    margin: 0.25rem 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .error {
    margin: 0 0 1rem;
    color: #f15e6c;
  }

  .retry {
    padding: 0.5rem 1.2rem;
    border: 1px solid var(--muted);
    border-radius: 999px;
    background: transparent;
    color: var(--text);
    font-weight: 700;
  }
</style>
