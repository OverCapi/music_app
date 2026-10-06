<script lang="ts">
  import { onMount } from "svelte";
  import { player } from "$lib/player.svelte";
  import { library } from "$lib/library.svelte";
  import LibraryView from "$lib/components/LibraryView.svelte";
  import PlayerBar from "$lib/components/PlayerBar.svelte";
  import QueueView from "$lib/components/QueueView.svelte";

  let queueOpen = $state(false);

  onMount(() => {
    library.refresh();
    player.init();
    return () => player.stopPolling();
  });
</script>

<main class:queue-open={queueOpen}>
  <LibraryView />
  {#if queueOpen}
    <QueueView onclose={() => (queueOpen = false)} />
  {/if}
  <PlayerBar bind:queueOpen />
</main>

<style>
  :global(:root) {
    --bg: #121212;
    --surface: #181818;
    --surface-2: #282828;
    --text: #fff;
    --muted: #b3b3b3;
    --accent: #1db954;
    --track: #4d4d4d;
    color-scheme: dark;
    font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  }

  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--text);
    user-select: none;
  }

  :global(button) {
    font: inherit;
    border: none;
    cursor: pointer;
  }

  :global(button:disabled) {
    cursor: default;
    opacity: 0.4;
  }

  :global(input[type="range"]) {
    flex: 1;
    height: 4px;
    margin: 0;
    appearance: none;
    border-radius: 2px;
    background: linear-gradient(to right, var(--text) var(--fill), var(--track) var(--fill));
    cursor: pointer;
  }

  :global(input[type="range"]:hover:not(:disabled)) {
    background: linear-gradient(to right, var(--accent) var(--fill), var(--track) var(--fill));
  }

  :global(input[type="range"]:disabled) {
    cursor: default;
    opacity: 0.5;
  }

  :global(input[type="range"]::-webkit-slider-thumb) {
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text);
    opacity: 0;
  }

  :global(input[type="range"]:hover:not(:disabled)::-webkit-slider-thumb) {
    opacity: 1;
  }

  main {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr) auto;
    height: 100vh;
  }

  main.queue-open {
    grid-template-columns: minmax(0, 1fr) 320px;
  }

  main > :global(footer) {
    grid-column: 1 / -1;
  }
</style>
