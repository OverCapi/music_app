<script lang="ts">
  import type { MixTrack } from "#lib/bindings.ts";
  import { progressive } from "#lib/progressive.svelte.ts";
  import { formatTime } from "#lib/format.ts";
  import { youtubeCover } from "#lib/covers.ts";
  import TrackList from "#lib/components/ui/TrackList.svelte";
  import TrackRow from "#lib/components/ui/TrackRow.svelte";

  /** Rows for YouTube suggestions (mixes, discoveries): played on click, downloaded on play. */
  let {
    tracks,
    onplay,
    showKnown = true,
  }: {
    tracks: MixTrack[];
    /** Plays the list starting at `index`. */
    onplay: (index: number) => void;
    /** Marks tracks the user already knows. */
    showKnown?: boolean;
  } = $props();

  const visible = progressive(() => tracks.length);
</script>

<TrackList>
  {#each tracks.slice(0, visible.count) as track, i (`${i}-${track.youtube_id}`)}
    <TrackRow
      index={i}
      cover={youtubeCover(track.youtube_id)}
      cols="minmax(0,2fr) minmax(0,1.4fr) 56px"
      colsSmall="minmax(0,1fr) 48px"
      onplay={() => onplay(i)}
    >
      <span class="tr-main">
        <span class="tr-title">{track.title}</span>
        <span class="tr-sub">{track.artist ?? "Artiste inconnu"}</span>
      </span>
      <span class="tr-sub hide-sm">{showKnown && track.known ? "Déjà connu" : ""}</span>
      <span class="tr-dur">{track.duration ? formatTime(track.duration) : ""}</span>
    </TrackRow>
  {/each}
</TrackList>
