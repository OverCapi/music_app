<script lang="ts">
  import { onDestroy } from "svelte";
  import { group, type NewEntry } from "#lib/stores/group.svelte.ts";
  import IconButton from "#lib/components/ui/IconButton.svelte";

  /** Adds a YouTube track to the group queue (check mark for a moment once done). */
  let { entry }: { entry: NewEntry } = $props();

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

<IconButton
  icon={added ? "check" : "groupAdd"}
  {label}
  size={34}
  iconSize={18}
  active={added}
  onclick={add}
/>
