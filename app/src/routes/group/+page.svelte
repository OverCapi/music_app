<script lang="ts">
  import { tick } from "svelte";
  import { account } from "#lib/stores/account.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { enterPage } from "#lib/motion.ts";
  import Button from "#lib/components/ui/Button.svelte";
  import GroupLogin from "#lib/components/group/GroupLogin.svelte";
  import GroupLobby from "#lib/components/group/GroupLobby.svelte";
  import GroupSession from "#lib/components/group/GroupSession.svelte";

  const step = $derived(
    group.view
      ? "session"
      : account.account === null
        ? "loading"
        : account.loggedIn
          ? "lobby"
          : "login",
  );

  // Each step fades in like a page.
  let body = $state<HTMLElement>();
  let previous = "";
  $effect(() => {
    const current = step;
    if (previous && previous !== current) tick().then(() => body && enterPage(body));
    previous = current;
  });
</script>

<div class="page" bind:this={body}>
  {#if step === "session" && group.view}
    <GroupSession view={group.view} />
  {:else if step === "loading"}
    {#if account.failed}
      <Button onclick={() => account.refresh()}>Réessayer</Button>
    {:else}
      <p class="muted">Chargement…</p>
    {/if}
  {:else if step === "login"}
    <GroupLogin />
  {:else}
    <GroupLobby />
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 28px;
    padding-top: 20px;
  }
</style>
