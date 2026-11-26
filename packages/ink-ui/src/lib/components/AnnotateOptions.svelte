<script>
  import { INK_COLORS } from '../ink/inkEngine.js';

  /**
   * 캡처 주석 보조 옵션(B §2.1): 도구를 고른 동안만 툴바 바깥쪽 4px에 붙는 작은 알약.
   * - 펜·형광펜·도형·글자: 색 6(R·Y·G·O·B·L) + 굵기 3단(2·4·8 px, Ctrl+휠로 미세 조절)
   * - 모자이크: 칸 3단(6·10·16 px, Ctrl+휠로 6~24) + 지금 값
   * 포인터 사건은 오버레이로 가지 않는다(범위·주석을 건드리지 않음). 위치는 부모가 `placeToolOptions`로 정한다.
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

  /** 색 이름(툴팁) */
  const COLOR_NAMES = /** @type {Record<string, string>} */ ({ R: '빨강', Y: '노랑', G: '초록', O: '주황', B: '파랑', L: '연두' });

  /** @param {Event} e */
  function stop(e) {
    e.stopPropagation();
  }
</script>

<div
  class="ink-annot-opts"
  role="toolbar"
  tabindex="-1"
  aria-label={kind === 'mosaic' ? '모자이크 칸 크기' : '색과 굵기'}
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
        title={COLOR_NAMES[c] ?? c}
        aria-label={COLOR_NAMES[c] ?? c}
        onclick={(e) => {
          e.stopPropagation();
          onColor(c);
        }}
      >
        <span class="ink-annot-dot" style:background={/** @type {Record<string, string>} */ (INK_COLORS)[c]}></span>
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
        title={`${w.label} ${w.px}px · Ctrl+휠`}
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
        title={`${b.label} ${b.px}px · Ctrl+휠`}
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
