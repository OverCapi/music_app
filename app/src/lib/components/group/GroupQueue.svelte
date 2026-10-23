<script lang="ts">
  import { resolve } from "$app/paths";
  import type { GroupView } from "#lib/bindings.ts";
  import { group } from "#lib/stores/group.svelte.ts";
  import { progressive } from "#lib/progressive.svelte.ts";
  import { formatTime } from "#lib/format.ts";
  import { youtubeCover } from "#lib/covers.ts";
  import EmptyState from "#lib/components/ui/EmptyState.svelte";
  import Avatar from "#lib/components/ui/Avatar.svelte";
  import IconButton from "#lib/components/ui/IconButton.svelte";
  import TrackList from "#lib/components/ui/TrackList.svelte";
  import TrackCell from "#lib/components/ui/TrackCell.svelte";
  import TrackMain from "#lib/components/ui/TrackMain.svelte";
  import TrackRow from "#lib/components/ui/TrackRow.svelte";
  import GroupTrackStatus from "#lib/components/group/GroupTrackStatus.svelte";

  let { view }: { view: GroupView } = $props();

  const visible = progressive(() => view.state.queue.length);

  const queue = $derived(view.state.queue);
  const playing = $derived(view.state.playback.status === "playing");
</script>

{#if queue.length === 0}
  <EmptyState>
    <p>La file du groupe est vide.</p>
    <p class="muted">
      Ajoute des titres depuis
      <a href={resolve("/search")}>Rechercher</a>
      ou depuis les titres de ta
      <a href={resolve("/library")}>bibliothèque</a>
      téléchargés depuis YouTube.
    </p>
  </EmptyState>
{:else}
  <TrackList>
    {#each queue.slice(0, visible.count) as entry, i (entry.entry_id)}
      {@const current = view.state.current === i}
      <TrackRow
        index={i}
        cover={youtubeCover(entry.youtube_id)}
        {current}
        playing={current && playing}
        cols="minmax(0,2fr) minmax(0,1fr) 56px 28px 34px"
        colsSmall="minmax(0,1fr) 28px 34px"
        onplay={() => group.playEntry(entry.entry_id)}
      >
        <TrackMain title={entry.title} subtitle={entry.artist ?? "Artiste inconnu"} />
        <TrackCell kind="plain" hideSmall>
          <span class="by">
            <Avatar name={entry.added_by} size={22} />
            <TrackCell>{entry.added_by === view.me ? "toi" : entry.added_by}</TrackCell>
          </span>
        </TrackCell>
        <TrackCell kind="dur" hideSmall>
          {entry.duration !== null ? formatTime(entry.duration) : ""}
        </TrackCell>
        <GroupTrackStatus download={view.downloads[entry.youtube_id]} />
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <span class="remove" onclick={(e) => e.stopPropagation()}>
          <IconButton
            icon="close"
            label="Retirer de la file du groupe"
            size={34}
            iconSize={16}
            onclick={() => group.remove(entry.entry_id)}
          />
        </span>
      </TrackRow>
    {/each}
  </TrackList>
{/if}

<style>
  .by {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .remove {
    display: grid;
    place-items: center;
  }
</style>
