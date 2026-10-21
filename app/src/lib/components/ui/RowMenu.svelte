<script lang="ts" module>
  export type MenuItem = {
    label: string;
    onclick?: () => void;
    /** Opens a sub-list (with a back header). */
    submenu?: MenuItem[];
    /** Shows an input; `submit` receives the text. */
    prompt?: { placeholder: string; submit: (value: string) => void };
    danger?: boolean;
    disabled?: boolean;
    /** Trailing check (already done). */
    check?: boolean;
    /** Accent-colored (favorite on). */
    active?: boolean;
    hint?: string;
  };
</script>

<script lang="ts">
  /** "…" button + glass menu (fixed, rendered in <body> so lists never clip it). */
  import { tick } from "svelte";
  import { portal } from "#lib/actions.ts";
  import Icon from "#lib/components/ui/Icon.svelte";

  let { items, label = "Plus d'actions" }: { items: MenuItem[]; label?: string } = $props();

  const W = 230;
  let open = $state(false);
  let stack = $state<MenuItem[][]>([]);
  let titles = $state<string[]>([]);
  let prompting = $state<MenuItem | null>(null);
  let draft = $state("");
  let pos = $state({ top: 0, left: 0, maxHeight: 320 });
  let button = $state<HTMLButtonElement>();
  let menu = $state<HTMLElement>();
  let input = $state<HTMLInputElement>();

  const shown = $derived(stack.length ? stack[stack.length - 1] : items);

  function toggle(e: MouseEvent) {
    e.stopPropagation();
    if (open) return close();
    const r = button!.getBoundingClientRect();
    const below = window.innerHeight - r.bottom - 12;
    const above = r.top - 12;
    const up = below < 200 && above > below;
    const maxHeight = Math.min(340, up ? above : below);
    pos = {
      top: up
        ? Math.max(8, r.top - 4 - Math.min(maxHeight, 44 * (items.length + 1)))
        : r.bottom + 4,
      left: Math.max(8, Math.min(window.innerWidth - W - 8, r.right - W)),
      maxHeight,
    };
    open = true;
  }

  function close() {
    open = false;
    stack = [];
    titles = [];
    prompting = null;
    draft = "";
  }

  async function pick(item: MenuItem) {
    if (item.disabled) return;
    if (item.submenu) {
      stack = [...stack, item.submenu];
      titles = [...titles, item.label];
    } else if (item.prompt) {
      prompting = item;
      await tick();
      input?.focus();
    } else {
      close();
      item.onclick?.();
    }
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const value = draft.trim();
    if (!value || !prompting?.prompt) return;
    const { submit } = prompting.prompt;
    close();
    submit(value);
  }

  function onwindow(e: MouseEvent) {
    const t = e.target as Node;
    if (open && !menu?.contains(t) && !button?.contains(t)) close();
  }
</script>

<svelte:window onclick={onwindow} onkeydown={(e) => open && e.key === "Escape" && close()} />

<button
  bind:this={button}
  class="more halo-btn"
  class:open
  type="button"
  aria-label={label}
  aria-haspopup="menu"
  aria-expanded={open}
  title={label}
  onclick={toggle}
  ondblclick={(e) => e.stopPropagation()}
>
  <Icon name="more" size={16} />
</button>

{#if open}
  <div
    use:portal
    bind:this={menu}
    class="menu glass"
    role="menu"
    tabindex="-1"
    style:top="{pos.top}px"
    style:left="{pos.left}px"
    style:width="{W}px"
    style:max-height="{pos.maxHeight}px"
  >
    {#if stack.length}
      <button
        class="item back"
        type="button"
        onclick={() => {
          stack = stack.slice(0, -1);
          titles = titles.slice(0, -1);
          prompting = null;
        }}
      >
        <Icon name="chevronLeft" size={14} />
        <span>{titles[titles.length - 1]}</span>
      </button>
      <div class="sep"></div>
    {/if}
    {#each shown as item (item.label)}
      {#if prompting === item && item.prompt}
        <form onsubmit={submit}>
          <input
            bind:this={input}
            bind:value={draft}
            class="field"
            maxlength="100"
            placeholder={item.prompt.placeholder}
          />
        </form>
      {:else}
        <button
          class="item"
          class:danger={item.danger}
          class:active={item.active}
          type="button"
          role="menuitem"
          disabled={item.disabled}
          title={item.hint}
          onclick={() => pick(item)}
        >
          <span class="lbl">{item.label}</span>
          {#if item.check}<Icon name="check" size={14} />{/if}
          {#if item.submenu}<Icon name="chevronRight" size={14} />{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .more {
    width: 34px;
    height: 34px;
    color: var(--mute);
  }
  .more.open {
    color: var(--fg);
  }
  .menu {
    position: fixed;
    z-index: 60;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    padding: 6px;
    overflow-y: auto;
    border-radius: calc(var(--rs) + 6px);
    background: color-mix(in srgb, var(--bg) 84%, transparent);
    animation: pop 0.22s var(--ease-spring);
    transform-origin: top right;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 10px;
    border-radius: var(--rs);
    background: transparent;
    color: var(--fg);
    font-size: 14px;
    font-weight: 600;
    text-align: left;
    transition: background 0.2s;
  }
  .item:hover:not(:disabled) {
    background: var(--glass2);
  }
  .item:disabled {
    opacity: 0.45;
  }
  .item.danger {
    color: var(--danger);
  }
  .item.active {
    color: var(--accent);
  }
  .lbl {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--line);
  }
  form {
    padding: 2px;
  }
  .field {
    width: 100%;
    height: 38px;
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.96);
    }
  }
</style>
