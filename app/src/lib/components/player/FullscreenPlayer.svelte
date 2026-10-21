<script lang="ts">
  /** Fullscreen player: veil over the app (blur 50px), cover left (blurred glow behind,
   * breathes 2.2 s while playing), title + like + progress + transport right,
   * 48 mirrored visualizer bars along the bottom that rise in a wave on open. */
  import { onMount } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { player } from "#lib/stores/player.svelte.ts";
  import { library } from "#lib/stores/library.svelte.ts";
  import { formatTime } from "#lib/format.ts";
  import { liquid } from "#lib/motion.ts";
  import Icon from "#lib/components/ui/Icon.svelte";
  import Visualizer from "#lib/components/ui/Visualizer.svelte";

  let { onclose }: { onclose: () => void } = $props();
  let root = $state<HTMLElement>();

  const s = $derived(player.state);
  const track = $derived(s.track_id !== null ? library.byId.get(s.track_id) : undefined);
  const cover = $derived(
    s.track_id !== null ? convertFileSrc(String(s.track_id), "cover") : undefined,
  );

  let now = $state(Date.now());
  $effect(() => {
    if (!s.playing) return;
    const t = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(t);
  });
  const pos = $derived(Math.min(player.positionAt(now), player.duration || Infinity));

  onMount(() => {
    root!.animate(
      [
        {
          opacity: 0,
          transform: "translateY(60px) scale(.96)",
          borderRadius: "40px",
          filter: "blur(8px)",
        },
        { opacity: 1, transform: "none", borderRadius: "0px", filter: "blur(0px)" },
      ],
      { duration: 520, easing: "cubic-bezier(.2,.85,.25,1)" },
    );
    root!.querySelectorAll<HTMLElement>(".art, button").forEach((n, i) =>
      n.animate(
        [
          { opacity: 0, transform: "translateY(16px)" },
          { opacity: 1, transform: "none" },
        ],
        {
          duration: 420,
          delay: 120 + Math.min(i * 12, 260),
          easing: "cubic-bezier(.2,.85,.25,1)",
          fill: "backwards",
        },
      ),
    );
  });

  function close() {
    root!.animate(
      [
        { opacity: 1, transform: "none", filter: "blur(0px)", borderRadius: "0px" },
        {
          opacity: 0,
          transform: "translateY(40px) scale(.97)",
          filter: "blur(8px)",
          borderRadius: "32px",
        },
      ],
      { duration: 320, easing: "cubic-bezier(.4,0,.2,1)", fill: "forwards" },
    ).onfinish = onclose;
  }

  function seek(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    player.seek(((e.clientX - r.left) / r.width) * player.duration);
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && close()} />

<div class="full" bind:this={root}>
  <div class="viz">
    <Visualizer playing={s.playing} bars={48} gap={6} color="var(--fg)" intro />
  </div>
  <button class="halo-btn back glass" use:liquid onclick={close} aria-label="Fermer"
    ><Icon name="chevronDown" size={20} /></button
  >
  <div class="grid">
    <div class="art-wrap">
      {#if cover}<img class="glow" src={cover} alt="" />{/if}
      <div class="art" class:breathe={s.playing}>
        {#if cover}<img src={cover} alt="" />{/if}
      </div>
    </div>
    <div class="side">
      <div class="title">{track?.title ?? ""}</div>
      <div class="muted sub">{track?.artist ?? ""} · {track?.album ?? ""}</div>
      <button class="bar" onclick={seek} aria-label="Position">
        <span class="rail"
          ><span class="fill" style:width="{player.duration ? (pos / player.duration) * 100 : 0}%"
          ></span></span
        >
      </button>
      <div class="times mono muted">
        <span>{formatTime(pos)}</span><span>{formatTime(player.duration)}</span>
      </div>
      <div class="controls">
        <button
          class="halo-btn"
          class:active={s.shuffle}
          use:liquid
          onclick={() => player.toggleShuffle()}><Icon name="shuffle" size={20} /></button
        >
        <button class="halo-btn big" use:liquid onclick={() => player.previous()}
          ><Icon name="previous" size={28} /></button
        >
        <button
          class="play-btn"
          style="width:84px;height:84px"
          use:liquid
          onclick={() => player.togglePlay()}
          ><Icon name={s.playing ? "pause" : "play"} size={34} /></button
        >
        <button class="halo-btn big" use:liquid onclick={() => player.next()}
          ><Icon name="next" size={28} /></button
        >
        <button
          class="halo-btn"
          class:active={s.repeat !== "off"}
          use:liquid
          onclick={() => player.cycleRepeat()}
          ><Icon name={s.repeat === "one" ? "repeatOne" : "repeat"} size={20} /></button
        >
      </div>
    </div>
  </div>
</div>

<style>
  .full {
    position: absolute;
    inset: 0;
    z-index: 30;
    overflow: hidden;
    background: var(--veil);
    backdrop-filter: blur(50px) saturate(170%);
    -webkit-backdrop-filter: blur(50px) saturate(170%);
  }
  .viz {
    position: absolute;
    left: 48px;
    right: 48px;
    bottom: 24px;
    height: 120px;
    opacity: 0.5;
    pointer-events: none;
  }
  .back {
    position: absolute;
    z-index: 2;
    top: 22px;
    left: 24px;
    width: 44px;
    height: 44px;
  }
  .grid {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 72px;
    align-items: center;
    height: 100%;
    box-sizing: border-box;
    padding: 80px 96px 120px;
  }
  .art-wrap {
    position: relative;
    justify-self: end;
    width: 100%;
    max-width: 440px;
  }
  .glow {
    position: absolute;
    inset: -8%;
    width: 116%;
    height: 116%;
    border-radius: var(--r);
    object-fit: cover;
    filter: blur(50px);
    opacity: 0.75;
  }
  .art {
    position: relative;
    aspect-ratio: 1;
    overflow: hidden;
    border-radius: var(--r);
    background: var(--line);
    box-shadow:
      0 40px 80px -24px var(--shadow),
      inset 0 1px 0 var(--hi);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .breathe {
    animation: breathe 2.2s ease-in-out infinite alternate;
  }
  @keyframes breathe {
    to {
      transform: scale(1.035);
    }
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 460px;
  }
  .title {
    font-size: 46px;
    font-weight: 700;
    letter-spacing: -0.04em;
    line-height: 1.02;
    text-wrap: pretty;
  }
  .sub {
    margin-bottom: 20px;
    font-size: 17px;
  }
  .bar {
    height: 18px;
    padding: 0;
    background: none;
  }
  .rail {
    position: relative;
    display: block;
    height: 6px;
    border-radius: 6px;
    background: var(--line);
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 6px;
    background: var(--accent);
    box-shadow: 0 0 14px var(--accent);
    transition: width 0.3s linear;
  }
  .times {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
  }
  .controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 20px;
  }
  .halo-btn {
    width: 44px;
    height: 44px;
  }
  @media (max-width: 900px) {
    .viz {
      left: 16px;
      right: 16px;
      height: 80px;
    }
    .grid {
      grid-template-columns: 1fr;
      gap: 28px;
      align-content: center;
      padding: 76px 28px 100px;
      overflow-y: auto;
    }
    .art-wrap {
      justify-self: center;
      max-width: min(340px, 56vh);
    }
    .side {
      max-width: 520px;
      margin: 0 auto;
      width: 100%;
    }
    .title {
      font-size: clamp(26px, 7vw, 46px);
    }
    .sub {
      margin-bottom: 8px;
      font-size: 15px;
    }
  }
  .halo-btn.big {
    width: 56px;
    height: 56px;
  }
</style>
