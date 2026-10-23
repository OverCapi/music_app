<script lang="ts">
  /** Fills its (positioned) parent with an image; a new `src` fades and un-blurs in. */
  let { src }: { src?: string } = $props();
</script>

{#key src}{#if src}<img {src} alt="" />{/if}{/key}

<style>
  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: swapin 0.45s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  @keyframes swapin {
    from {
      opacity: 0.3;
      transform: scale(1.1);
      filter: blur(10px);
    }
  }
</style>
