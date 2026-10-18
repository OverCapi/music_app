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
  import { feedback } from "#lib/stores/feedback.svelte.ts";
  import { sync } from "#lib/stores/sync.svelte.ts";
  import { errorMessage, toasts } from "#lib/stores/toasts.svelte.ts";
  import Sidebar from "#lib/components/Sidebar.svelte";
  import TopBar from "#lib/components/TopBar.svelte";
  import PlayerBar from "#lib/components/player/PlayerBar.svelte";
  import QueueView from "#lib/components/player/QueueView.svelte";
  import Toasts from "#lib/components/ui/Toasts.svelte";

  let { children }: { children: Snippet } = $props();

  let queueOpen = $state(false);

  onMount(() => {
    account.refresh();
    // Each store subscribes to its events before loading its initial state.
    const unlisteners = [player, library, playlists, search, downloads, group, sync, feedback].map(
      (store) =>
        store.init().catch((e) => {
          toasts.push(errorMessage(e));
          return () => {};
        }),
    );
    return () => unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
  });
</script>

<main>
  <Sidebar />
  <div class="content">
    <TopBar />
    {@render children()}
  </div>
  {#if queueOpen}
    <QueueView onclose={() => (queueOpen = false)} />
  {/if}
  <PlayerBar bind:queueOpen />
</main>
<Toasts />

<style>
  /* Floating shell: rail | top bar + page; the queue and the player dock float over the page. */
  main {
    position: relative;
    display: flex;
    gap: 16px;
    height: 100vh;
    box-sizing: border-box;
    padding: 16px;
    overflow: hidden;
  }

  .content {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }
</style>
