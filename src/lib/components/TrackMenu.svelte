<script lang="ts" module>
  import type { Track } from '../types'

  export interface MenuState {
    x: number
    y: number
    track: Track
    context: Track[]
  }
</script>

<script lang="ts">
  import { player } from '../stores/player.svelte'

  let { menu = $bindable() }: { menu: MenuState | null } = $props()

  const MENU_W = 190
  const MENU_H = 110
  // Keep the menu inside the window when opened near an edge.
  const left = $derived(menu ? Math.min(menu.x, window.innerWidth - MENU_W) : 0)
  const top = $derived(menu ? Math.min(menu.y, window.innerHeight - MENU_H) : 0)

  function run(action: (m: MenuState) => void) {
    if (menu) action(menu)
    menu = null
  }
</script>

<svelte:window onclick={() => (menu = null)} onkeydown={(e) => e.key === 'Escape' && (menu = null)} onblur={() => (menu = null)} />

{#if menu}
  <div class="menu" role="menu" style:left="{left}px" style:top="{top}px">
    <button role="menuitem" onclick={() => run((m) => player.play(m.track, m.context))}>Lire</button>
    <button role="menuitem" onclick={() => run((m) => player.playNext(m.track))}>Lire ensuite</button>
    <button role="menuitem" onclick={() => run((m) => player.addToQueue(m.track))}>Ajouter à la file</button>
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 100;
    min-width: 180px;
    padding: 4px;
    border-radius: 8px;
    background: var(--surface-3);
    border: 1px solid var(--border);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.4);
    display: flex;
    flex-direction: column;
  }
  button {
    padding: 7px 10px;
    border-radius: 5px;
    text-align: left;
  }
  button:hover {
    background: var(--accent);
    color: var(--on-accent);
  }
</style>
