<script lang="ts">
  /** Placeholder track rows shown while a list loads. */
  let { rows = 6 }: { rows?: number } = $props();
</script>

<div class="glass-flat sk" aria-busy="true" aria-label="Chargement">
  {#each Array.from({ length: rows }, (_, i) => i) as i (i)}
    <div class="row" style:animation-delay="{i * 90}ms">
      <span class="cover"></span>
      <span class="lines"><span class="a"></span><span class="b"></span></span>
    </div>
  {/each}
</div>

<style>
  .sk {
    display: flex;
    flex-direction: column;
    padding: 8px;
    border-radius: calc(var(--rs) + 8px);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 14px;
    animation: pulse 1.4s ease-in-out infinite alternate;
  }
  .cover {
    flex: none;
    width: 44px;
    height: 44px;
    border-radius: calc(var(--rs) / 2);
    background: var(--line);
  }
  .lines {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 6px;
  }
  .a,
  .b {
    height: 9px;
    border-radius: 4px;
    background: var(--line);
  }
  .a {
    width: 40%;
    height: 10px;
  }
  .b {
    width: 22%;
  }
  @keyframes pulse {
    from {
      opacity: 0.45;
    }
  }
</style>
