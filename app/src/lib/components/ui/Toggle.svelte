<script lang="ts">
  /** On/off switch (46×28) with a spring knob. */
  let {
    checked,
    onchange,
    label,
  }: { checked: boolean; onchange: (value: boolean) => void; label: string } = $props();
</script>

<button
  class="toggle"
  class:on={checked}
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    flex: none;
    width: 46px;
    height: 28px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--line);
    transition: background 0.3s;
  }
  .toggle.on {
    background: var(--accent);
  }
  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--fg);
    box-shadow: 0 2px 6px var(--shadow);
    transition:
      transform 0.45s var(--ease-spring),
      background 0.3s;
  }
  .on .knob {
    background: var(--on-accent);
    transform: translateX(18px);
  }
</style>
