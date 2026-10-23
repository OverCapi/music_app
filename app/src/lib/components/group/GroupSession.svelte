<script lang="ts">
  import SectionLabel from "#lib/components/ui/SectionLabel.svelte";
  import { onDestroy } from "svelte";
  import type { GroupView } from "#lib/bindings.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { toasts } from "#lib/stores/toasts.svelte.ts";
  import Button from "#lib/components/ui/Button.svelte";
  import Panel from "#lib/components/ui/Panel.svelte";
  import GroupQueue from "#lib/components/group/GroupQueue.svelte";
  import MemberChip from "#lib/components/group/MemberChip.svelte";

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

<Panel radius={14} pad={24} class="top">
  <div class="code-block">
    <SectionLabel>Code du groupe</SectionLabel>
    <div class="code-line">
      <span class="code">{view.code}</span>
      <Button size={36} onclick={copyLink} title={view.link} class="copy">
        <span class="swap">
          <span class:out={copied}>Copier le lien</span>
          <span class:out={!copied}>Copié</span>
        </span>
      </Button>
    </div>
  </div>
  <div class="side">
    <span class="conn" class:offline={!view.connected}>
      <span class="dot"></span>{view.connected ? "Connecté" : "Reconnexion…"}
    </span>
    <Button variant="outline" size={36} onclick={() => group.leave()} disabled={group.busy}>
      Quitter
    </Button>
  </div>
</Panel>

<section>
  <SectionLabel>Membres · {view.state.members.length}</SectionLabel>
  <div class="members">
    {#each view.state.members as member (member.username)}
      <MemberChip
        name={member.username}
        me={member.username === view.me}
        ready={hasCurrent ? member.ready : undefined}
      />
    {/each}
  </div>
</section>

<section>
  <div class="queue-header">
    <SectionLabel>File du groupe</SectionLabel>
    <span class="status">{status}</span>
  </div>
  <GroupQueue {view} />
</section>

<style>
  :global(.top) {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
  }
  .code-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .code-line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 16px;
  }
  .code {
    font-family: var(--mono);
    font-size: clamp(32px, 6vw, 48px);
    font-weight: 500;
    letter-spacing: 0.14em;
    line-height: 1;
    user-select: text;
  }
  .swap {
    display: grid;
    font-size: 13px;
  }
  .swap > span {
    grid-area: 1 / 1;
    text-align: center;
    transition:
      opacity 0.25s,
      transform 0.35s var(--ease-spring);
  }
  .swap > .out {
    opacity: 0;
    transform: translateY(-8px) scale(0.9);
  }
  .side {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .conn {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--mute);
    font-size: 13px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 2s ease-in-out infinite;
  }
  .offline .dot {
    background: var(--danger);
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .members {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .queue-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  .status {
    color: var(--accent);
    font-size: 13px;
    font-weight: 600;
  }
</style>
