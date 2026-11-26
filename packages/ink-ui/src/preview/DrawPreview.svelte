<script>
  import '../lib/tokens.css';
  import { onMount } from 'svelte';
  import DrawCanvas from '../lib/components/DrawCanvas.svelte';
  import Island from '../lib/components/Island.svelte';
  import MiniMap from '../lib/components/MiniMap.svelte';
  import { locale, setLang, setOs, syncLang, t } from '../lib/i18n/index.svelte.js';
  import { InkEngine } from '../lib/ink/inkEngine.js';
  import { renderDocument } from '../lib/ink/canvasRender.js';
  import {
    DRAW_IDLE_DIM_MS,
    DRAW_MINIMAP_MARGIN_PX,
    DRAW_PALETTE_EXPAND_MS,
    ISLAND_AVOID_MARGIN_PX,
    drawMinimapState,
    formatBoardIslandText,
    resolveDrawIdleDim,
    shouldAvoidIsland,
    shouldExpandDrawPalette,
  } from '../lib/models/drawUi.js';

  /**
   * 브라우저 미리보기 `?draw=1` — DrawCanvas + 실제 그리기 섬(6/8색 펼침·4초 물러남·커서 회피) + MiniMap 데모.
   * `&board=1` 칠판, `&platform=macos` 맥 키, `&lang=en` 영어, `&apply=auto` 멈춤 도형 바로 바꾸기, `&desync=0` desynchronized 끔.
   * @type {{ client: import('@screen-ink/client').InkClient }}
   */
  let { client } = $props();

  const params = new URLSearchParams(typeof location !== 'undefined' ? location.search : '');
  const board = params.get('board') === '1';
  const platform = params.get('platform') === 'macos' ? 'macos' : 'windows';
  const initialLang = params.get('lang') === 'en' ? 'en' : 'ko';
  const desync = params.get('desync') !== '0';
  const vw = typeof window !== 'undefined' ? window.innerWidth : 1280;
  const vh = typeof window !== 'undefined' ? window.innerHeight : 720;
  const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;

  setLang(initialLang);
  setOs(platform);

  const engine = new InkEngine({
    docOptions: {
      id: board ? 'board-1' : 'draw-1',
      mode: board ? 'board' : 'draw',
      docSize: { w: vw, h: vh },
      physSize: { w: Math.round(vw * dpr), h: Math.round(vh * dpr) },
      scale: dpr,
      frameId: board ? null : 1,
    },
    viewport: { w: vw, h: vh },
    platform,
    lang: initialLang,
    settings: params.get('apply') === 'auto' ? { 'draw.shapeApply': 'auto' } : {},
  });

  /** 가짜 정지 화면: 수업 슬라이드처럼 보이는 합성 그림(물리 px). */
  function makeFakeFrame() {
    const c = document.createElement('canvas');
    c.width = Math.round(vw * dpr);
    c.height = Math.round(vh * dpr);
    const g = c.getContext('2d');
    if (!g) return c;
    g.scale(dpr, dpr);
    g.fillStyle = '#F4F1EA';
    g.fillRect(0, 0, vw, vh);
    g.strokeStyle = 'rgba(0,0,0,0.06)';
    g.lineWidth = 1;
    for (let x = 0; x < vw; x += 40) {
      g.beginPath();
      g.moveTo(x + 0.5, 0);
      g.lineTo(x + 0.5, vh);
      g.stroke();
    }
    for (let y = 0; y < vh; y += 40) {
      g.beginPath();
      g.moveTo(0, y + 0.5);
      g.lineTo(vw, y + 0.5);
      g.stroke();
    }
    g.fillStyle = '#1F2933';
    g.font = '600 44px "Freesentation", "IBM Plex Sans KR", sans-serif';
    g.fillText(t('meta.preview.drawSampleTitle'), 80, 120);
    g.font = '28px "IBM Plex Mono", monospace';
    g.fillText('ax² + bx + c = 0  →  x = (−b ± √(b² − 4ac)) / 2a', 80, 210);
    g.font = '24px "Freesentation", "IBM Plex Sans KR", sans-serif';
    [
      t('meta.preview.drawSampleBullet1'),
      t('meta.preview.drawSampleBullet2'),
      t('meta.preview.drawSampleBullet3'),
      t('meta.preview.drawSampleBullet4'),
    ].forEach((line, i) => g.fillText(line, 100, 300 + i * 48));
    g.fillStyle = '#2F6FED';
    g.fillRect(vw - 420, 260, 300, 200);
    g.fillStyle = '#FFFFFF';
    g.font = '600 22px "Freesentation", "IBM Plex Sans KR", sans-serif';
    g.fillText(t('meta.preview.drawSampleExample'), vw - 390, 300);
    g.font = '20px "IBM Plex Mono", monospace';
    g.fillText('x² − 5x + 6 = 0', vw - 390, 350);
    return c;
  }

  /** @type {HTMLCanvasElement | null} */
  let frame = $state.raw(null);
  let patch = $state.raw(engine.islandPatch);
  let minimap = $state.raw(drawMinimapState(engine.view, engine.viewport, engine.doc.docSize));
  let notice = $state('');
  /** @type {string | null} */
  let suggestion = $state(null);
  /** @type {import('../lib/ink/penMetrics.js').PenLatencySummary | null} */
  let metrics = $state.raw(null);
  let undoDepth = $state(0);
  let helpOpen = $state(false);
  /** @type {import('../lib/ink/inkEngine.js').InkDocument | null} */
  let kept = null;
  /** @type {any} */
  let canvasRef = $state(null);
  /** @type {HTMLDivElement | null} */
  let islandAnchor = $state(null);

  // 4.8 그리기 섬 상태(0.8초 펼침 · 4초 40% 물러남 · bbox+24px 커서 회피) + 데모 강제 토글
  let paletteTransient = $state(false);
  let forcePalette = $state(false);
  let autoIdleDim = $state(false);
  let forceIdleDim = $state(false);
  let islandHover = $state(false);
  let cursorAvoid = $state(false);

  let noticeTimer = 0;
  let idleTimer = 0;
  let paletteTimer = 0;
  let prevColor = engine.islandPatch.color;

  function resetIdle() {
    autoIdleDim = false;
    clearTimeout(idleTimer);
    idleTimer = window.setTimeout(() => {
      autoIdleDim = true;
    }, DRAW_IDLE_DIM_MS);
  }

  /** @param {string} text */
  function showNotice(text) {
    notice = text;
    clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => (notice = ''), 2400);
  }

  /** @param {import('../lib/ink/inkEngine.js').InkEffect} fx */
  function handleEffect(fx) {
    resetIdle();
    switch (fx.type) {
      case 'notice':
        showNotice(fx.text);
        break;
      case 'suggest':
        suggestion = fx.text;
        break;
      case 'suggestEnd':
        suggestion = null;
        break;
      case 'help':
        helpOpen = fx.open;
        break;
      case 'exit':
        metrics = canvasRef?.flushPenMetrics?.() ?? metrics;
        if (engine.hasItems()) kept = engine.exportDocument();
        if (kept) {
          const restore = platform === 'macos' ? '⌥⇧1' : 'Alt+Shift+1';
          const demo = platform === 'macos' ? '⌘⇧R' : 'Ctrl+Shift+R';
          showNotice(t('meta.preview.drawExitKept', { restore, demo }));
        } else {
          showNotice(t('meta.preview.drawExitEnded'));
        }
        engine.loadDocument({ ...engine.doc, pages: engine.doc.pages.map((p) => ({ ...p, items: [] })) });
        engine.setView({ s: 1, tx: 0, ty: 0 });
        break;
      case 'restore':
        for (const e of engine.restoreKept(kept, kept ? 'same' : 'none')) handleEffect(e);
        break;
      case 'copy':
      case 'save': {
        const doc = engine.exportDocument();
        const out = document.createElement('canvas');
        out.width = doc.physSize.w;
        out.height = doc.physSize.h;
        const g = out.getContext('2d');
        if (g) renderDocument(/** @type {any} */ (g), doc, { image: frame });
        showNotice(
          t('meta.preview.drawCopySynth', {
            action: fx.type === 'copy' ? t('common.copy') : t('common.save'),
            w: out.width,
            h: out.height,
          }),
        );
        break;
      }
      case 'centerCursor':
        break;
    }
  }

  /** @param {PointerEvent} ev */
  function handleStagePointerMove(ev) {
    resetIdle();
    if (suggestion || forcePalette || paletteTransient || !islandAnchor) {
      cursorAvoid = false;
      return;
    }
    cursorAvoid = shouldAvoidIsland(
      { x: ev.clientX, y: ev.clientY },
      islandAnchor.getBoundingClientRect(),
      ISLAND_AVOID_MARGIN_PX,
    );
  }

  function handleStagePointerLeave() {
    cursorAvoid = false;
  }

  function toggleBoardBackground() {
    const next = patch.background === 'frame' ? 'white' : patch.background === 'white' ? 'black' : 'frame';
    engine.setBackground(next);
  }

  $effect(() => {
    void locale.lang;
    if (!board && typeof document !== 'undefined') {
      frame = makeFakeFrame();
    }
  });

  onMount(() => {
    resetIdle();
    const off = engine.subscribe(() => {
      const next = engine.islandPatch;
      const variant = next.board || next.background !== 'frame' ? 'board' : 'draw';
      if (shouldExpandDrawPalette(prevColor, next.color, variant)) {
        paletteTransient = true;
        clearTimeout(paletteTimer);
        paletteTimer = window.setTimeout(() => (paletteTransient = false), DRAW_PALETTE_EXPAND_MS);
      }
      prevColor = next.color;
      patch = next;
      minimap = drawMinimapState(engine.view, engine.viewport, engine.doc.docSize);
      undoDepth = engine.undoStack.length;
      resetIdle();
    });
    const offState = client.subscribe((e) => {
      if (e.type === 'state') syncLang(e.state);
    });
    client
      .getState()
      .then((state) => {
        syncLang(state);
        engine.setShortcuts(state.shortcuts);
        engine.setSettings({ ...state.settings, ...(params.get('apply') === 'auto' ? { 'draw.shapeApply': 'auto' } : {}) });
      })
      .catch(() => {});
    return () => {
      off();
      offState();
      clearTimeout(noticeTimer);
      clearTimeout(idleTimer);
      clearTimeout(paletteTimer);
    };
  });

  function toolLabel(tool) {
    const map = {
      pen: 'island.draw.tool.pen',
      marker: 'island.draw.tool.marker',
      text: 'island.draw.tool.text',
      select: 'island.draw.tool.select',
      eraser: 'island.draw.tool.eraser',
      pan: 'island.draw.tool.pan',
    };
    return map[tool] ? t(map[tool]) : tool;
  }

  let isBoardView = $derived(patch.board || patch.background !== 'frame');
  let boardText = $derived(
    formatBoardIslandText({
      background: patch.background,
      pageIndex: patch.pageIndex,
      pageCount: patch.pageCount,
      lang: locale.lang,
    }),
  );
  let zoomValue = $derived.by(() => {
    if (patch.zoom <= 1 + 1e-6) return '';
    const r = Math.round(patch.zoom * 100);
    return r % 10 === 0 ? `×${(r / 100).toFixed(1)}` : `×${(r / 100).toFixed(2)}`;
  });
  let islandIdleDim = $derived(
    resolveDrawIdleDim({
      explicitIdleDim: forceIdleDim ? true : undefined,
      autoIdleDim,
      hovering: islandHover,
      hasAction: Boolean(suggestion),
    }),
  );
  let islandAvoid = $derived(cursorAvoid && !suggestion && !forcePalette && !paletteTransient);
  let currentBgName = $derived(
    patch.background === 'white'
      ? t('page.draw.boardWhite')
      : patch.background === 'black'
        ? t('page.draw.boardBlack')
        : t('meta.preview.drawBgScreen'),
  );
</script>

<svelte:window onkeydown={resetIdle} />

<div
  class="draw-preview"
  role="presentation"
  onpointermove={handleStagePointerMove}
  onpointerdown={resetIdle}
  onpointerleave={handleStagePointerLeave}
>
  <DrawCanvas
    bind:this={canvasRef}
    {engine}
    backgroundImage={frame}
    desynchronized={desync}
    onEffect={handleEffect}
    onPenMetrics={(m) => (metrics = m)}
  />

  <div class="draw-preview-island-wrap">
    <div class="draw-preview-island-anchor" bind:this={islandAnchor}>
      <Island
        variant={isBoardView ? 'board' : 'draw'}
        penColor={patch.color}
        penWidth={isBoardView ? '' : `${patch.widthPx}px`}
        text={suggestion ? suggestion : isBoardView ? boardText.text : t('island.draw.title', { tool: toolLabel(patch.tool) })}
        value={suggestion ? '' : isBoardView ? boardText.value : zoomValue}
        keys={suggestion ? undefined : isBoardView ? ['W', 'K'] : ['?']}
        {platform}
        paletteExpanded={forcePalette || paletteTransient}
        idleDim={islandIdleDim}
        avoid={islandAvoid}
        onSelectColor={(code) => engine.setColor(code)}
        button={suggestion
          ? { label: t('island.draw.acceptSuggestion'), key: 'Enter', onClick: () => engine.acceptSuggestion().forEach(handleEffect) }
          : null}
        onmouseenter={() => (islandHover = true)}
        onmouseleave={() => (islandHover = false)}
      />
    </div>
  </div>

  {#if notice}
    <div class="draw-preview-notice">{notice}</div>
  {/if}

  {#if helpOpen}
    <div class="draw-preview-help">{t('meta.preview.drawHelp')}</div>
  {/if}

  <div class="draw-preview-hud" aria-label={t('meta.preview.drawHudAria')}>
    <span class="muted">undo {undoDepth}</span>
    <button type="button" data-active={forcePalette ? 'true' : undefined} onclick={() => (forcePalette = !forcePalette)}>
      {t('meta.preview.drawForcePalette')}
    </button>
    <button type="button" data-active={forceIdleDim ? 'true' : undefined} onclick={() => (forceIdleDim = !forceIdleDim)}>
      {t('meta.preview.drawForceDim')}
    </button>
    <button type="button" onclick={toggleBoardBackground}>
      {t('meta.preview.drawBgLabel', { bg: currentBgName })}
    </button>
    <button type="button" onclick={() => (metrics = canvasRef?.getPenMetrics?.() ?? null)}>{t('meta.preview.drawMetrics')}</button>
    {#if metrics}
      <span class="muted">pen n={metrics.n} p50={metrics.p50} p95={metrics.p95} max={metrics.max} ms</span>
    {/if}
  </div>

  {#if minimap.visible}
    <div
      class="draw-preview-minimap"
      aria-hidden="true"
      style:right={`${DRAW_MINIMAP_MARGIN_PX}px`}
      style:bottom={`${DRAW_MINIMAP_MARGIN_PX}px`}
    >
      <MiniMap size="sm" viewport={minimap.viewport} label="" />
    </div>
  {/if}
</div>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
  }

  .draw-preview {
    position: fixed;
    inset: 0;
    font-family: var(--ink-font-sans);
  }

  .draw-preview-island-wrap {
    position: absolute;
    left: 0;
    right: 0;
    top: 14px;
    display: flex;
    justify-content: center;
    pointer-events: none;
    z-index: 30;
  }

  .draw-preview-island-anchor {
    display: inline-flex;
    pointer-events: none;
  }

  .draw-preview-notice,
  .draw-preview-help {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(17, 18, 20, 0.92);
    color: #f7f8fa;
    border-radius: 999px;
    font-size: 13px;
    padding: 8px 14px;
    display: flex;
    gap: 10px;
    align-items: center;
    white-space: nowrap;
    pointer-events: none;
    z-index: 25;
  }

  .draw-preview-notice {
    top: 62px;
  }

  .draw-preview-help {
    bottom: 64px;
    border-radius: 12px;
    white-space: normal;
    max-width: 720px;
  }

  .draw-preview-hud {
    position: absolute;
    left: 16px;
    top: 14px;
    background: rgba(17, 18, 20, 0.85);
    color: #f7f8fa;
    border-radius: 999px;
    padding: 6px 12px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    z-index: 25;
  }

  .draw-preview-hud button {
    background: rgba(255, 255, 255, 0.12);
    color: inherit;
    border: none;
    border-radius: 999px;
    padding: 3px 10px;
    font: inherit;
    cursor: pointer;
  }

  .draw-preview-hud button[data-active='true'] {
    background: var(--ink-accent);
    color: #111315;
    font-weight: 500;
  }

  .muted {
    color: rgba(247, 248, 250, 0.65);
    font-family: var(--ink-font-mono);
    font-size: 11.5px;
  }

  .draw-preview-minimap {
    position: absolute;
    pointer-events: none;
    z-index: 20;
  }
</style>
