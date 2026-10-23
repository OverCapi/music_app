<script lang="ts">
  import Field from "#lib/components/ui/Field.svelte";
  import IconBadge from "#lib/components/ui/IconBadge.svelte";
  import { account } from "#lib/stores/account.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import Button from "#lib/components/ui/Button.svelte";
  import Panel from "#lib/components/ui/Panel.svelte";

  let invite = $state("");

  function onsubmit(e: SubmitEvent) {
    e.preventDefault();
    if (invite.trim()) group.join(invite);
  }
</script>

<header>
  <h1>Groupe d'écoute</h1>
  <button class="logout" onclick={() => account.logout()} disabled={account.busy}>
    {account.username} · Se déconnecter
  </button>
</header>

<div class="cards">
  <Panel radius={14} pad={28} class="card-lobby">
    <IconBadge icon="plus" />
    <h2>Créer un groupe</h2>
    <Button variant="primary" onclick={() => group.create()} disabled={group.busy}>Créer</Button>
  </Panel>

  <form {onsubmit}>
    <Panel radius={14} pad={28} class="card-lobby">
      <IconBadge icon="copy" />
      <h2>Rejoindre</h2>
      <div class="join">
        <Field
          variant="code"
          bind:value={invite}
          placeholder="Code ou lien"
          spellcheck="false"
          aria-label="Code ou lien d'invitation"
        />
        <Button size={44} type="submit" disabled={!invite.trim() || group.busy}>Rejoindre</Button>
      </div>
    </Panel>
  </form>
</div>

<style>
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
  }
  .logout {
    padding: 0;
    background: transparent;
    color: var(--mute);
    font-size: 13px;
    font-weight: 600;
    transition: color 0.2s;
  }
  .logout:hover {
    color: var(--fg);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 18px;
  }
  .cards :global(.card-lobby) {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 18px;
    box-sizing: border-box;
    height: 100%;
  }
  .cards h2 {
    margin: 0;
    font-size: 22px !important;
  }
  form {
    display: contents;
  }
  .join {
    display: flex;
    gap: 10px;
    width: 100%;
  }
</style>
