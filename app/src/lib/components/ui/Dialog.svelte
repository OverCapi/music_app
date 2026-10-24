<script lang="ts">
  import type { Snippet } from "svelte";
  import Panel from "#lib/components/ui/Panel.svelte";

  /** Modal on a dimmed backdrop: title, content and an actions row. The backdrop and Escape
   * call `onclose`. */
  let {
    title,
    onclose,
    children,
    actions,
  }: { title: string; onclose: () => void; children: Snippet; actions: Snippet } = $props();
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="wrap">
  <button class="backdrop" type="button" aria-label="Fermer" onclick={onclose}></button>
  <Panel radius={14} class="dialog" role="dialog" aria-modal="true" aria-label={title}>
    <h2>{title}</h2>
    <div class="body">{@render children()}</div>
    <div class="actions">{@render actions()}</div>
  </Panel>
</div>

<style>
  .wrap {
    position: fixed;
    inset: 0;
    z-index: 45;
    display: grid;
    place-items: center;
    padding: 16px;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    background: oklch(0 0 0 / 0.45);
    animation: fade 0.25s;
  }
  :global(.dialog) {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 14px;
    width: min(420px, 100%);
    padding: 24px;
    animation: pop 0.35s var(--ease-spring);
  }
  h2 {
    margin: 0;
    font-size: 18px;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 14px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 10px;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      scale: 0.94;
    }
  }
</style>
