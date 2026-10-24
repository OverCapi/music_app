<script lang="ts">
  import { lan } from "#lib/stores/lan.svelte.ts";
  import Button from "#lib/components/ui/Button.svelte";
  import Dialog from "#lib/components/ui/Dialog.svelte";

  const request = $derived(lan.incoming);
</script>

{#if request}
  <Dialog title="Demande de synchronisation" onclose={() => lan.answer(false)}>
    <p>
      <strong>{request.name}</strong>
      {#if request.direction === "send"}
        veut t'envoyer sa bibliothèque et ses playlists.
      {:else if request.direction === "receive"}
        veut récupérer ta bibliothèque et tes playlists. La tienne ne change pas.
      {:else}
        veut échanger sa bibliothèque et ses playlists avec les tiens : vous récupérez chacun ce qui
        manque à l'autre.
      {/if}
    </p>
    {#if request.mirror}
      <p class="warn">
        Mode miroir : ta bibliothèque deviendra une copie de la sienne. Les titres et playlists
        qu'elle n'a pas seront supprimés de chez toi.
      </p>
    {/if}
    {#snippet actions()}
      <Button onclick={() => lan.answer(false)}>Refuser</Button>
      <Button variant="primary" onclick={() => lan.answer(true)}>Accepter</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  p {
    margin: 0;
  }
  .warn {
    color: var(--danger);
  }
</style>
