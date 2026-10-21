<script lang="ts">
  /** Round initial. */
  let { name, size = 28 }: { name: string; size?: number } = $props();
</script>

<span
  class="avatar"
  style:width="{size}px"
  style:height="{size}px"
  style:font-size="{Math.round(size * 0.42)}px"
  aria-hidden="true"
>
  {name.slice(0, 1).toUpperCase()}
</span>

<style>
  .avatar {
    display: grid;
    flex: none;
    place-items: center;
    border-radius: 50%;
    background: var(--line);
    font-weight: 700;
  }
</style>
