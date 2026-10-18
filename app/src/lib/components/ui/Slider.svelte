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
    height: 6px;
    margin: 0;
    appearance: none;
    border-radius: 3px;
    background: linear-gradient(to right, var(--accent) var(--fill), var(--track) var(--fill));
    cursor: pointer;
  }

  input:disabled {
    cursor: default;
    opacity: 0.5;
  }

  input::-webkit-slider-thumb {
    appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--text);
    box-shadow: 0 0 0 4px var(--accent-soft);
    opacity: 0;
    transition: opacity 0.12s;
  }

  input:hover:not(:disabled)::-webkit-slider-thumb,
  input:focus-visible::-webkit-slider-thumb {
    opacity: 1;
  }
</style>
