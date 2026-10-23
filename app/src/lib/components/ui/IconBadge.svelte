<script lang="ts">
  import Icon, { type IconName } from "#lib/components/ui/Icon.svelte";

  /** Round tinted badge holding an accent icon (card headers). */
  let { icon, size = 48 }: { icon: IconName; size?: number } = $props();
</script>

<span class="badge" style:width="{size}px" style:height="{size}px">
  <Icon name={icon} size={Math.round(size / 2.3)} />
</span>

<style>
  .badge {
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--line);
    color: var(--accent);
  }
</style>
