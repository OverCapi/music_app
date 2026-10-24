<script lang="ts">
  import type { Direction, Peer } from "#lib/bindings.ts";
  import { lan } from "#lib/stores/lan.svelte.ts";
  import Button from "#lib/components/ui/Button.svelte";
  import SegmentedPills from "#lib/components/ui/SegmentedPills.svelte";
  import Toggle from "#lib/components/ui/Toggle.svelte";

  let { peer }: { peer: Peer } = $props();

  const directions = [
    { id: "send", label: "Envoyer" },
    { id: "receive", label: "Recevoir" },
    { id: "both", label: "Les deux" },
  ];

  let direction = $state<Direction>("both");
  let mirror = $state(false);
  // A mirror needs a single receiver.
  const canMirror = $derived(direction !== "both");
</script>

<div class="peer">
  <div class="head">
    <span class="name">{peer.name}</span>
    <Button
      variant="primary"
      disabled={lan.syncing}
      onclick={() => lan.request(peer, direction, canMirror && mirror)}
    >
      Synchroniser
    </Button>
  </div>
  <SegmentedPills
    items={directions}
    value={direction}
    tabWidth={96}
    height={34}
    onselect={(id) => (direction = id as Direction)}
  />
  <div class="mirror" class:off={!canMirror}>
    <Toggle
      checked={canMirror && mirror}
      label="Mode miroir"
      onchange={(on) => canMirror && (mirror = on)}
    />
    <span class="muted">
      Miroir : {direction === "send" ? "l'autre appareil" : "cet appareil"} devient une copie (supprime
      ce qui manque)
    </span>
  </div>
</div>

<style>
  .peer {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 16px;
    border-top: 1px solid var(--line);
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .name {
    font-size: 15px;
    font-weight: 700;
  }
  .mirror {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }
  .mirror.off {
    opacity: 0.45;
  }
</style>
