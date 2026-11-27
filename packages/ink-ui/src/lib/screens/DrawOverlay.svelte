<script>
  import { untrack } from 'svelte';
  import CheatSheet from './CheatSheet.svelte';
  import DrawCanvas from '../components/DrawCanvas.svelte';
  import MiniMap from '../components/MiniMap.svelte';
  import { syncLang } from '../i18n/index.svelte.js';
  import { InkEngine, documentOptionsFromSession } from '../ink/inkEngine.js';
  import { renderDocument } from '../ink/canvasRender.js';
  import { BASIC_COLOR_CODES } from '../models/palette.js';
  import { parseAnnotateStyle, serializeAnnotateStyle } from '../models/inkSize.js';
  import { clampDrawSettingWidth, drawIslandColor } from '../models/drawToolbar.js';
  import {
    DRAW_IDLE_DIM_MS,
    DRAW_MINIMAP_MARGIN_PX,
    ISLAND_AVOID_MARGIN_PX,
    drawMinimapState,
    estimateTopIslandRect,
    sessionCursorToCss,
    shouldAvoidIsland,
  } from '../models/drawUi.js';

  /**
   * S12 Alt+1 판서 · S14 Alt+3 칠판 — 오버레이 창(모니터 1개) 하나에 판서 한 번(A §2.4·§3.7).
   * 그리기(포인터·획·확대·Undo·Esc 단계·스마트 우클릭·키 29칸)는 InkEngine + DrawCanvas가,
   * 이 화면은 그 결과(InkEffect)를 InkClient로 잇는다:
   *   exit → drawFinish(문서 보관) · copy/save → 모니터 전체 합성 → drawExport(원본 바이트) ·
   *   restore → drawRestore → engine.restoreKept · help → 단축키 한 장 · notice/suggest → drawIsland ·
   *   섬 단추(drawCommand) → acceptSuggestion · undo · 첫 그림 → drawReady(M4) · 펜 지연 → drawPenMetrics(M5).
   * Rust가 먼저 끝내면(같은 키·다른 모드 키·잠금·화면 구성 변경) 상태가 바뀌어 이 화면이 내려가고,
   * 그때 문서를 늦은 끝(drawFinish)으로 보내 보관한다(문서는 웹뷰에만 있음).
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   session: import('@screen-ink/client').DrawSession,
   *   monitorId?: number,
   *   class?: string,
   * }}
   */
  let { client, session, monitorId = 0, class: className = '' } = $props();

  // 세션 하나 = 엔진 하나(App.svelte가 세션마다 이 화면을 새로 만든다 — {#key})
  const engine = untrack(
    () =>
      new InkEngine({
        docOptions: documentOptionsFromSession(session),
        view: session.initialView,
        viewport: { w: typeof window !== 'undefined' ? window.innerWidth : session.monitor.bounds.w, h: typeof window !== 'undefined' ? window.innerHeight : session.monitor.bounds.h },
      }),
  );
  const sessionId = untrack(() => session.session);

  let helpOpen = $state(false);
  /** @type {HTMLImageElement | ImageBitmap | null} */
  let bgImage = $state.raw(null);
  let bgFailed = $state(false);
  /** @type {any} */
  let canvasRef = $state(null);
  let minimap = $state.raw(drawMinimapState(engine.view, engine.viewport, engine.doc.docSize));
  let initialCursor = $derived(sessionCursorToCss(session));

  let frameUrl = $derived(session.board ? '' : session.monitor?.frameUrl || '');
  // 정지 화면이 읽히기 전에는 캔버스를 숨긴다 — 투명 창 너머 바탕이 곧 그 화면이라, 빈 바탕색이 번쩍이지 않게
  let canvasReady = $derived(!frameUrl || bgFailed || bgImage !== null);

  let finished = false;
  let readySent = false;
  let imageReadyAt = 0;
  let stateApplied = false;
  let lastPatchKey = '';
  let lastSyncedStyleRaw = '';
  /** @type {import('../models/inkSize.js').AnnotateStyleState | null} */
  let drawStyle = $state.raw(null);
  let idleDim = false;
  let cursorAvoid = false;
  /** @type {ReturnType<typeof setTimeout> | null} */
  let idleTimer = null;
  const mountedAt = nowMs();

  function nowMs() {
    return typeof performance !== 'undefined' && typeof performance.now === 'function' ? performance.now() : Date.now();
  }

  /** @type {Promise<void> | undefined} */
  let loadingFrame;
  $effect(() => {
    const url = frameUrl;
    if (!url) return;
    const abort = new AbortController();
    /** @type {ImageBitmap | null} */
    let bitmap = null;
    /** @type {HTMLImageElement | null} */
    let fallbackImage = null;
    loadingFrame = (async () => {
      if (client.drawFrame && url.startsWith('http://ink.localhost/')) {
        bitmap = await client.drawFrame(sessionId, abort.signal);
        if (abort.signal.aborted) {
          bitmap?.close();
          return;
        }
      }
      let image = /** @type {HTMLImageElement | ImageBitmap | null} */ (bitmap);
      if (!image) {
        const img = new Image();
        fallbackImage = img;
        img.crossOrigin = 'anonymous';
        img.decoding = 'async';
        img.src = url;
        await img.decode();
        image = img;
      }
      if (!abort.signal.aborted) {
        imageReadyAt = nowMs();
        bgImage = image;
      }
    })().catch(() => {
      if (!abort.signal.aborted) bgFailed = true;
    });
    return () => {
      abort.abort();
      bitmap?.close();
      fallbackImage?.removeAttribute('src');
      bgImage = null;
    };
  });

  /** @param {number} at 첫 그림을 마친 시각 @param {number} [renderMs] 캔버스 그리기 시간 */
  function sendReady(at, renderMs) {
    if (readySent) return;
    readySent = true;
    client.drawReady(sessionId, at - mountedAt, imageReadyAt ? imageReadyAt - mountedAt : undefined, renderMs);
  }

  // ───────────── 설정·키(화면 안 키는 설정값에서, R-KEY-6) ─────────────

  /** @param {any} state */
  function applyState(state) {
    if (!state) return;
    syncLang(state);
    const platform = state.platform === 'macos' ? 'macos' : 'windows';
    if (engine.platform !== platform) engine.platform = platform;
    if (Array.isArray(state.shortcuts)) engine.setShortcuts(state.shortcuts);
    // 처음 한 번은 펜 색·굵기도 설정값으로(그 뒤에는 그리는 중 바꾼 값을 지킨다)
    if (state.settings) {
      engine.setSettings(state.settings, { initial: !stateApplied });
      const rawStyle = typeof state.settings['annotate.style'] === 'string' ? state.settings['annotate.style'] : '';
      if (rawStyle && rawStyle !== lastSyncedStyleRaw) {
        lastSyncedStyleRaw = rawStyle;
        drawStyle = parseAnnotateStyle(rawStyle);
      }
    }
    stateApplied = true;
    pushIsland();
  }

  /**
   * 도구 막대·색 굵기 패널 변경을 설정에 저장한다.
   * Rust 설정 `draw.width`는 1~30, `draw.color`는 기본 8색으로 제한한다(R2-ANN-5).
   * @param {import('../models/inkSize.js').AnnotateStyleState} next
   * @param {{ kind: 'color' | 'size', toolId: string, color?: string, size?: number }} meta
   */
  function handleStyleChange(next, meta) {
    drawStyle = next;
    const raw = serializeAnnotateStyle(next);
    if (raw !== lastSyncedStyleRaw && typeof client?.updateSetting === 'function') {
      lastSyncedStyleRaw = raw;
      Promise.resolve(client.updateSetting('annotate.style', raw)).catch(() => {});
    }
    if (typeof client?.updateSetting !== 'function') return;
    if (
      meta?.kind === 'size' &&
      typeof meta.size === 'number' &&
      (meta.toolId === 'pen' || meta.toolId === 'line' || meta.toolId === 'rect' || meta.toolId === 'ellipse')
    ) {
      Promise.resolve(client.updateSetting('draw.width', clampDrawSettingWidth(meta.size))).catch(() => {});
    }
    if (meta?.kind === 'color' && meta.color && BASIC_COLOR_CODES.includes(meta.color)) {
      Promise.resolve(client.updateSetting('draw.color', meta.color)).catch(() => {});
    }
  }

  // ───────────── 그리기 섬 ─────────────

  function patchOf() {
    const p = engine.islandPatch;
    const color = drawIslandColor(p.color);
    return {
      session: sessionId,
      ...(color ? { color } : {}),
      tool: p.tool,
      widthPx: clampDrawSettingWidth(p.widthPx),
      zoom: p.zoom,
      background: p.background,
      pageIndex: p.pageIndex,
      pageCount: p.pageCount,
      suggestion: p.suggestion,
      idleDim,
      avoid: cursorAvoid,
    };
  }

  function resetIdleTimer() {
    if (idleTimer !== null) clearTimeout(idleTimer);
    if (idleDim) {
      idleDim = false;
      pushIsland();
    }
    idleTimer = setTimeout(() => {
      idleTimer = null;
      if (finished || idleDim) return;
      idleDim = true;
      pushIsland();
    }, DRAW_IDLE_DIM_MS);
  }

  /** @param {PointerEvent} e */
  function handleOverlayPointerMove(e) {
    resetIdleTimer();
    const vw = typeof window !== 'undefined' ? window.innerWidth : engine.viewport.w;
    const rect = estimateTopIslandRect(vw);
    const nextAvoid =
      !engine.suggestion && shouldAvoidIsland({ x: e.clientX, y: e.clientY }, rect, ISLAND_AVOID_MARGIN_PX);
    if (nextAvoid !== cursorAvoid) {
      cursorAvoid = nextAvoid;
      pushIsland();
    }
  }

  function handleOverlayPointerLeave() {
    if (cursorAvoid) {
      cursorAvoid = false;
      pushIsland();
    }
  }

  function syncMinimap() {
    minimap = drawMinimapState(engine.view, engine.viewport, engine.doc.docSize);
  }

  /** 섬 한 줄이 바뀔 때만 drawIsland(포인터 이동마다 IPC를 타지 않게). */
  function pushIsland() {
    const p = patchOf();
    const key = JSON.stringify(p);
    if (key === lastPatchKey) return;
    lastPatchKey = key;
    client.drawIsland(p);
  }
  // Rust가 진입 섬을 설정값으로 이미 띄웠다 — 처음 모양은 보내지 않는다
  lastPatchKey = JSON.stringify(patchOf());

  $effect(() => {
    let alive = true;
    Promise.resolve(client.getState())
      .then((s) => {
        if (alive) applyState(s);
      })
      .catch(() => {});
    const off = client.subscribe((e) => {
      if (e.type === 'state') applyState(e.state);
      else if (e.type === 'drawCommand' && e.session === sessionId) runCommand(e.command);
    });
    const unsubscribe = engine.subscribe(() => {
      syncMinimap();
      resetIdleTimer();
      pushIsland();
    });
    syncMinimap();
    resetIdleTimer();
    return () => {
      alive = false;
      if (idleTimer !== null) {
        clearTimeout(idleTimer);
        idleTimer = null;
      }
      off();
      unsubscribe();
    };
  });

  /** 섬 단추 → 엔진(멈춤 도형 [바꾸기] · 모두 지우기 [되돌리기]). @param {string} command */
  function runCommand(command) {
    if (command === 'acceptSuggestion') {
      for (const fx of engine.acceptSuggestion()) handleEffect(fx);
    } else if (command === 'undo') {
      if (engine.undo()) engine.notify();
    }
    canvasRef?.redraw?.();
  }

  // ───────────── 엔진 효과 ─────────────

  /** @param {import('../ink/inkEngine.js').InkEffect} fx */
  function handleEffect(fx) {
    switch (fx.type) {
      case 'exit':
        finish();
        break;
      case 'copy':
      case 'save':
        exportImage(fx.type);
        break;
      case 'restore':
        restore();
        break;
      case 'help':
        helpOpen = fx.open;
        break;
      case 'centerCursor':
        // 1× 복귀는 엔진이 했다 — 운영체제 커서는 옮기지 않는다
        break;
      case 'notice':
        lastPatchKey = '';
        client.drawIsland({ ...patchOf(), notice: fx.text, ...(fx.action === 'undo' ? { noticeAction: 'undo' } : {}) });
        lastPatchKey = JSON.stringify(patchOf());
        break;
      case 'suggest':
      case 'suggestEnd':
        pushIsland();
        break;
    }
  }

  /**
   * Ctrl+C · Ctrl+S: 확대 중이어도 늘 모니터 전체(물리 해상도 그대로, 사용자 결정) — 배경 + 지금 장 합성 → 원본 바이트.
   * @param {'copy' | 'save'} action
   */
  async function exportImage(action) {
    const doc = engine.exportDocument();
    if (doc.pages[doc.activePage]?.background === 'frame' && !bgImage) {
      await loadingFrame;
      if (!bgImage || finished) return;
    }
    const { w, h } = doc.physSize;
    if (!(w > 0 && h > 0) || typeof document === 'undefined') return;
    /** @type {OffscreenCanvas | HTMLCanvasElement} */
    const canvas = typeof OffscreenCanvas === 'function' ? new OffscreenCanvas(w, h) : Object.assign(document.createElement('canvas'), { width: w, height: h });
    const ctx = /** @type {any} */ (canvas.getContext('2d', { willReadFrequently: true }));
    if (!ctx) return;
    let pixels;
    try {
      renderDocument(ctx, doc, { image: bgImage });
      pixels = ctx.getImageData(0, 0, w, h).data;
    } catch {
      // 배경이 교차 출처로 막혔으면 내보낼 수 없다(조용히 실패하지 않게 섬은 Rust가 띄우지 못함 — 기록만)
      console.warn('판서 내보내기에서 캔버스 픽셀을 읽지 못했습니다');
      return;
    } finally {
      canvas.width = 0;
      canvas.height = 0;
    }
    await client.drawExport(action, pixels, { w, h, session: sessionId });
  }

  /** Ctrl+Shift+R · restoreOnOpen — Rust 판정(모니터·배율) → 엔진에 싣기(안내 문구는 엔진이 낸다). */
  async function restore() {
    const r = await client.drawRestore(sessionId);
    for (const fx of engine.restoreKept(r?.kept ?? null, r?.status ?? 'none')) handleEffect(fx);
  }

  // Alt+Shift+1: 열자마자 복원. 판서 중 다시 누르면 같은 세션이 restoreOnOpen=true로 한 번 더 온다
  $effect(() => {
    if (session.restoreOnOpen) untrack(() => restore());
  });

  // ───────────── 끝내기 · 늦은 끝 ─────────────

  /** 판서 끝(보관 여부는 Rust가 draw.keep·항목 수로 정한다). 세션당 한 번. */
  function finish() {
    if (finished) return;
    finished = true;
    try {
      canvasRef?.flushPenMetrics?.();
    } catch {
      // 캔버스가 먼저 내려갔으면 내려갈 때 스스로 보냈다
    }
    const doc = engine.hasItems() ? engine.exportDocument() : null;
    client.drawFinish({ session: sessionId, keep: true, kept: doc ? { document: doc } : undefined });
  }

  $effect(() => {
    return () => finish();
  });

  /** @param {MouseEvent} e */
  function handleContextMenu(e) {
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={resetIdleTimer} />

<div
  class="ink-draw-overlay {className}"
  data-monitor={monitorId}
  data-board={session.board ? 'true' : undefined}
  role="presentation"
  onpointermove={handleOverlayPointerMove}
  onpointerdown={resetIdleTimer}
  onpointerleave={handleOverlayPointerLeave}
  onwheel={resetIdleTimer}
  oncontextmenu={handleContextMenu}
>
  <div class="ink-draw-canvas-wrap" data-ready={canvasReady ? 'true' : undefined}>
    <DrawCanvas
      bind:this={canvasRef}
      {engine}
      backgroundImage={bgImage}
      {initialCursor}
      styleState={drawStyle}
      onStyleChange={handleStyleChange}
      onEffect={handleEffect}
      onPenMetrics={(sample) => client.drawPenMetrics(sample)}
      onFirstPaint={sendReady}
    />
  </div>

  {#if minimap.visible}
    <div
      class="ink-draw-minimap"
      aria-hidden="true"
      style:right={`${DRAW_MINIMAP_MARGIN_PX}px`}
      style:bottom={`${DRAW_MINIMAP_MARGIN_PX}px`}
    >
      <MiniMap size="sm" viewport={minimap.viewport} label="" />
    </div>
  {/if}

  {#if helpOpen}
    <CheatSheet {client} mode="draw" bind:open={helpOpen} onClose={() => (engine.helpOpen = false)} />
  {/if}
</div>

<style>
  .ink-draw-overlay {
    position: fixed;
    inset: 0;
    overflow: hidden;
    user-select: none;
    touch-action: none;
    background: transparent;
  }

  .ink-draw-canvas-wrap {
    position: absolute;
    inset: 0;
    visibility: hidden;
  }

  .ink-draw-canvas-wrap[data-ready='true'] {
    visibility: visible;
  }

  .ink-draw-canvas-wrap :global(.ink-draw) {
    position: absolute;
    inset: 0;
  }

  .ink-draw-minimap {
    position: absolute;
    pointer-events: none;
    z-index: 20;
  }
</style>
