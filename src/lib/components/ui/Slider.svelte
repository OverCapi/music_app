<script lang="ts">
  let {
    value,
    max,
    step,
    label,
    disabled = false,
    oninput,
    onchange,
  }: {
    value: number;
    max: number;
    step: number;
    label: string;
    disabled?: boolean;
    /** While dragging. */
    oninput?: (value: number) => void;
    /** On release. */
    onchange?: (value: number) => void;
  } = $props();

  const fill = $derived(max > 0 ? (value / max) * 100 : 0);
</script>

<input
  type="range"
  min="0"
  {max}
  {step}
  {value}
  {disabled}
  oninput={(e) => oninput?.(Number(e.currentTarget.value))}
  onchange={(e) => onchange?.(Number(e.currentTarget.value))}
  style:--fill="{fill}%"
  aria-label={label}
/>

<style>
  input {
    flex: 1;
    min-width: 0;
    height: 4px;
    margin: 0;
    appearance: none;
    border-radius: 2px;
    background: linear-gradient(to right, var(--text) var(--fill), var(--track) var(--fill));
    cursor: pointer;
  }

  input:hover:not(:disabled) {
    background: linear-gradient(to right, var(--accent) var(--fill), var(--track) var(--fill));
  }

  input:disabled {
    cursor: default;
    opacity: 0.5;
  }

  input::-webkit-slider-thumb {
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text);
    opacity: 0;
  }

  input:hover:not(:disabled)::-webkit-slider-thumb {
    opacity: 1;
  }
</style>
