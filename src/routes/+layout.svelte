<script lang="ts">
  import "../app.css";
  import { onMount, type Snippet } from "svelte";
  import { player } from "#lib/stores/player.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { playlists } from "#lib/stores/playlists.svelte.ts";
  import { search } from "#lib/stores/search.svelte.ts";
  import { downloads } from "#lib/stores/downloads.svelte.ts";
  import { account } from "#lib/stores/account.svelte.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { sync } from "#lib/stores/sync.svelte.ts";
  import { errorMessage, toasts } from "#lib/stores/toasts.svelte.ts";
  import Sidebar from "#lib/components/Sidebar.svelte";
  import PlayerBar from "#lib/components/player/PlayerBar.svelte";
  import QueueView from "#lib/components/player/QueueView.svelte";
  import Toasts from "#lib/components/ui/Toasts.svelte";

  let { children }: { children: Snippet } = $props();

  let queueOpen = $state(false);

  onMount(() => {
    account.refresh();
    // Each store subscribes to its events before loading its initial state.
    const unlisteners = [player, library, playlists, search, downloads, group, sync].map((store) =>
      store.init().catch((e) => {
        toasts.push(errorMessage(e));
        return () => {};
      }),
    );
    return () => unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
  });
</script>

<main class:queue-open={queueOpen}>
  <Sidebar />
  {@render children()}
  {#if queueOpen}
    <QueueView onclose={() => (queueOpen = false)} />
  {/if}
  <PlayerBar bind:queueOpen />
</main>
<Toasts />

<style>
  main {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr) auto;
    height: 100vh;
  }

  main.queue-open {
    grid-template-columns: 220px minmax(0, 1fr) 320px;
  }
</style>
