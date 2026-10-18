<script lang="ts">
  import { onMount } from "svelte";
  import Hls from "hls.js";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { lives } from "#lib/stores/lives.svelte.ts";
  import { player } from "#lib/stores/player.svelte.ts";

  let video = $state<HTMLVideoElement>();
  const live = $derived(player.state.live);
  const playing = $derived(player.state.playing);

  // The live ended (stopped, or another track started): nothing left to show.
  $effect(() => {
    if (!live) lives.videoOpen = false;
  });

  // The sound comes from the audio engine; this is only the picture (muted), which
  // follows pause/resume. It may lag a few seconds behind the sound.
  $effect(() => {
    if (!video) return;
    if (playing) video.play().catch(() => {});
    else video.pause();
  });

  onMount(() => {
    const id = live?.id;
    if (!id || !video) return;
    const el = video;
    // The `live://` scheme relays YouTube's HLS with CORS headers (see `lives/proxy.rs`).
    const hls = new Hls({ liveSyncDurationCount: 3 });
    hls.on(Hls.Events.MANIFEST_PARSED, () => {
      if (player.state.playing) el.play().catch(() => {});
    });
    hls.on(Hls.Events.ERROR, (_, data) => {
      if (!data.fatal) return;
      if (data.type === Hls.ErrorTypes.NETWORK_ERROR) hls.startLoad();
      else if (data.type === Hls.ErrorTypes.MEDIA_ERROR) hls.recoverMediaError();
      else console.error("live video", data);
    });
    hls.loadSource(convertFileSrc(`${id}.m3u8`, "live"));
    hls.attachMedia(el);
    return () => hls.destroy();
  });
</script>

<section class="view">
  <div class="stage">
    <video bind:this={video} muted autoplay playsinline></video>
  </div>
  <p class="title">{live?.title ?? ""}</p>
</section>

<style>
  .view {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 0.75rem;
    overflow: hidden;
  }

  .stage {
    display: grid;
    flex: 1;
    min-width: 0;
    min-height: 0;
    place-items: center;
    overflow: hidden;
    container-type: size;
  }

  /* Largest 16:9 box inside the stage, whatever the window size: no black bars, and
     the stream's intrinsic size can't make it overflow when the window shrinks. */
  video {
    width: min(100cqw, 100cqh * 16 / 9);
    aspect-ratio: 16 / 9;
    border-radius: 16px;
    box-shadow: var(--shadow-float);
    object-fit: cover;
  }

  .title {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 700;
  }
</style>
