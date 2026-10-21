<script lang="ts">
  import type { LiveChannel } from "#lib/bindings.ts";
  import { liquid } from "#lib/motion.ts";

  /** Round channel avatar (96 px) with name and handle / followers underneath. */
  let { channel, sub, onclick }: { channel: LiveChannel; sub?: string; onclick: () => void } =
    $props();
</script>

<button class="channel" type="button" data-liquid use:liquid {onclick}>
  <span class="avatar"><img src={channel.thumbnail} alt="" loading="lazy" /></span>
  <span class="name">{channel.name}</span>
  {#if sub}<span class="sub">{sub}</span>{/if}
</button>

<style>
  .channel {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    min-width: 0;
    background: transparent;
    color: var(--fg);
    cursor: pointer;
    transition: transform 0.45s var(--ease-spring);
  }
  .channel:hover {
    transform: translateY(-5px);
  }
  .avatar {
    width: 96px;
    height: 96px;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 50%;
    box-shadow:
      inset 0 1px 0 var(--hi),
      0 20px 40px -18px var(--shadow);
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .name {
    max-width: 100%;
    overflow: hidden;
    font-size: 14px;
    font-weight: 600;
    text-align: center;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    margin-top: -4px;
    color: var(--mute);
    font-size: 12px;
  }
</style>
