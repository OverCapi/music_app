<script lang="ts">
  import { onDestroy } from "svelte";
  import type { GroupView } from "#lib/bindings.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { toasts } from "#lib/stores/toasts.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import GroupQueue from "#lib/components/group/GroupQueue.svelte";

  let { view }: { view: GroupView } = $props();

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  const hasCurrent = $derived(view.state.current !== null);

  const status = $derived.by(() => {
    const p = view.state.playback;
    if (view.state.queue.length === 0) return "File vide";
    switch (p.status) {
      case "preparing": {
        const n = group.waitingFor.length;
        return n === 0
          ? "Préparation — démarrage…"
          : `Préparation — en attente de ${n} membre${n > 1 ? "s" : ""}`;
      }
      case "playing":
        return "En lecture";
      case "paused":
        return "En pause";
      default:
        return "À l'arrêt — choisis un titre dans la file";
    }
  });

  async function copyLink() {
    try {
      await navigator.clipboard.writeText(view.link);
      copied = true;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copied = false), 1500);
    } catch {
      toasts.push("Impossible de copier le lien.");
    }
  }

  onDestroy(() => clearTimeout(copyTimer));
</script>

<div class="top">
  <div>
    <span class="label">Code du groupe</span>
    <div class="code-line">
      <span class="code">{view.code}</span>
      <button class="copy" class:copied onclick={copyLink} title={view.link}>
        {#if copied}
          Copié !
        {:else}
          <Icon name="copy" size={14} />
          Copier le lien
        {/if}
      </button>
    </div>
  </div>
  <div class="side">
    {#if view.connected}
      <span class="conn">Connecté</span>
    {:else}
      <span class="conn offline">Reconnexion…</span>
    {/if}
    <button class="leave" onclick={() => group.leave()} disabled={group.busy}>
      Quitter le groupe
    </button>
  </div>
</div>

<h3 class="section-title">Membres · {view.state.members.length}</h3>
<ul class="members">
  {#each view.state.members as member (member.username)}
    {@const me = member.username === view.me}
    <li class:me>
      <span class="avatar" aria-hidden="true">{member.username.slice(0, 1).toUpperCase()}</span>
      <span
        >{member.username}{#if me}<span class="muted"> (toi)</span>{/if}</span
      >
      {#if hasCurrent}
        <span
          class="ready"
          class:ok={member.ready}
          title={member.ready ? "Titre en cours téléchargé" : "Téléchargement du titre en cours…"}
        >
          {member.ready ? "✓" : "…"}
        </span>
      {/if}
    </li>
  {/each}
</ul>

<div class="queue-header">
  <h3 class="section-title">File du groupe</h3>
  <span class="status" class:live={view.state.playback.status === "playing"}>{status}</span>
</div>

<GroupQueue {view} />

<style>
  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .label {
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .code-line {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .code {
    font-size: 2.5rem;
    font-weight: 800;
    letter-spacing: 0.15em;
    font-variant-numeric: tabular-nums;
    user-select: text;
  }

  .copy {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.45rem 0.9rem;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.8rem;
    font-weight: 600;
  }

  .copy:hover {
    background: var(--surface-3);
  }

  .copy.copied {
    color: var(--accent);
  }

  .side {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .conn {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--muted);
    font-size: 0.8rem;
  }

  .conn::before {
    content: "";
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
  }

  .conn.offline {
    color: var(--warning);
  }

  .conn.offline::before {
    background: var(--warning);
    animation: pulse 1.2s ease-in-out infinite;
  }

  .leave {
    padding: 0.5rem 1.1rem;
    border: 1px solid var(--muted);
    border-radius: 999px;
    background: transparent;
    color: var(--text);
    font-size: 0.85rem;
    font-weight: 700;
  }

  .leave:not(:disabled):hover {
    border-color: var(--danger);
    color: var(--danger);
  }

  .members {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .members li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.3rem 0.75rem 0.3rem 0.3rem;
    border-radius: 999px;
    background: var(--surface-2);
    font-size: 0.85rem;
  }

  .members li.me {
    box-shadow: inset 0 0 0 1px var(--accent);
  }

  .avatar {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--surface-3);
    font-size: 0.75rem;
    font-weight: 700;
  }

  .me .avatar {
    background: var(--accent);
    color: #000;
  }

  .ready {
    color: var(--muted);
    font-size: 0.8rem;
  }

  .ready.ok {
    color: var(--accent);
  }

  .queue-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
  }

  .status {
    color: var(--muted);
    font-size: 0.85rem;
  }

  .status.live {
    color: var(--accent);
  }

  @keyframes pulse {
    50% {
      opacity: 0.4;
    }
  }
</style>
