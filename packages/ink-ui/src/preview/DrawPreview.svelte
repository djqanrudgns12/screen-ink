<script>
  import '../lib/tokens.css';
  import { onMount } from 'svelte';
  import DrawCanvas from '../lib/components/DrawCanvas.svelte';
  import MiniMap from '../lib/components/MiniMap.svelte';
  import { InkEngine, INK_COLORS } from '../lib/ink/inkEngine.js';
  import { visibleDocFraction } from '../lib/ink/coords.js';
  import { renderDocument } from '../lib/ink/canvasRender.js';

  /**
   * 브라우저 미리보기 `?draw=1` — DrawCanvas 데모(가짜 배경 그림 + FakeInkClient 단축키·설정).
   * `&board=1` 칠판, `&platform=macos` 맥 키, `&apply=auto` 멈춤 도형 바로 바꾸기, `&desync=0` desynchronized 끔.
   * @type {{ client: import('@screen-ink/client').InkClient }}
   */
  let { client } = $props();

  const params = new URLSearchParams(typeof location !== 'undefined' ? location.search : '');
  const board = params.get('board') === '1';
  const platform = params.get('platform') === 'macos' ? 'macos' : 'windows';
  const desync = params.get('desync') !== '0';
  const vw = typeof window !== 'undefined' ? window.innerWidth : 1280;
  const vh = typeof window !== 'undefined' ? window.innerHeight : 720;
  const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;

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
    g.font = '600 44px "IBM Plex Sans KR", sans-serif';
    g.fillText('3단원 · 이차방정식의 근의 공식', 80, 120);
    g.font = '28px "IBM Plex Mono", monospace';
    g.fillText('ax² + bx + c = 0  →  x = (−b ± √(b² − 4ac)) / 2a', 80, 210);
    g.font = '24px "IBM Plex Sans KR", sans-serif';
    ['① 판별식 D = b² − 4ac', '② D > 0 이면 서로 다른 두 실근', '③ D = 0 이면 중근', '④ D < 0 이면 허근'].forEach((t, i) =>
      g.fillText(t, 100, 300 + i * 48),
    );
    g.fillStyle = '#2F6FED';
    g.fillRect(vw - 420, 260, 300, 200);
    g.fillStyle = '#FFFFFF';
    g.font = '600 22px "IBM Plex Sans KR", sans-serif';
    g.fillText('예제 1', vw - 390, 300);
    g.font = '20px "IBM Plex Mono", monospace';
    g.fillText('x² − 5x + 6 = 0', vw - 390, 350);
    return c;
  }

  /** @type {HTMLCanvasElement | null} */
  let frame = $state.raw(null);
  let patch = $state.raw(engine.islandPatch);
  let fraction = $state.raw({ x: 0, y: 0, w: 1, h: 1 });
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
  let noticeTimer = 0;

  /** @param {string} text */
  function showNotice(text) {
    notice = text;
    clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => (notice = ''), 2400);
  }

  /** @param {import('../lib/ink/inkEngine.js').InkEffect} fx */
  function handleEffect(fx) {
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
        showNotice(kept ? '판서를 보관했어요 · Alt+Shift+1 (데모: Ctrl+Shift+R로 복원)' : '판서를 끝냈어요');
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
        showNotice(`${fx.type === 'copy' ? '복사' : '저장'} 합성 ${out.width}×${out.height} (데모 — 클립보드·파일 없음)`);
        break;
      }
      case 'centerCursor':
        break;
    }
  }

  onMount(() => {
    frame = board ? null : makeFakeFrame();
    const off = engine.subscribe(() => {
      patch = engine.islandPatch;
      fraction = visibleDocFraction(engine.view, engine.viewport, engine.doc.docSize);
      undoDepth = engine.undoStack.length;
    });
    client
      .getState()
      .then((state) => {
        engine.setShortcuts(state.shortcuts);
        engine.setSettings({ ...state.settings, ...(params.get('apply') === 'auto' ? { 'draw.shapeApply': 'auto' } : {}) });
      })
      .catch(() => {});
    return off;
  });

  const TOOL_LABEL = /** @type {Record<string, string>} */ ({
    pen: '펜',
    marker: '형광펜',
    text: '글자',
    select: '고르기',
    eraser: '지우개',
    pan: '손',
  });
</script>

<div class="draw-preview">
  <DrawCanvas
    bind:this={canvasRef}
    {engine}
    backgroundImage={frame}
    desynchronized={desync}
    onEffect={handleEffect}
    onPenMetrics={(m) => (metrics = m)}
  />

  <div class="draw-preview-island" aria-live="polite">
    <span class="dot" style:background={INK_COLORS[/** @type {keyof typeof INK_COLORS} */ (patch.color)]}></span>
    <b>{patch.board ? '칠판' : '판서'}</b>
    <span>{TOOL_LABEL[patch.tool] ?? patch.tool} {patch.widthPx}px</span>
    <span class="zoom">×{patch.zoom.toFixed(2)}</span>
    <span class="muted">{patch.background}{patch.board ? ` · ${patch.pageIndex + 1} / ${patch.pageCount}` : ''}</span>
    <span class="muted">undo {undoDepth}</span>
    {#if metrics}
      <span class="muted">pen n={metrics.n} p50={metrics.p50} p95={metrics.p95} max={metrics.max} ms</span>
    {/if}
    <button type="button" onclick={() => (metrics = canvasRef?.getPenMetrics?.() ?? null)}>지연 보기</button>
  </div>

  {#if suggestion}
    <button type="button" class="draw-preview-suggest" onclick={() => engine.acceptSuggestion().forEach(handleEffect)}>
      {suggestion}
    </button>
  {:else if notice}
    <div class="draw-preview-notice">{notice}</div>
  {/if}

  {#if helpOpen}
    <div class="draw-preview-help">
      P M T S D E 도구 · R Y G O B L 색 · W/K 칠판 · Shift 직선 · Ctrl 사각 · Ctrl+Shift 화살표 · Tab 원 · +Alt 점선 ·
      휠 확대 · Ctrl+휠 굵기 · 우클릭 손 모드 · Space 1× · Ctrl+Z/Y · Esc 한 단계씩
    </div>
  {/if}

  {#if patch.zoom > 1}
    <MiniMap class="draw-preview-minimap" size="sm" viewport={fraction} label="" />
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
    font-family: 'IBM Plex Sans KR', sans-serif;
  }

  .draw-preview-island,
  .draw-preview-notice,
  .draw-preview-suggest,
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
  }

  .draw-preview-island {
    top: 14px;
  }

  .draw-preview-island button,
  .draw-preview-suggest {
    pointer-events: auto;
  }

  .draw-preview-island button {
    background: rgba(255, 255, 255, 0.12);
    color: inherit;
    border: none;
    border-radius: 999px;
    padding: 3px 10px;
    font: inherit;
    cursor: pointer;
  }

  .draw-preview-notice,
  .draw-preview-suggest {
    top: 60px;
  }

  .draw-preview-suggest {
    border: 1px solid #ff9f0a;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }

  .draw-preview-help {
    bottom: 24px;
    border-radius: 12px;
    white-space: normal;
    max-width: 720px;
  }

  .dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.3);
  }

  .zoom {
    color: #ffb340;
  }

  .muted {
    color: rgba(247, 248, 250, 0.6);
    font-family: 'IBM Plex Mono', monospace;
    font-size: 12px;
  }

  :global(.draw-preview-minimap) {
    position: absolute;
    right: 16px;
    bottom: 16px;
  }
</style>
