<script lang="ts">
  import { player } from "#lib/stores/player.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import type { PlaySource } from "#lib/bindings.ts";

  let { trackId, source }: { trackId: number; source: PlaySource } = $props();
</script>

<button
  class="row-action"
  onclick={() => player.enqueue([trackId], source)}
  aria-label="Ajouter à la file d'attente"
  title="Ajouter à la file d'attente"
>
  <Icon name="enqueue" size={16} />
</button>
