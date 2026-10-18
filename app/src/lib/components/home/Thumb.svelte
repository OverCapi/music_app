<script lang="ts">
  let { id, size }: { id: string; size?: number } = $props();
  let failed = $state(false);

  // A new track id gets a fresh chance to load.
  $effect(() => {
    void id;
    failed = false;
  });
</script>

<span
  class="thumb"
  style:width={size ? `${size}px` : undefined}
  style:height={size ? `${size}px` : undefined}
>
  {#if !failed}
    <img
      src={`https://i.ytimg.com/vi/${id}/mqdefault.jpg`}
      alt=""
      loading="lazy"
      onerror={() => (failed = true)}
    />
  {/if}
</span>

<style>
  .thumb {
    display: block;
    overflow: hidden;
    background: var(--surface-2);
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
