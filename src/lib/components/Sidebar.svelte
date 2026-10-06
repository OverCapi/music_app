<script lang="ts">
  export type View = "library" | "search";

  let { view = $bindable() }: { view: View } = $props();

  const items: { id: View; label: string; icon: string }[] = [
    {
      id: "library",
      label: "Bibliothèque",
      icon: "M4 4h2v16H4zm4 0h2v16H8zm4.5.3 1.9-.6 5 15.2-1.9.6z",
    },
    {
      id: "search",
      label: "Rechercher",
      icon: "M10 3a7 7 0 0 1 5.6 11.2l5.1 5.1-1.4 1.4-5.1-5.1A7 7 0 1 1 10 3zm0 2a5 5 0 1 0 0 10 5 5 0 0 0 0-10z",
    },
  ];
</script>

<nav class="sidebar">
  {#each items as item (item.id)}
    <button
      class:active={view === item.id}
      onclick={() => (view = item.id)}
      aria-current={view === item.id ? "page" : undefined}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d={item.icon} /></svg>
      {item.label}
    </button>
  {/each}
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 1.25rem 0.75rem;
    background: #000;
  }

  button {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    padding: 0.6rem 0.75rem;
    border-radius: 4px;
    background: transparent;
    color: var(--muted);
    font-weight: 700;
    font-size: 0.9rem;
    text-align: left;
  }

  button:hover,
  button.active {
    color: var(--text);
  }

  button.active {
    background: var(--surface-2);
  }

  svg {
    flex: none;
    width: 22px;
    height: 22px;
    fill: currentColor;
  }
</style>
