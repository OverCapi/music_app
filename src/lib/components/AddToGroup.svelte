<script lang="ts">
  import { onDestroy } from "svelte";
  import { group, type NewEntry } from "$lib/group.svelte";

  let { entry }: { entry: NewEntry } = $props();

  let added = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  const queued = $derived(group.state?.queue.some((e) => e.youtube_id === entry.youtubeId) ?? false);
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
  {#if added}
    <svg viewBox="0 0 24 24"><path d="m9.5 16.2-4.2-4.2-1.4 1.4 5.6 5.6 11-11-1.4-1.4z" /></svg>
  {:else}
    <svg viewBox="0 0 24 24">
      <path d="M8 4a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7zm0 9c3.3 0 6 1.5 6 3.8V19H2v-2.2C2 14.5 4.7 13 8 13zm10-4h2v3h3v2h-3v3h-2v-3h-3v-2h3z" />
    </svg>
  {/if}
</button>

<style>
  /* Size and hover come from the parent's `row-action` styles. */
  button.add-group.added {
    color: var(--accent);
    visibility: visible;
  }
</style>
