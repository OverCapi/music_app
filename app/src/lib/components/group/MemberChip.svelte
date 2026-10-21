<script lang="ts">
  import Avatar from "#lib/components/ui/Avatar.svelte";

  /** Member of the group: initial, name, and ✓ / … when a track is current. Your own chip
   * has an accent border. */
  let { name, me = false, ready }: { name: string; me?: boolean; ready?: boolean } = $props();
</script>

<span class="chip glass-flat" class:me>
  <Avatar {name} />
  <span class="name">{name}</span>
  {#if ready !== undefined}
    <span
      class="ready"
      class:ok={ready}
      title={ready ? "Titre en cours téléchargé" : "Téléchargement du titre en cours…"}
    >
      {ready ? "✓" : "…"}
    </span>
  {/if}
</span>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px 5px 5px;
    border-radius: 999px;
  }
  .chip.me {
    border-color: var(--accent);
  }
  .name {
    font-size: 13px;
    font-weight: 600;
  }
  .ready {
    color: var(--mute);
    font-size: 12px;
    font-weight: 700;
  }
  .ready.ok {
    color: var(--accent);
  }
</style>
