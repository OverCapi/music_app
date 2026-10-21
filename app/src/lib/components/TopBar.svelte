<script lang="ts">
  /** Floating pill: logo (pulses with the music) · draggable segmented nav · search · settings.
   * Narrow windows (compact): icon tabs, a playlists-drawer button, and the search field
   * moves to the search page. */
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import { player } from "#lib/stores/player.svelte.ts";
  import { search } from "#lib/stores/search.svelte.ts";
  import { ui } from "#lib/stores/ui.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import IconButton from "#lib/components/ui/IconButton.svelte";
  import SegmentedPills from "#lib/components/ui/SegmentedPills.svelte";

  const tabs = [
    { id: resolve("/"), href: resolve("/"), label: "Accueil", icon: "home" as const },
    {
      id: resolve("/library"),
      href: resolve("/library"),
      label: "Bibliothèque",
      icon: "library" as const,
    },
    { id: resolve("/lives"), href: resolve("/lives"), label: "Lives", icon: "live" as const },
    { id: resolve("/group"), href: resolve("/group"), label: "Groupe", icon: "group" as const },
  ];

  let input = $state<HTMLInputElement>();
  const current = $derived(tabs.find((t) => t.id === page.url.pathname)?.id);
  const settings = $derived(page.url.pathname === resolve("/settings"));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!search.query.trim()) return;
    if (page.url.pathname !== resolve("/search")) goto(resolve("/search"));
    search.search();
    input?.blur();
  }

  function onkeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      if (ui.compact) return goto(resolve("/search"));
      input?.focus();
      input?.select();
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="wrap" style:left="{ui.left}px" style:right="{ui.right}px">
  <header class="bar glass">
    {#if ui.compact}
      <IconButton
        icon="queue"
        label="Playlists"
        size={40}
        onclick={() => {
          ui.queueOpen = false;
          ui.sideOpen = true;
        }}
      />
    {:else}
      <a class="brand" href={resolve("/")}>
        <span class="logo" class:playing={player.state.playing}><span></span></span>
        <span class="word">Halo</span>
      </a>
    {/if}

    <SegmentedPills
      items={tabs}
      value={current}
      onselect={(href) => goto(href)}
      tabWidth={ui.compact ? 44 : 104}
      height={40}
      glass={false}
      iconOnly={ui.compact}
    />

    {#if ui.compact}
      <IconButton
        icon="search"
        label="Rechercher"
        size={40}
        active={page.url.pathname === resolve("/search")}
        href={resolve("/search")}
      />
    {:else}
      <form class="search" role="search" onsubmit={submit}>
        <Icon name="search" size={15} />
        <input
          bind:this={input}
          bind:value={search.query}
          type="search"
          placeholder="Rechercher"
          aria-label="Rechercher"
        />
      </form>
    {/if}

    <IconButton
      icon="settings"
      label="Paramètres"
      size={40}
      active={settings}
      href={resolve("/settings")}
    />
  </header>
</div>

<style>
  .wrap {
    position: absolute;
    top: 18px;
    z-index: 10;
    display: flex;
    justify-content: center;
    box-sizing: border-box;
    padding: 0 16px;
    pointer-events: none;
    transition:
      left 0.55s var(--ease-out),
      right 0.55s var(--ease-out);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    padding: 6px;
    box-sizing: border-box;
    border-radius: 999px;
    pointer-events: auto;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px 0 12px;
  }
  .logo {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    box-sizing: border-box;
    border: 2px solid var(--accent);
    border-radius: 50%;
  }
  .logo span {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
  }
  .logo.playing {
    animation: beat 0.9s ease-in-out infinite alternate;
  }
  @keyframes beat {
    from {
      transform: scale(1);
      box-shadow: 0 0 4px var(--accent);
    }
    to {
      transform: scale(1.3);
      box-shadow: 0 0 20px var(--accent);
    }
  }
  .word {
    font-size: 16px;
    font-weight: 800;
    letter-spacing: -0.03em;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    margin-left: 4px;
    color: var(--mute);
  }
  .search :global(svg) {
    position: absolute;
    left: 13px;
    pointer-events: none;
  }
  .search input {
    width: 150px;
    height: 40px;
    box-sizing: border-box;
    padding: 0 14px 0 36px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: var(--line);
    color: var(--fg);
    font: inherit;
    font-size: 13px;
    outline: none;
    user-select: text;
    transition:
      width 0.5s var(--ease-out),
      border-color 0.3s,
      background 0.3s;
  }
  .search input:focus {
    width: 230px;
    border-color: var(--border);
    background: var(--glass2);
  }
</style>
