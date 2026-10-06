<script lang="ts">
  import Icon from './Icon.svelte'

  let { query = $bindable(), input = $bindable() }: { query: string; input?: HTMLInputElement } = $props()
</script>

<label class="search">
  <Icon name="search" size={16} />
  <input
    bind:this={input}
    bind:value={query}
    type="search"
    placeholder="Rechercher un titre, un artiste, un album…"
    onkeydown={(e) => e.key === 'Escape' && ((query = ''), input?.blur())}
  />
  {#if query}
    <button class="icon" title="Effacer" onclick={() => (query = '')}><Icon name="close" size={14} /></button>
  {/if}
</label>

<style>
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    width: min(420px, 100%);
    padding: 0 12px;
    height: 36px;
    border-radius: 18px;
    background: var(--surface-2);
    color: var(--text-dim);
    border: 1px solid transparent;
  }
  .search:focus-within {
    border-color: var(--accent);
    color: var(--text);
  }
  input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    color: var(--text);
    font: inherit;
  }
  input::-webkit-search-cancel-button {
    display: none;
  }
</style>
