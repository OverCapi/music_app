<script lang="ts">
  import { onDestroy } from "svelte";
  import { group, type NewEntry } from "#lib/stores/group.svelte.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  /** `size`: icon size (16 in track rows, 22 next to the download button). */
  let { entry, size = 16 }: { entry: NewEntry; size?: number } = $props();

  let added = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  const queued = $derived(
    group.state?.queue.some((e) => e.youtube_id === entry.youtubeId) ?? false,
  );
  const label = $derived(
    added
      ? "Ajouté au groupe"
      : queued
        ? "Déjà dans la file du groupe — ajouter encore"
        : "Ajouter au groupe",
  );

  async function add() {
    if (!(await group.add(entry))) return;
    added = true;
    clearTimeout(timer);
    timer = setTimeout(() => (added = false), 1500);
  }

  onDestroy(() => clearTimeout(timer));
</script>

<button class="row-action add-group" class:added onclick={add} aria-label={label} title={label}>
  <Icon name={added ? "check" : "groupAdd"} {size} />
</button>

<style>
  /* In a track row, size and hover come from the `row-action` styles of TrackTable. */
  .add-group.added {
    color: var(--accent);
    visibility: visible;
  }
</style>
