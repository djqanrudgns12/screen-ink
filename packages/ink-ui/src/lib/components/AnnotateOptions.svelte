<script>
  import { t } from '../i18n/index.svelte.js';
  import { INK_COLORS, resolvePaletteHex } from '../models/palette.js';

  /**
   * 캡처 주석 보조 옵션(B §2.1): 하위 호환용 경량 알약.
   * @type {{
   *   kind: 'stroke' | 'mosaic',
   *   colors: ReadonlyArray<string>,
   *   widths: ReadonlyArray<{ px: number, label: string }>,
   *   blocks: ReadonlyArray<{ px: number, label: string }>,
   *   color?: string,
   *   width?: number,
   *   block?: number,
   *   x?: number,
   *   y?: number,
   *   onColor?: (code: string) => void,
   *   onWidth?: (px: number) => void,
   *   onBlock?: (px: number) => void,
   *   onHover?: (over: boolean) => void,
   *   boxWidth?: number,
   *   boxHeight?: number,
   * }}
   */
  let {
    kind,
    colors,
    widths,
    blocks,
    color = 'R',
    width = 4,
    block = 10,
    x = 0,
    y = 0,
    onColor = () => {},
    onWidth = () => {},
    onBlock = () => {},
    onHover = () => {},
    boxWidth = $bindable(0),
    boxHeight = $bindable(0),
  } = $props();

  /** @param {Event} e */
  function stop(e) {
    e.stopPropagation();
  }

  /** @param {string} c */
  function colorLabel(c) {
    return c in INK_COLORS ? t(`capture.color.${c}`) : c;
  }

  let wheelHint = $derived(t('capture.opt.wheel'));
</script>

<div
  class="ink-annot-opts"
  role="toolbar"
  tabindex="-1"
  aria-label={kind === 'mosaic' ? t('capture.opt.mosaicLabel') : t('capture.opt.strokeLabel')}
  style:transform={`translate3d(${x}px, ${y}px, 0)`}
  bind:offsetWidth={boxWidth}
  bind:offsetHeight={boxHeight}
  onpointerenter={() => onHover(true)}
  onpointerleave={() => onHover(false)}
  onpointerdown={stop}
  onpointermove={stop}
  onpointerup={stop}
  ondblclick={stop}
  oncontextmenu={(e) => {
    e.preventDefault();
    e.stopPropagation();
  }}
>
  {#if kind === 'stroke'}
    {#each colors as c (c)}
      <button
        type="button"
        class="ink-annot-opt"
        class:active={color === c}
        data-color={c}
        aria-pressed={color === c}
        title={colorLabel(c)}
        aria-label={colorLabel(c)}
        onclick={(e) => {
          e.stopPropagation();
          onColor(c);
        }}
      >
        <span class="ink-annot-dot" style:background={resolvePaletteHex(c)}></span>
      </button>
    {/each}
    <span class="ink-annot-sep" aria-hidden="true"></span>
    {#each widths as w (w.px)}
      <button
        type="button"
        class="ink-annot-opt"
        class:active={width === w.px}
        data-width={w.px}
        aria-pressed={width === w.px}
        title={`${w.label} ${w.px}px · ${wheelHint}`}
        aria-label={`${w.label} ${w.px}px`}
        onclick={(e) => {
          e.stopPropagation();
          onWidth(w.px);
        }}
      >
        <span class="ink-annot-line" style:height={`${Math.min(8, w.px)}px`}></span>
      </button>
    {/each}
  {:else}
    {#each blocks as b (b.px)}
      <button
        type="button"
        class="ink-annot-opt"
        class:active={block === b.px}
        data-block={b.px}
        aria-pressed={block === b.px}
        title={`${b.label} ${b.px}px · ${wheelHint}`}
        aria-label={`${b.label} ${b.px}px`}
        onclick={(e) => {
          e.stopPropagation();
          onBlock(b.px);
        }}
      >
        <span class="ink-annot-cell" style:width={`${4 + b.px / 2}px`} style:height={`${4 + b.px / 2}px`}></span>
      </button>
    {/each}
    <span class="ink-annot-value ink-mono" aria-live="polite">{block}px</span>
  {/if}
</div>

<style>
  .ink-annot-opts {
    position: absolute;
    left: 0;
    top: 0;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 28px;
    padding: 0 4px;
    box-sizing: border-box;
    border-radius: var(--ink-r-island);
    background: var(--ink-island);
    color: var(--ink-island-text);
    box-shadow: var(--ink-island-shadow);
    cursor: default;
    pointer-events: auto;
    will-change: transform;
    z-index: 5;
  }

  .ink-annot-opt {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--ink-island-text);
    cursor: pointer;
  }

  .ink-annot-opt:hover {
    background: var(--ink-island-key-bg);
  }

  .ink-annot-opt:focus-visible {
    outline: 2px solid var(--ink-accent);
    outline-offset: 1px;
  }

  .ink-annot-opt.active {
    box-shadow: inset 0 0 0 1.5px var(--ink-accent);
  }

  .ink-annot-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.5);
  }

  .ink-annot-line {
    width: 14px;
    border-radius: 999px;
    background: currentColor;
  }

  .ink-annot-cell {
    border-radius: 2px;
    background: currentColor;
    opacity: 0.8;
  }

  .ink-annot-sep {
    flex: none;
    width: 1px;
    height: 14px;
    margin: 0 3px;
    background: var(--ink-island-key-bg);
  }

  .ink-annot-value {
    min-width: 34px;
    padding: 0 4px;
    font-size: 11px;
    text-align: right;
    color: var(--ink-island-text-2);
  }
</style>
