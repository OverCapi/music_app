<script lang="ts">
  import { account } from "#lib/stores/account.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import GroupLogin from "#lib/components/group/GroupLogin.svelte";
  import GroupLobby from "#lib/components/group/GroupLobby.svelte";
  import GroupSession from "#lib/components/group/GroupSession.svelte";
</script>

<section class="view">
  <header>
    <h1>Groupe d'écoute</h1>
    <p class="subtitle">Écoute la même musique en même temps que tes amis.</p>
  </header>

  {#if group.view}
    <GroupSession view={group.view} />
  {:else if account.account === null}
    {#if account.failed}
      <button class="pill secondary" onclick={() => account.refresh()}>Réessayer</button>
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
  header {
    margin-bottom: 1.5rem;
  }

  .subtitle {
    margin: 0.25rem 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
</style>
