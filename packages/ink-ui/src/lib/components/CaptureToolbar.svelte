<script>
  import { onDestroy } from 'svelte';
  import Icon from '../icons/Icon.svelte';
  import AnnotateGlyph from './AnnotateGlyph.svelte';
  import { t } from '../i18n/index.svelte.js';
  import { buildFontFamilyStack, compactFontLabel, normalizeFontWeight } from '../models/fontPicker.js';
  import { resolvePaletteHex } from '../models/palette.js';

  /**
   * 선택 툴바(R2-UI-TB, 04 S08): 확정한 범위 옆 유리 알약.
   * 묶음(왼→오): [그리기 도구 8종 │ 색·굵기 단추 │ 되돌리기·다시 하기 │ 닫기·고정·저장·복사(주황)].
   * 단추 32×32, 아이콘 18px, 호버/초점 0.4초 말풍선("사각형 · R").
   * @type {{
   *   actions: ReadonlyArray<{ id: string, icon: string, label: string, labelKey?: string, command: string, primary?: boolean }>,
   *   tools?: ReadonlyArray<{ id: string, icon: string, label: string, labelKey?: string, key?: string }>,
   *   history?: ReadonlyArray<{ id: string, icon: string, label: string, labelKey?: string, command: string }>,
   *   activeTool?: string | null,
   *   panelOpen?: boolean,
   *   activeColor?: string,
   *   activeSize?: number,
   *   activeFont?: string,
   *   activeFontWeight?: number,
   *   isMarker?: boolean,
   *   canUndo?: boolean,
   *   canRedo?: boolean,
   *   tooltips?: Record<string, string>,
   *   x?: number,
   *   y?: number,
   *   onCommand?: (command: string) => void,
   *   onTool?: (id: string) => void,
   *   onStyleToggle?: () => void,
   *   onAnchorChange?: (anchorX: number | null) => void,
   *   onHover?: (over: boolean) => void,
   *   width?: number,
   *   height?: number,
   * }}
   */
  let {
    actions,
    tools = [],
    history = [],
    activeTool = null,
    panelOpen = false,
    activeColor = 'R',
    activeSize = 4,
    activeFont = '',
    activeFontWeight = 400,
    isMarker = false,
    canUndo = false,
    canRedo = false,
    tooltips = {},
    x = 0,
    y = 0,
    onCommand = () => {},
    onTool = () => {},
    onStyleToggle = () => {},
    onAnchorChange = () => {},
    onHover = () => {},
    width = $bindable(0),
    height = $bindable(0),
  } = $props();

  /** @type {HTMLDivElement | null} */
  let rootEl = $state(null);
  /** @type {HTMLButtonElement | null} */
  let styleBtnEl = $state(null);
  /** @type {Record<string, HTMLButtonElement | null>} */
  let toolEls = {};

  /** @type {{ text: string, cx: number } | null} */
  let tip = $state(null);
  /** @type {ReturnType<typeof setTimeout> | null} */
  let tipTimer = null;

  /** @param {Event} e */
  function stop(e) {
    e.stopPropagation();
  }

  /** @param {string} id */
  function historyEnabled(id) {
    return id === 'undo' ? canUndo : id === 'redo' ? canRedo : true;
  }

  function clearTip() {
    if (tipTimer !== null) {
      clearTimeout(tipTimer);
      tipTimer = null;
    }
    tip = null;
  }

  /**
   * 0.4초 호버/포커스 지연 말풍선(R2-UI-TB-3).
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
    }, 400);
  }

  onDestroy(clearTip);

  let previewColor = $derived(resolvePaletteHex(activeColor));
  let previewDotPx = $derived(Math.max(4, Math.min(14, Math.round(3 + Math.sqrt(Math.max(1, activeSize)) * 1.4))));
  let isTextTool = $derived(activeTool === 'text');
  let styleLabel = $derived(
    isTextTool && activeFont
      ? `${tooltips.style ?? t('capture.action.style')} · ${compactFontLabel(activeFont, activeFontWeight)}`
      : tooltips.style ?? t('capture.action.style'),
  );

  // 고른 도구 단추(없으면 색·굵기 단추)의 툴바 안 가운데 X를 부모에 알려 패널 자리를 맞춘다(R2-FIX-11).
  $effect(() => {
    void width;
    const target = (activeTool && toolEls[activeTool]) || styleBtnEl || null;
    if (target && rootEl) {
      onAnchorChange(target.offsetLeft + target.offsetWidth / 2);
    } else {
      onAnchorChange(null);
    }
  });
</script>

<div
  bind:this={rootEl}
  class="ink-toolbar"
  role="toolbar"
  tabindex="-1"
  aria-label={t('capture.toolbar.label')}
  style:transform={`translate3d(${x}px, ${y}px, 0)`}
  bind:offsetWidth={width}
  bind:offsetHeight={height}
  onpointerenter={() => onHover(true)}
  onpointerleave={() => {
    onHover(false);
    clearTip();
  }}
  onpointerdown={stop}
  onpointermove={stop}
  onpointerup={stop}
  ondblclick={stop}
  oncontextmenu={(e) => {
    e.preventDefault();
    e.stopPropagation();
  }}
>
  {#if tip}
    <div class="ink-toolbar-tip" role="tooltip" style:left={`${tip.cx}px`}>
      {tip.text}
    </div>
  {/if}

  {#if tools.length}
    {#each tools as tool (tool.id)}
      {@const label = tool.labelKey ? t(tool.labelKey) : tool.label}
      {@const tipText = tooltips[tool.id] ?? (tool.key ? `${label} · ${tool.key}` : label)}
      <button
        bind:this={toolEls[tool.id]}
        type="button"
        class="ink-toolbar-btn"
        class:active={activeTool === tool.id}
        data-tool={tool.id}
        aria-pressed={activeTool === tool.id}
        aria-label={tipText}
        onpointerenter={(e) => scheduleTip(e.currentTarget, tipText)}
        onpointerleave={clearTip}
        onfocus={(e) => scheduleTip(e.currentTarget, tipText)}
        onblur={clearTip}
        onclick={(e) => {
          e.stopPropagation();
          clearTip();
          onTool(tool.id);
        }}
      >
        <AnnotateGlyph name={tool.icon} size={18} filled={activeTool === tool.id} />
      </button>
    {/each}
    <span class="ink-toolbar-divider" aria-hidden="true"></span>
  {/if}

  <button
    bind:this={styleBtnEl}
    type="button"
    class="ink-toolbar-btn ink-toolbar-style"
    class:active={panelOpen}
    data-action="style"
    aria-pressed={panelOpen}
    aria-expanded={panelOpen}
    aria-label={styleLabel}
    onpointerenter={(e) => scheduleTip(e.currentTarget, styleLabel)}
    onpointerleave={clearTip}
    onfocus={(e) => scheduleTip(e.currentTarget, styleLabel)}
    onblur={clearTip}
    onclick={(e) => {
      e.stopPropagation();
      clearTip();
      onStyleToggle();
    }}
  >
    {#if isTextTool && activeFont}
      <span
        class="ink-toolbar-style-font"
        style:font-family={buildFontFamilyStack(activeFont)}
        style:font-weight={normalizeFontWeight(activeFontWeight)}
        style:color={previewColor}
      >
        Aa
      </span>
    {:else}
      <span
        class="ink-toolbar-style-dot"
        class:marker={isMarker}
        style:width={`${previewDotPx}px`}
        style:height={`${previewDotPx}px`}
        style:background={previewColor}
      ></span>
    {/if}
  </button>

  {#if history.length}
    <span class="ink-toolbar-divider" aria-hidden="true"></span>
    {#each history as h (h.id)}
      {@const label = h.labelKey ? t(h.labelKey) : h.label}
      {@const tipText = tooltips[h.id] ?? label}
      <button
        type="button"
        class="ink-toolbar-btn"
        data-action={h.id}
        disabled={!historyEnabled(h.id)}
        aria-label={tipText}
        onpointerenter={(e) => scheduleTip(e.currentTarget, tipText)}
        onpointerleave={clearTip}
        onfocus={(e) => scheduleTip(e.currentTarget, tipText)}
        onblur={clearTip}
        onclick={(e) => {
          e.stopPropagation();
          clearTip();
          onCommand(h.command);
        }}
      >
        <AnnotateGlyph name={h.icon} size={18} />
      </button>
    {/each}
  {/if}

  <span class="ink-toolbar-divider" aria-hidden="true"></span>
  {#each actions as a (a.id)}
    {@const label = a.labelKey ? t(a.labelKey) : a.label}
    {@const tipText = tooltips[a.id] ?? label}
    <button
      type="button"
      class="ink-toolbar-btn"
      class:primary={a.primary}
      data-action={a.id}
      aria-label={tipText}
      onpointerenter={(e) => scheduleTip(e.currentTarget, tipText)}
      onpointerleave={clearTip}
      onfocus={(e) => scheduleTip(e.currentTarget, tipText)}
      onblur={clearTip}
      onclick={(e) => {
        e.stopPropagation();
        clearTip();
        onCommand(a.command);
      }}
    >
      <Icon name={a.icon} size={18} accent={false} />
    </button>
  {/each}
</div>

<style>
  .ink-toolbar {
    --tb-bg: rgba(20, 21, 25, 0.8);
    --tb-fg: #f7f8fa;
    --tb-hover: rgba(255, 255, 255, 0.12);
    --tb-active-bg: rgba(255, 159, 10, 0.22);
    --tb-divider: rgba(255, 255, 255, 0.14);
    --tb-border: rgba(255, 255, 255, 0.14);
    --tb-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.1), 0 14px 36px rgba(0, 0, 0, 0.42);
    --tb-tip-bg: rgba(14, 15, 18, 0.94);
    --tb-tip-fg: #f7f8fa;
    --tb-dot-ring: rgba(255, 255, 255, 0.45);

    position: absolute;
    left: 0;
    top: 0;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 40px;
    padding: 0 4px;
    box-sizing: border-box;
    border-radius: var(--ink-r-island);
    background: var(--tb-bg);
    color: var(--tb-fg);
    border: 1px solid var(--tb-border);
    box-shadow: var(--tb-shadow);
    backdrop-filter: blur(22px) saturate(1.4);
    -webkit-backdrop-filter: blur(22px) saturate(1.4);
    cursor: default;
    pointer-events: auto;
    will-change: transform;
    z-index: 6;
  }

  :global([data-theme='light']) .ink-toolbar {
    --tb-bg: rgba(250, 250, 248, 0.86);
    --tb-fg: #141414;
    --tb-hover: rgba(0, 0, 0, 0.06);
    --tb-active-bg: rgba(255, 159, 10, 0.16);
    --tb-divider: rgba(0, 0, 0, 0.12);
    --tb-border: rgba(0, 0, 0, 0.12);
    --tb-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.8), 0 12px 32px rgba(0, 0, 0, 0.16);
    --tb-tip-bg: rgba(24, 25, 28, 0.94);
    --tb-tip-fg: #f7f8fa;
    --tb-dot-ring: rgba(0, 0, 0, 0.25);
  }

  @media (prefers-color-scheme: light) {
    :global(:root:not([data-theme='dark'])) .ink-toolbar {
      --tb-bg: rgba(250, 250, 248, 0.86);
      --tb-fg: #141414;
      --tb-hover: rgba(0, 0, 0, 0.06);
      --tb-active-bg: rgba(255, 159, 10, 0.16);
      --tb-divider: rgba(0, 0, 0, 0.12);
      --tb-border: rgba(0, 0, 0, 0.12);
      --tb-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.8), 0 12px 32px rgba(0, 0, 0, 0.16);
      --tb-dot-ring: rgba(0, 0, 0, 0.25);
    }
  }

  @media (prefers-reduced-transparency: reduce) {
    .ink-toolbar {
      --tb-bg: #18191d;
      backdrop-filter: none;
      -webkit-backdrop-filter: none;
    }
    :global([data-theme='light']) .ink-toolbar {
      --tb-bg: #fafaf8;
    }
  }

  .ink-toolbar-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--tb-fg);
    cursor: pointer;
    transition:
      background-color 120ms ease-out,
      color 120ms ease-out;
  }

  .ink-toolbar-btn:hover {
    background: var(--tb-hover);
  }

  .ink-toolbar-btn:focus-visible {
    outline: 2px solid var(--ink-accent);
    outline-offset: 1px;
  }

  .ink-toolbar-btn:disabled {
    opacity: 0.32;
    cursor: default;
  }

  .ink-toolbar-btn:disabled:hover {
    background: transparent;
  }

  .ink-toolbar-btn.active {
    background: var(--tb-active-bg);
    color: var(--ink-accent);
    box-shadow: inset 0 0 0 1.5px var(--ink-accent);
  }

  .ink-toolbar-btn.primary {
    background: var(--ink-accent);
    color: #111315;
  }

  .ink-toolbar-btn.primary:hover {
    filter: brightness(1.08);
  }

  .ink-toolbar-style-dot {
    border-radius: 999px;
    box-shadow: 0 0 0 1.5px var(--tb-dot-ring);
    transition:
      width 120ms ease-out,
      height 120ms ease-out;
  }

  .ink-toolbar-style-dot.marker {
    opacity: 0.55;
  }

  .ink-toolbar-style-font {
    font-size: 12px;
    line-height: 1;
    letter-spacing: -0.02em;
    text-shadow: 0 0 1px rgba(0, 0, 0, 0.35);
  }

  .ink-toolbar-divider {
    flex: none;
    width: 1px;
    height: 18px;
    margin: 0 3px;
    background: var(--tb-divider);
  }

  .ink-toolbar-tip {
    position: absolute;
    bottom: calc(100% + 8px);
    transform: translateX(-50%);
    padding: 4px 8px;
    border-radius: 6px;
    background: var(--tb-tip-bg);
    color: var(--tb-tip-fg);
    font-size: 11px;
    font-weight: 500;
    line-height: 1.25;
    white-space: nowrap;
    pointer-events: none;
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.32);
    z-index: 10;
  }
</style>
