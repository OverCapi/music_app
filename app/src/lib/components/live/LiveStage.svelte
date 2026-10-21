<script lang="ts">
  /** Picture of the live playing in the audio engine: hls.js on the `live://` relay, muted.
   * Fills its parent (the parent sets the size). The sound never comes from here; the picture
   * follows pause/resume and may lag a few seconds behind the sound. */
  import { onMount } from "svelte";
  import Hls from "hls.js";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { player } from "#lib/stores/player.svelte.ts";

  let { fit = "cover" }: { fit?: "cover" | "contain" } = $props();

  let video = $state<HTMLVideoElement>();
  const live = $derived(player.state.live);
  const playing = $derived(player.state.playing);

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

<video bind:this={video} muted autoplay playsinline style:object-fit={fit}></video>

<style>
  video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    background: #000;
  }
</style>
