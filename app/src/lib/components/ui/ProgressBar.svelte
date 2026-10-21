<script lang="ts">
  /** 4 px bar; `value` 0..1, or `null` for an indeterminate sweep. */
  let { value }: { value: number | null } = $props();
</script>

<div
  class="bar"
  role="progressbar"
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={value === null ? undefined : Math.round(value * 100)}
>
  <div
    class="fill"
    class:indeterminate={value === null}
    style:width="{(value ?? 0.3) * 100}%"
  ></div>
</div>

<style>
  .bar {
    position: relative;
    height: 4px;
    overflow: hidden;
    border-radius: 4px;
    background: var(--line);
  }
  .fill {
    height: 100%;
    border-radius: 4px;
    background: var(--accent);
    box-shadow: 0 0 10px var(--accent);
    transition: width 0.4s var(--ease-out);
  }
  .indeterminate {
    animation: sweep 1.3s ease-in-out infinite;
  }
  @keyframes sweep {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
</style>
