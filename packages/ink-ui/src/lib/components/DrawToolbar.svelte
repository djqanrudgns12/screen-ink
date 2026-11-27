<script>
  import { onDestroy } from 'svelte';
  import Icon from '../icons/Icon.svelte';
  import AnnotateGlyph from './AnnotateGlyph.svelte';
  import { t } from '../i18n/index.svelte.js';
  import { resolveInkColor } from '../ink/canvasRender.js';
  import { isLightColor } from '../models/palette.js';
  import {
    DRAW_TOOLBAR_BOTTOM_PX,
    DRAW_TOOLBAR_DIM_OPACITY,
    DRAW_TOOLBAR_HISTORY,
    DRAW_TOOLBAR_TOOLS,
    formatDrawTooltip,
    resolveBoardToggleAction,
  } from '../models/drawToolbar.js';

  /**
   * 그리기·칠판 아래 가운데 유리 알약 도구 막대(R2-DRAW-TB):
   * - 도구: 펜 · 형광펜 · 지우개 · 도형(직선 · 사각형 · 원) · 글자
   * - 색·굵기 단추(현재 색 점 + 굵기 숫자)
   * - 되돌리기 · 다시 하기
   * - 칠판 ↔ 화면 전환 · 다 지우기 · 닫기
   * @type {{
   *   activeTool: string,
   *   color?: string,
   *   sizeValue?: number,
   *   sizeUnit?: 'px' | 'pt',
   *   background?: 'frame' | 'white' | 'black',
   *   mode?: 'draw' | 'board' | 'capture',
   *   canUndo?: boolean,
   *   canRedo?: boolean,
   *   hasItems?: boolean,
   *   panelOpen?: boolean,
   *   dimmed?: boolean,
   *   interactive?: boolean,
   *   platform?: 'windows' | 'macos',
   *   combos?: Record<string, string>,
   *   onTool?: (tool: (typeof DRAW_TOOLBAR_TOOLS)[number]) => void,
   *   onStyleToggle?: () => void,
   *   onUndo?: () => void,
   *   onRedo?: () => void,
   *   onBoardToggle?: (nextBackground: 'frame' | 'white' | 'black') => void,
   *   onClear?: () => void,
   *   onClose?: () => void,
   *   onHover?: (over: boolean) => void,
   *   onStyleAnchorChange?: (centerX: number | null) => void,
   * }}
   */
  let {
    activeTool = 'pen',
    color = 'R',
    sizeValue = 4,
    sizeUnit = 'px',
    background = 'frame',
    mode = 'draw',
    canUndo = false,
    canRedo = false,
    hasItems = false,
    panelOpen = false,
    dimmed = false,
    interactive = true,
    platform = 'windows',
    combos = {},
    onTool = () => {},
    onStyleToggle = () => {},
    onUndo = () => {},
    onRedo = () => {},
    onBoardToggle = () => {},
    onClear = () => {},
    onClose = () => {},
    onHover = () => {},
    onStyleAnchorChange = () => {},
  } = $props();

  /** @type {HTMLDivElement | null} */
  let rootEl = $state(null);
  /** @type {HTMLButtonElement | null} */
  let styleBtnEl = $state(null);
  let barW = $state(0);

  /** @type {{ text: string, cx: number } | null} */
  let tip = $state(null);
  /** @type {ReturnType<typeof setTimeout> | null} */
  let tipTimer = null;

  /** @param {Event} e */
  function stop(e) {
    e.stopPropagation();
  }

  function clearTip() {
    if (tipTimer !== null) {
      clearTimeout(tipTimer);
      tipTimer = null;
    }
    tip = null;
  }

  /**
   * 0.35초 호버/포커스 지연 말풍선.
   * @param {HTMLElement | null} btn
   * @param {string} text
   */
  function scheduleTip(btn, text) {
    clearTip();
    if (!btn || !text) return;
    tipTimer = setTimeout(() => {
      tipTimer = null;
      const cx = btn.offsetLeft + btn.offsetWidth / 2;
      tip = { text, cx };
    }, 350);
  }

  onDestroy(clearTip);

  /** 현재 도구 막대 바깥 사각형(CSS px)을 돌려준다. */
  export function getBarRect() {
    return rootEl ? rootEl.getBoundingClientRect() : null;
  }

  let boardAction = $derived(resolveBoardToggleAction({ mode, background }));
  let dotColor = $derived(resolveInkColor(color, background));
  let dotLight = $derived(isLightColor(dotColor));
  let isMarker = $derived(activeTool === 'marker');
  let styleTip = $derived(t('draw.action.style'));
  let boardTip = $derived(
    formatDrawTooltip({ label: t(boardAction.labelKey), slot: boardAction.slot }, combos, platform),
  );
  let clearTipText = $derived(formatDrawTooltip({ label: t('draw.action.clear'), slot: 'd-clear' }, combos, platform));
  let closeTipText = $derived(formatDrawTooltip({ label: t('draw.action.close'), slot: 'd-exit' }, combos, platform));

  $effect(() => {
    void barW;
    if (styleBtnEl && rootEl) {
      const r = styleBtnEl.getBoundingClientRect();
      onStyleAnchorChange(r.left + r.width / 2);
    } else {
      onStyleAnchorChange(null);
    }
  });
</script>

<div
  bind:this={rootEl}
  bind:offsetWidth={barW}
  class="ink-draw-toolbar ink-draw-ui"
  role="toolbar"
  tabindex="-1"
  aria-label={t('draw.toolbar.label')}
  data-dimmed={dimmed ? 'true' : undefined}
  data-interactive={interactive ? 'true' : 'false'}
  style:bottom={`${DRAW_TOOLBAR_BOTTOM_PX}px`}
  style:opacity={dimmed ? DRAW_TOOLBAR_DIM_OPACITY : 1}
  onpointerenter={() => onHover(true)}
  onpointerleave={() => {
    onHover(false);
    clearTip();
  }}
  onpointerdown={stop}
  onpointermove={stop}
  onpointerup={stop}
  ondblclick={stop}
  onwheel={stop}
  oncontextmenu={(e) => {
    e.preventDefault();
    e.stopPropagation();
  }}
>
  {#if tip && !dimmed && !panelOpen}
    <div class="ink-draw-tip" role="tooltip" style:left={`${tip.cx}px`}>
      {tip.text}
    </div>
  {/if}

  {#each DRAW_TOOLBAR_TOOLS as tool (tool.id)}
    {@const active = activeTool === tool.id}
    {@const tipText = formatDrawTooltip(
      { label: t(tool.labelKey), slot: tool.slot, modKey: tool.modKey },
      combos,
      platform,
    )}
    <button
      type="button"
      class="ink-draw-btn"
      class:active
      data-tool={tool.id}
      aria-pressed={active}
      aria-label={tipText}
      onpointerenter={(e) => scheduleTip(e.currentTarget, tipText)}
      onpointerleave={clearTip}
      onfocus={(e) => scheduleTip(e.currentTarget, tipText)}
      onblur={clearTip}
      onclick={(e) => {
        e.stopPropagation();
        clearTip();
        onTool(tool);
      }}
    >
      <AnnotateGlyph name={tool.icon} size={18} filled={active} />
    </button>
  {/each}

  <span class="ink-draw-divider" aria-hidden="true"></span>

  <button
    bind:this={styleBtnEl}
    type="button"
    class="ink-draw-style-btn"
    class:active={panelOpen}
    data-action="style"
    aria-pressed={panelOpen}
    aria-expanded={panelOpen}
    aria-label={`${styleTip} (${sizeValue}${sizeUnit})`}
    onpointerenter={(e) => scheduleTip(e.currentTarget, styleTip)}
    onpointerleave={clearTip}
    onfocus={(e) => scheduleTip(e.currentTarget, styleTip)}
    onblur={clearTip}
    onclick={(e) => {
      e.stopPropagation();
      clearTip();
      onStyleToggle();
    }}
  >
    <span
      class="ink-draw-style-dot"
      class:light={dotLight}
      class:marker={isMarker}
      style:background={dotColor}
    ></span>
    <span class="ink-draw-style-size ink-mono">{sizeValue}{sizeUnit}</span>
  </button>

  <span class="ink-draw-divider" aria-hidden="true"></span>

  {#each DRAW_TOOLBAR_HISTORY as h (h.id)}
    {@const enabled = h.id === 'undo' ? canUndo : canRedo}
    {@const tipText = formatDrawTooltip({ label: t(h.labelKey), slot: h.slot }, combos, platform)}
    <button
      type="button"
      class="ink-draw-btn"
      data-action={h.id}
      disabled={!enabled}
      aria-label={tipText}
      onpointerenter={(e) => scheduleTip(e.currentTarget, tipText)}
      onpointerleave={clearTip}
      onfocus={(e) => scheduleTip(e.currentTarget, tipText)}
      onblur={clearTip}
      onclick={(e) => {
        e.stopPropagation();
        clearTip();
        if (h.id === 'undo') onUndo();
        else onRedo();
      }}
    >
      <Icon name={h.icon} size={18} accent={false} />
    </button>
  {/each}

  <span class="ink-draw-divider" aria-hidden="true"></span>

  <button
    type="button"
    class="ink-draw-btn"
    data-action="board-toggle"
    aria-label={boardTip}
    onpointerenter={(e) => scheduleTip(e.currentTarget, boardTip)}
    onpointerleave={clearTip}
    onfocus={(e) => scheduleTip(e.currentTarget, boardTip)}
    onblur={clearTip}
    onclick={(e) => {
      e.stopPropagation();
      clearTip();
      onBoardToggle(boardAction.nextBackground);
    }}
  >
    <Icon name={boardAction.icon} size={18} accent={false} />
  </button>

  <button
    type="button"
    class="ink-draw-btn"
    data-action="clear"
    disabled={!hasItems}
    aria-label={clearTipText}
    onpointerenter={(e) => scheduleTip(e.currentTarget, clearTipText)}
    onpointerleave={clearTip}
    onfocus={(e) => scheduleTip(e.currentTarget, clearTipText)}
    onblur={clearTip}
    onclick={(e) => {
      e.stopPropagation();
      clearTip();
      onClear();
    }}
  >
    <Icon name="trash" size={18} accent={false} />
  </button>

  <button
    type="button"
    class="ink-draw-btn"
    data-action="close"
    aria-label={closeTipText}
    onpointerenter={(e) => scheduleTip(e.currentTarget, closeTipText)}
    onpointerleave={clearTip}
    onfocus={(e) => scheduleTip(e.currentTarget, closeTipText)}
    onblur={clearTip}
    onclick={(e) => {
      e.stopPropagation();
      clearTip();
      onClose();
    }}
  >
    <Icon name="close" size={18} accent={false} />
  </button>
</div>

<style>
  .ink-draw-toolbar {
    --dtb-bg: var(--ink-dtb-bg);
    --dtb-fg: var(--ink-tb-fg);
    --dtb-sub: var(--ink-tb-sub);
    --dtb-hover: var(--ink-tb-hover);
    --dtb-active-bg: var(--ink-tb-active-bg);
    --dtb-divider: var(--ink-tb-divider);
    --dtb-border: var(--ink-tb-border);
    --dtb-shadow: var(--ink-tb-shadow);
    --dtb-tip-bg: var(--ink-tip-bg);
    --dtb-tip-fg: var(--ink-tip-fg);

    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 42px;
    padding: 0 5px;
    box-sizing: border-box;
    border-radius: 999px;
    background: var(--dtb-bg);
    color: var(--dtb-fg);
    border: 1px solid var(--dtb-border);
    box-shadow: var(--dtb-shadow);
    backdrop-filter: blur(22px) saturate(1.4);
    -webkit-backdrop-filter: blur(22px) saturate(1.4);
    cursor: default;
    pointer-events: auto;
    transition: opacity 180ms ease-out;
    z-index: 22;
  }

  .ink-draw-toolbar[data-interactive='false'] {
    pointer-events: none;
  }

  @media (prefers-reduced-transparency: reduce) {
    .ink-draw-toolbar {
      --dtb-bg: var(--ink-glass-solid);
      backdrop-filter: none;
      -webkit-backdrop-filter: none;
    }
  }

  .ink-draw-btn,
  .ink-draw-style-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--dtb-fg);
    cursor: pointer;
    transition:
      background-color 120ms ease-out,
      color 120ms ease-out;
  }

  .ink-draw-btn {
    width: 32px;
    border-radius: 8px;
  }

  .ink-draw-style-btn {
    gap: 6px;
    padding: 0 9px;
    border-radius: 999px;
  }

  .ink-draw-btn:hover,
  .ink-draw-style-btn:hover {
    background: var(--dtb-hover);
  }

  .ink-draw-btn:focus-visible,
  .ink-draw-style-btn:focus-visible {
    outline: 2px solid var(--ink-accent);
    outline-offset: 1px;
  }

  .ink-draw-btn:disabled {
    opacity: 0.32;
    cursor: default;
  }

  .ink-draw-btn:disabled:hover {
    background: transparent;
  }

  .ink-draw-btn.active,
  .ink-draw-style-btn.active {
    background: var(--dtb-active-bg);
    color: var(--ink-accent);
    box-shadow: inset 0 0 0 1.5px var(--ink-accent);
  }

  .ink-draw-style-dot {
    width: 12px;
    height: 12px;
    border-radius: 999px;
    flex: none;
    box-shadow: var(--ink-draw-style-dot-ring);
  }

  .ink-draw-style-dot.light {
    box-shadow: var(--ink-draw-style-dot-ring-light);
  }

  .ink-draw-style-dot.marker {
    opacity: 0.55;
  }

  .ink-draw-style-size {
    font-size: 11.5px;
    font-weight: 600;
    color: inherit;
    line-height: 1;
  }

  .ink-draw-divider {
    flex: none;
    width: 1px;
    height: 18px;
    margin: 0 3px;
    background: var(--dtb-divider);
  }

  .ink-draw-tip {
    position: absolute;
    bottom: calc(100% + 8px);
    transform: translateX(-50%);
    padding: 4px 8px;
    border-radius: 6px;
    background: var(--dtb-tip-bg);
    color: var(--dtb-tip-fg);
    font-size: 11px;
    font-weight: 500;
    line-height: 1.25;
    white-space: nowrap;
    pointer-events: none;
    box-shadow: var(--ink-tip-shadow);
    z-index: 10;
  }
</style>
