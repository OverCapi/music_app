<script lang="ts">
  /** Text input in the glass `.field` style. Extra attributes (`type`, `placeholder`,
   * `aria-label`, `style`…) go to the `<input>`. Variants: `mono` (monospace), `code`
   * (invite code: grows, spaced uppercase), `num` (small centered number box), `search`
   * (narrow, for page headers), `menu` (full width, 38 px), `grow` (fills a flex row), `pill`
   * (full width, rounded). */
  let {
    value = $bindable(""),
    el = $bindable(),
    variant,
    ...rest
  }: {
    value?: string | number;
    el?: HTMLInputElement;
    variant?: "mono" | "code" | "num" | "search" | "menu" | "grow" | "pill";
    [key: string]: unknown;
  } = $props();
</script>

<input class="field {variant ?? ''}" bind:this={el} bind:value {...rest} />

<style>
  .mono {
    font-family: var(--mono);
  }
  .code {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .code::placeholder {
    text-transform: none;
    letter-spacing: normal;
  }
  .num {
    width: 76px;
    height: 38px;
    text-align: center;
    font-family: var(--mono);
  }
  .menu {
    width: 100%;
    height: 38px;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .pill {
    width: 100%;
    border-radius: 999px;
  }
  .search {
    width: 200px;
    max-width: 100%;
  }
</style>
