<script lang="ts">
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import { search } from "#lib/stores/search.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { plural } from "#lib/format.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  let input = $state<HTMLInputElement>();
  const inSettings = $derived(page.url.pathname === resolve("/settings"));

  function onsubmit(e: SubmitEvent) {
    e.preventDefault();
    if (!search.query.trim()) return;
    if (page.url.pathname !== resolve("/search")) goto(resolve("/search"));
    search.search();
  }

  // ⌘K / Ctrl+K focuses the search field from anywhere.
  function onkeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      input?.focus();
      input?.select();
    }
  }
</script>

<svelte:window {onkeydown} />

<header class="top-bar">
  <div class="history">
    <button class="icon-btn" onclick={() => history.back()} aria-label="Page précédente">
      <Icon name="chevronLeft" size={22} />
    </button>
    <button class="icon-btn" onclick={() => history.forward()} aria-label="Page suivante">
      <Icon name="chevronRight" size={22} />
    </button>
  </div>

  <form class="search" role="search" {onsubmit}>
    <Icon name="search" size={22} />
    <input
      bind:this={input}
      bind:value={search.query}
      type="search"
      placeholder="Rechercher un titre, un artiste… sur YouTube et YouTube Music"
      aria-label="Rechercher"
    />
    <span class="kbd">⌘K</span>
  </form>

  <div class="end">
    {#if group.isActive}
      <a class="group-chip" href={resolve("/group")}>
        <span class="dot" class:offline={!group.view?.connected}></span>
        Groupe · {plural(group.state?.members.length ?? 0, "membre")}
      </a>
    {/if}
    <a
      class="icon-btn"
      class:active={inSettings}
      href={resolve("/settings")}
      aria-label="Paramètres"
      aria-current={inSettings ? "page" : undefined}
      title="Paramètres"
    >
      <Icon name="settings" size={22} />
    </a>
  </div>
</header>

<style>
  .top-bar {
    display: flex;
    flex: none;
    align-items: center;
    gap: 16px;
    padding-top: 4px;
  }

  .history {
    display: flex;
    gap: 4px;
  }

  .search {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 10px;
    max-width: 640px;
    height: 44px;
    margin: 0 auto;
    box-sizing: border-box;
    padding: 0 12px 0 16px;
    border: 1px solid transparent;
    border-radius: 12px;
    background: var(--surface-2);
    color: var(--muted);
  }

  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 4px var(--accent-soft);
  }

  input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 0.95rem;
    outline: none;
    user-select: text;
  }

  .end {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .group-chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 14px;
    border-radius: 999px;
    background: var(--surface-2);
    font-size: 0.8rem;
    font-weight: 600;
  }

  .group-chip:hover {
    background: var(--surface-3);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--coral);
  }

  .dot.offline {
    background: var(--warning);
  }
</style>
