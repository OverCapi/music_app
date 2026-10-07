<script lang="ts">
  import { player } from "#lib/stores/player.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  let { trackId }: { trackId: number } = $props();
</script>

<button
  class="row-action"
  onclick={() => player.enqueue([trackId])}
  aria-label="Ajouter à la file d'attente"
  title="Ajouter à la file d'attente"
>
  <Icon name="enqueue" size={16} />
</button>
