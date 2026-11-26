<script>
  import { untrack } from 'svelte';
  import AnnotateLayer from '../components/AnnotateLayer.svelte';
  import CaptureToolbar from '../components/CaptureToolbar.svelte';
  import ColorWidthPanel from '../components/ColorWidthPanel.svelte';
  import Magnifier from '../components/Magnifier.svelte';
  import SelectionFrame from '../components/SelectionFrame.svelte';
  import { setLang, syncLang, t } from '../i18n/index.svelte.js';
  import { formatComboReadable } from '../models/shortcutDisplay.js';
  import {
    TOOLBAR_ACTIONS,
    TOOLBAR_ANNOTATE_TOOLS,
    TOOLBAR_HISTORY,
    TOOLBAR_STYLE_TRIGGER,
    comboMap,
    estimateToolbarWidth,
    formatSizeBadge,
    placeSizeBadge,
    placeToolOptions,
    placeToolbar,
    toggleAnnotTool,
    toolbarOwner,
    toolbarTooltip,
  } from '../models/captureToolbar.js';
  import {
    MOSAIC_BLOCK_DEFAULT,
    anchorToCss,
    annotDocOptions,
    annotEngineSettings,
    annotKeyRoute,
    annotTarget,
    clampMosaicBlock,
    exportAnnotation,
    matchCaptureAnnotKey,
    mosaicSourcePlan,
    wheelStep,
  } from '../models/captureAnnotate.js';
  import {
    normalizeColorValue,
    pushRecentColor,
    rgbToHex,
  } from '../models/palette.js';
  import {
    clampToolSize,
    createDefaultAnnotateStyle,
    getToolSizeSpec,
    parseAnnotateStyle,
    serializeAnnotateStyle,
    stepSize,
  } from '../models/inkSize.js';
  import { TEXT_SIZE_MUL, createInkEngine, itemCount } from '../ink/inkEngine.js';
  import { evidenceBoxes } from '../models/evidence.js';
  import {
    buildMagnifierPatch,
    clientToImagePixel,
    clientToPhysical,
    computeMagnifierPosition,
    computeSampleRect,
    findMonitorForOverlay,
    isPhysicalPointInMonitor,
    isPhysicalRectOverlappingMonitor,
    physicalRectToClient,
    physicalToClient,
    resolveCaptureFrame,
    resolveMonitorGuides,
    shouldPreventCaptureKey,
    toCaptureKeyInput,
    toCapturePointerInput,
    toCaptureWheelInput,
    toggleColorFormat,
  } from '../models/captureOverlay.js';

  /**
   * S07 캡처 오버레이 화면(모니터 1개 = 창 1개).
   * - nativeFrame=true(Windows 네이티브 정지 화면):
   *   바탕은 완전 투명(`<img>`·`--ink-scrim` 없음), 첫 강조(highlight)가 이 모니터와 겹치면
   *   SelectionFrame 테두리·꺾쇠만 표시. 세션 알림 뒤 rAF 2회에 overlayReady(session, {loadMs: 0, paintMs})를
   *   1회 호출하고, 그 뒤에 보이지 않는 `<img crossorigin="anonymous">`로 frameUrl을 늦게 읽어
   *   오프스크린 캔버스 준비가 끝난 시점부터 확대경을 보여 준다.
   * - nativeFrame=false(맥 등):
   *   정지 `<img>` + 50% 어둡게(강조 사각형이 있으면 SelectionFrame으로 밝게 뚫기),
   *   img load → rAF 2회 뒤 overlayReady(session, {loadMs, paintMs}) 1회 호출 후 오프스크린 캔버스 준비.
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   monitorId?: number,
   *   selection?: {
   *     x: number,
   *     y: number,
   *     width: number,
   *     height: number,
   *     state?: 'picking' | 'confirmed' | 'recording' | 'afterimage',
   *   } | null,
   *   class?: string,
   * }}
   */
  let { client, monitorId = 0, selection = null, class: className = '' } = $props();

  /** @type {number | null} */
  let activeSession = $state(null);
  /** @type {'region' | 'window' | 'record'} */
  let sessionKind = $state('region');
  let nativeFrame = $state(false);
  /** @type {import('@screen-ink/client').PhysRect | null} */
  let sessionHighlight = $state(null);
  /** @type {import('@screen-ink/client').CaptureView | null} */
  let captureView = $state(null);
  /** @type {import('@screen-ink/client').OverlayMonitor | null} */
  let monitor = $state(null);
  let frameUrl = $state('');
  let deferredLoad = $state(false);
  /** 이 세션의 모든 모니터(선택 툴바를 어느 모니터에 띄울지) */
  /** @type {import('@screen-ink/client').OverlayMonitor[]} */
  let sessionMonitors = $state([]);
  /** 칸 id → 지금 키(툴팁) */
  /** @type {Record<string, string>} */
  let combos = $state({});
  /** @type {'windows' | 'macos'} */
  let platform = $state('windows');
  let toolbarW = $state(0);
  let toolbarH = $state(0);
  let overToolbar = $state(false);

  /** @type {'hex' | 'rgb'} */
  let colorFormat = $state('hex');
  /** @type {{ x: number, y: number } | null} */
  let cursorCss = $state(null);
  /** @type {{ x: number, y: number } | null} */
  let cursorPhys = $state(null);
  /** @type {[number, number, number]} */
  let rgb = $state([255, 255, 255]);
  /** @type {{ data: Uint8ClampedArray, width: number, height: number } | null} */
  let patch = $state(null);
  let magnifierReady = $state(false);
  let viewportW = $state(typeof window !== 'undefined' ? window.innerWidth : 1920);
  let viewportH = $state(typeof window !== 'undefined' ? window.innerHeight : 1080);

  /** @type {HTMLImageElement | null} */
  let imgEl = $state(null);
  /** @type {HTMLImageElement | null} */
  let loaderImgEl = $state(null);
  /** @type {HTMLDivElement | null} */
  let rootEl = $state(null);

  /** @type {HTMLCanvasElement | null} */
  let offscreenCanvas = null;
  /** @type {CanvasRenderingContext2D | null} */
  let offscreenCtx = null;
  let offscreenW = 0;
  let offscreenH = 0;

  /** @type {number | null} */
  let readyScheduledSession = null;
  /** @type {number | null} */
  let readySentSession = null;
  /** @type {number | null} */
  let deferredLoadedSession = null;
  let sessionReceivedAt = 0;
  let raf1 = 0;
  let raf2 = 0;
  /** @type {ReturnType<typeof setTimeout> | null} */
  let deferTimer = null;
  let moveRaf = 0;
  /** @type {{ clientX: number, clientY: number } | null} */
  let pendingPointer = null;

  // ── 캡처 주석(4.9, R-CAP-28): 판서 엔진을 범위 안 문서로. 결과 판정은 Rust, 주석 층은 부탁받으면 보낸다 ──
  /** @type {string | null} 고른 주석 도구(null = 범위 조정) */
  let annotTool = $state(null);
  /**
   * 이 창의 주석(세션당 하나, 처음 도구를 고를 때 만든다). anchor = 그때 범위 왼쪽 위(물리 px) = 문서 원점.
   * @type {{ session: number, engine: import('../ink/inkEngine.js').InkEngine, anchor: { x: number, y: number }, off: () => void } | null}
   */
  let annot = $state.raw(null);
  let annotCanUndo = $state(false);
  let annotCanRedo = $state(false);
  let annotEditing = $state(false);
  let annotHasMosaic = $state(false);
  let annotColor = $state('R');
  let annotWidth = $state(4);
  let annotBlock = $state(MOSAIC_BLOCK_DEFAULT);
  let annotStyle = $state(createDefaultAnnotateStyle());
  let lastSyncedStyleRaw = '';
  let panelOpen = $state(false);
  let eyedropperActive = $state(false);
  let hasOffscreenPixels = $state(false);
  /** @type {number | null} */
  let toolbarAnchorX = $state(null);
  let badgeW = $state(92);
  let badgeH = $state(24);
  /** 마지막으로 Rust에 알린 주석 개수(0 ↔ n이 바뀔 때만 알린다) */
  let annotReported = 0;
  let annotWheel = 0;
  /** 모자이크 미리보기 원본(정지 이미지를 문서 원점에서 시작하게 옮긴 캔버스) */
  /** @type {HTMLCanvasElement | null} */
  let mosaicSource = $state.raw(null);
  let mosaicSourceKey = '';
  /** 오프스크린(확대경) 이미지가 준비될 때마다 올린다 */
  let frameRev = $state(0);
  let optsW = $state(284);
  let optsH = $state(216);
  let overOpts = $state(false);
  /** @type {Record<string, any>} 판서 설정(색·굵기·떨림 보정 — 주석도 같은 값) */
  let drawSettings = {};
  /** @type {Array<{ id: string, combo: string }>} */
  let shortcutRows = [];

  // ── 확정 직후 주황 잔상 0.4초(R-CAP-26): Rust가 창마다 screen-ink://afterimage(그 창 CSS 사각형) ──
  /** @type {{ token: number, x: number, y: number, width: number, height: number } | null} */
  let afterimage = $state(null);
  let afterimageSeq = 0;
  /** @type {ReturnType<typeof setTimeout> | null} */
  let afterimageTimer = null;
  /** 세션이 끝난 뒤에도 잔상 좌표를 바꿀 수 있게(가짜 클라이언트처럼 css가 없는 알림) */
  /** @type {import('@screen-ink/client').OverlayMonitor | null} */
  let lastMonitor = null;

  function nowMs() {
    return typeof performance !== 'undefined' && typeof performance.now === 'function'
      ? performance.now()
      : Date.now();
  }

  /** @param {number} ms */
  function roundMs(ms) {
    return Math.max(0, Math.round(ms * 10) / 10);
  }

  function cancelPendingWork() {
    if (raf1 && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(raf1);
    if (raf2 && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(raf2);
    if (moveRaf && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(moveRaf);
    if (deferTimer !== null) clearTimeout(deferTimer);
    raf1 = 0;
    raf2 = 0;
    moveRaf = 0;
    deferTimer = null;
    pendingPointer = null;
  }

  function releaseOffscreen() {
    if (offscreenCanvas) {
      offscreenCanvas.width = 0;
      offscreenCanvas.height = 0;
    }
    offscreenCanvas = null;
    offscreenCtx = null;
    offscreenW = 0;
    offscreenH = 0;
    hasOffscreenPixels = false;
  }

  function clearOverlay() {
    // 모드가 끝남 — 주석은 버린다(Rust 쪽 세션도 끝났으니 알리지 않음)
    dropAnnot(false);
    cancelPendingWork();
    releaseOffscreen();
    if (imgEl) {
      imgEl.removeAttribute('src');
    }
    if (loaderImgEl) {
      loaderImgEl.removeAttribute('src');
    }
    activeSession = null;
    nativeFrame = false;
    sessionHighlight = null;
    captureView = null;
    monitor = null;
    sessionMonitors = [];
    overToolbar = false;
    frameUrl = '';
    deferredLoad = false;
    cursorCss = null;
    cursorPhys = null;
    patch = null;
    rgb = [255, 255, 255];
    magnifierReady = false;
    colorFormat = 'hex';
    readyScheduledSession = null;
    deferredLoadedSession = null;
  }

  /**
   * @param {number} cssX
   * @param {number} cssY
   */
  function sampleAtCss(cssX, cssY) {
    if (!offscreenCtx || offscreenW <= 0 || offscreenH <= 0) return;
    const vw = rootEl?.clientWidth || viewportW;
    const vh = rootEl?.clientHeight || viewportH;
    const { x: px, y: py } = clientToImagePixel(cssX, cssY, vw, vh, offscreenW, offscreenH);
    const rect = computeSampleRect(px, py, offscreenW, offscreenH);
    if (rect.sw <= 0 || rect.sh <= 0) return;
    try {
      const imgData = offscreenCtx.getImageData(rect.sx, rect.sy, rect.sw, rect.sh);
      const built = buildMagnifierPatch(imgData.data, rect);
      patch = { data: built.data, width: built.width, height: built.height };
      rgb = built.centerRgb;
    } catch {
      // 교차 출처 등으로 픽셀 읽기가 막혀도 좌표·오버레이 동작은 유지한다.
      releaseOffscreen();
    }
  }

  /**
   * 첫 그림이 끝난 뒤에만 오프스크린 캔버스를 한 번 만든다.
   * @param {number} session
   * @param {HTMLImageElement | null} sourceImg
   * @param {boolean} requirePixels  true면 픽셀을 읽을 수 있을 때만 확대경을 연다(nativeFrame 늦은 읽기)
   */
  function initOffscreenCanvas(session, sourceImg, requirePixels = false) {
    if (session !== activeSession) return;
    let drawn = false;
    if (
      sourceImg &&
      sourceImg.naturalWidth > 0 &&
      sourceImg.naturalHeight > 0 &&
      typeof document !== 'undefined'
    ) {
      const canvas = document.createElement('canvas');
      canvas.width = sourceImg.naturalWidth;
      canvas.height = sourceImg.naturalHeight;
      const ctx = canvas.getContext('2d', { willReadFrequently: true });
      if (ctx) {
        try {
          ctx.drawImage(sourceImg, 0, 0);
          offscreenCanvas = canvas;
          offscreenCtx = ctx;
          offscreenW = canvas.width;
          offscreenH = canvas.height;
          drawn = true;
        } catch {
          releaseOffscreen();
        }
      }
    }
    if (drawn) {
      hasOffscreenPixels = true;
      frameRev += 1; // 모자이크 미리보기 원본을 다시 만들 수 있다
    }
    if (requirePixels && !drawn) {
      magnifierReady = false;
      return;
    }
    magnifierReady = true;
    if (cursorCss) {
      sampleAtCss(cursorCss.x, cursorCss.y);
    }
  }

  /**
   * nativeFrame=true에서 늦게 읽은 숨겨진 `<img>`가 끝나면 오프스크린 캔버스에 옮기고 DOM 이미지를 비운다.
   * @param {number | null} session
   */
  function handleDeferredImgLoad(session) {
    if (session === null || session !== activeSession) return;
    if (deferredLoadedSession === session) return;
    deferredLoadedSession = session;
    const el = loaderImgEl;
    initOffscreenCanvas(session, el, true);
    if (el) {
      el.removeAttribute('src');
    }
    deferredLoad = false;
  }

  /** @param {number | null} session */
  function handleDeferredImgError(session) {
    if (session === null || session !== activeSession) return;
    deferredLoadedSession = session;
    if (loaderImgEl) {
      loaderImgEl.removeAttribute('src');
    }
    deferredLoad = false;
    magnifierReady = false;
  }

  /**
   * 다음 animation frame 2회를 기다렸다가 overlayReady를 세션당 한 번 호출한다.
   * @param {number | null} session
   * @param {{ isNative: boolean, hasImage: boolean, loadMs: number, paintStartAt: number }} opts
   */
  function scheduleReady(session, { isNative, hasImage, loadMs, paintStartAt }) {
    if (session === null || session !== activeSession) return;
    if (readyScheduledSession === session || readySentSession === session) return;
    readyScheduledSession = session;

    const done = () => {
      if (session !== activeSession || readySentSession === session) return;
      readySentSession = session;
      const paintMs = roundMs(nowMs() - paintStartAt);
      client.overlayReady(session, { loadMs: roundMs(loadMs), paintMs });
      deferTimer = setTimeout(() => {
        deferTimer = null;
        if (session !== activeSession) return;
        if (isNative) {
          if (frameUrl) {
            deferredLoad = true;
          }
        } else if (hasImage) {
          initOffscreenCanvas(session, imgEl, false);
        } else {
          magnifierReady = true;
        }
      }, 0);
    };

    if (typeof requestAnimationFrame !== 'function') {
      done();
      return;
    }

    raf1 = requestAnimationFrame(() => {
      raf1 = 0;
      if (session !== activeSession) return;
      raf2 = requestAnimationFrame(() => {
        raf2 = 0;
        done();
      });
    });
  }

  // ─────────────── 캡처 주석(4.9) ───────────────

  /** @param {number} w @param {number} h */
  function createCanvas(w, h) {
    const c = document.createElement('canvas');
    c.width = w;
    c.height = h;
    return c;
  }

  function releaseMosaicSource() {
    if (mosaicSource) {
      mosaicSource.width = 0;
      mosaicSource.height = 0;
    }
    mosaicSource = null;
    mosaicSourceKey = '';
  }

  /**
   * 엔진이 바뀔 때마다: 단추 상태 + 주석 개수가 0 ↔ n으로 바뀌면 Rust에 알린다(결과 경로를 고름).
   * @param {import('../ink/inkEngine.js').InkEngine} engine
   */
  function syncAnnot(engine) {
    const a = annot;
    if (!a || a.engine !== engine) return;
    const n = itemCount(engine.doc);
    annotCanUndo = engine.canUndo;
    annotCanRedo = engine.canRedo;
    annotEditing = Boolean(engine.editing);
    annotHasMosaic = engine.items.some((it) => it.kind === 'mosaic');
    annotColor = engine.color;
    annotWidth = engine.widthPx;
    if (n > 0 !== annotReported > 0) {
      annotReported = n;
      if (monitor && activeSession === a.session && typeof client.captureAnnotations === 'function') {
        client.captureAnnotations(a.session, monitor.id, n);
      }
    }
  }

  /** 이 세션의 주석 엔진(없으면 지금 범위로 만든다). */
  function ensureAnnot() {
    if (annot && annot.session === activeSession) return annot;
    const rect = captureView?.rect;
    if (activeSession === null || !monitor || !rect || !(rect.w > 0) || !(rect.h > 0)) return null;
    const engine = createInkEngine({
      docOptions: annotDocOptions(activeSession, rect, monitor),
      platform,
      shortcuts: shortcutRows.length ? shortcutRows : undefined,
      settings: annotEngineSettings(drawSettings),
    });
    const a = { session: activeSession, engine, anchor: { x: rect.x, y: rect.y }, off: () => {} };
    annot = a;
    annotReported = 0;
    a.off = engine.subscribe(syncAnnot);
    syncAnnot(engine);
    return a;
  }

  /** @param {any} next */
  function persistAnnotStyle(next) {
    annotStyle = next;
    const raw = serializeAnnotateStyle(next);
    lastSyncedStyleRaw = raw;
    if (typeof client?.updateSetting === 'function') {
      Promise.resolve(client.updateSetting('annotate.style', raw)).catch(() => {});
    }
  }

  /**
   * 도구별 저장된 색·크기를 엔진과 모자이크 상태에 반영한다.
   * @param {string} toolId
   * @param {import('../ink/inkEngine.js').InkEngine} engine
   * @param {import('../models/inkSize.js').AnnotateStyleState} style
   */
  function applyToolStyleToEngine(toolId, engine, style) {
    const spec = getToolSizeSpec(toolId);
    const entry = style.tools[toolId] ?? { color: style.lastColor, size: spec.defaultVal };
    const size = clampToolSize(entry.size, spec);
    if (toolId === 'mosaic') {
      annotBlock = size;
      return;
    }
    if (toolId === 'eraser') {
      engine.widthPx = size / 3;
    } else if (toolId === 'text') {
      engine.widthPx = size / TEXT_SIZE_MUL;
    } else {
      engine.widthPx = size;
    }
    if (entry.color) {
      const norm = normalizeColorValue(entry.color);
      if (norm) engine.setColor(norm);
    }
  }

  /** @param {boolean} report  지금 세션이 이어지면 Rust에 "0개"를 알린다(범위를 버림) */
  function dropAnnot(report) {
    const a = annot;
    if (!a) return;
    a.off();
    if (report && annotReported > 0 && monitor && activeSession === a.session && typeof client.captureAnnotations === 'function') {
      client.captureAnnotations(a.session, monitor.id, 0);
    }
    annot = null;
    annotTool = null;
    annotCanUndo = false;
    annotCanRedo = false;
    annotEditing = false;
    annotHasMosaic = false;
    annotReported = 0;
    annotWheel = 0;
    panelOpen = false;
    eyedropperActive = false;
    overOpts = false;
    releaseMosaicSource();
  }

  /** 입력 중인 글자를 확정(결과·도구 바꾸기 전에) */
  function flushAnnotText() {
    const a = annot;
    if (a?.engine.editing) {
      a.engine.commitText();
      a.engine.notify();
    }
  }

  /**
   * 툴바 도구 단추: 같은 도구 = 해제(범위 조정으로). 새 도구 = 저장된 스타일 반영 + 색·굵기 패널 열기.
   * @param {string} id
   */
  function selectTool(id) {
    const next = toggleAnnotTool(annotTool, id);
    if (next === annotTool) return;
    eyedropperActive = false;
    if (next) {
      const a = ensureAnnot();
      if (!a) return;
      const target = annotTarget(next);
      if (target.engineTool) a.engine.setTool(target.engineTool);
      else if (a.engine.editing) a.engine.commitText();
      applyToolStyleToEngine(next, a.engine, annotStyle);
      a.engine.notify();
      panelOpen = true;
    } else {
      flushAnnotText();
      panelOpen = false;
    }
    annotTool = next;
  }

  function toggleStylePanel() {
    if (!annotTool) {
      selectTool('pen');
      return;
    }
    panelOpen = !panelOpen;
    if (!panelOpen) eyedropperActive = false;
  }

  /**
   * @param {string} codeOrHex
   * @param {{ recordRecent?: boolean }} [opts]
   */
  function setAnnotColor(codeOrHex, { recordRecent = false } = {}) {
    const norm = normalizeColorValue(codeOrHex);
    if (!norm) return;
    const a = ensureAnnot();
    if (a) {
      a.engine.setColor(/** @type {any} */ (norm));
      a.engine.notify();
    }
    const key = annotTool && annotTool !== 'mosaic' && annotTool !== 'eraser' ? annotTool : 'pen';
    const spec = getToolSizeSpec(key);
    const prev = annotStyle.tools[key] ?? { size: spec.defaultVal };
    const recentColors =
      recordRecent || norm.startsWith('#') ? pushRecentColor(annotStyle.recentColors, norm) : annotStyle.recentColors;
    persistAnnotStyle({
      ...annotStyle,
      lastColor: norm,
      recentColors,
      tools: { ...annotStyle.tools, [key]: { ...prev, color: norm } },
    });
  }

  /** @param {number} val */
  function setAnnotSize(val) {
    const key = annotTool ?? 'pen';
    const spec = getToolSizeSpec(key);
    const clamped = clampToolSize(val, spec);
    if (key === 'mosaic') {
      annotBlock = clamped;
    } else {
      const a = ensureAnnot();
      if (a) {
        if (key === 'text') {
          a.engine.widthPx = clamped / TEXT_SIZE_MUL;
          if (a.engine.editing) {
            a.engine.editing = { ...a.engine.editing, fontSize: clamped / a.engine.view.s };
          }
        } else if (key === 'eraser') {
          a.engine.widthPx = clamped / 3;
        } else {
          a.engine.widthPx = clamped;
        }
        a.engine.revs.active++;
        a.engine.notify();
      }
    }
    const prev = annotStyle.tools[key] ?? { size: spec.defaultVal };
    persistAnnotStyle({
      ...annotStyle,
      tools: { ...annotStyle.tools, [key]: { ...prev, size: clamped } },
    });
  }

  function toggleEyedropper() {
    if (!hasOffscreenPixels) return;
    eyedropperActive = !eyedropperActive;
  }

  /** @param {'undo' | 'redo'} dir */
  function annotHistory(dir) {
    const a = annot;
    if (!a) return;
    a.engine.runAction(dir === 'undo' ? 'd-undo' : 'd-redo');
    a.engine.notify();
  }

  /** @param {string} name */
  function handleToolbarCommand(name) {
    if (name === 'annot.undo') return annotHistory('undo');
    if (name === 'annot.redo') return annotHistory('redo');
    // 쓰던 글자를 먼저 확정 → 개수 알림이 결과 단추보다 먼저 Rust에 닿는다
    flushAnnotText();
    sendCaptureInput({ type: 'command', name });
  }

  /**
   * Ctrl(맥 Cmd)+휠: 현재 도구 크기 한 단계 증감.
   * @param {WheelEvent} e
   */
  function annotWheelStep(e) {
    const k = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? viewportH || 800 : 1;
    const r = wheelStep(annotWheel, e.deltaY * k);
    annotWheel = r.acc;
    if (!r.dir) return;
    const key = annotTool ?? 'pen';
    const spec = getToolSizeSpec(key);
    const cur = key === 'mosaic' ? annotBlock : annotStyle.tools[key]?.size ?? spec.defaultVal;
    setAnnotSize(stepSize(cur, r.dir, spec, { large: e.shiftKey }));
  }

  /**
   * Rust의 부탁(주석이 있는 범위의 결과): 투명 주석 층 + 모자이크 사각형을 원본 바이트로 보낸다.
   * 주석을 쥔 창만 답한다.
   * @param {import('@screen-ink/client').CaptureCommitRequest} req
   */
  function handleCommitRequest(req) {
    const a = annot;
    if (!a || !monitor || activeSession === null || req.session !== activeSession || a.session !== req.session) return;
    if (typeof client.captureCommitAnnotated !== 'function') return;
    flushAnnotText();
    const items = a.engine.items;
    if (!items.length) return; // 개수 0 알림이 먼저 갔다 — Rust가 주석 없이 낸다
    const out = exportAnnotation(items, {
      anchor: a.anchor,
      rect: req.rect,
      scale: monitor.scale > 0 ? monitor.scale : 1,
      createCanvas,
    });
    client.captureCommitAnnotated({ session: req.session, seq: req.seq, action: req.action, w: out.w, h: out.h, mosaics: out.mosaics }, out.rgba);
  }

  // 모자이크를 쓰면(도구를 골랐거나 이미 있음) 미리보기 원본을 한 번 만든다(정지 이미지가 준비된 뒤)
  $effect(() => {
    const a = annot;
    const mon = monitor;
    const needed = Boolean(a) && (annotTool === 'mosaic' || annotHasMosaic);
    const rev = frameRev;
    // 만든 결과(mosaicSource)에는 기대지 않는다
    untrack(() => {
      if (!a || !mon || !needed || !offscreenCanvas) return;
      const key = `${a.session}|${a.anchor.x}|${a.anchor.y}|${rev}`;
      if (key === mosaicSourceKey && mosaicSource) return;
      const plan = mosaicSourcePlan(a.anchor, mon);
      if (!plan) return;
      const c = createCanvas(plan.w, plan.h);
      const ctx = c.getContext('2d');
      if (!ctx) return;
      try {
        ctx.drawImage(offscreenCanvas, plan.dx, plan.dy, plan.dw, plan.dh);
      } catch {
        c.width = 0;
        c.height = 0;
        return;
      }
      releaseMosaicSource();
      mosaicSource = c;
      mosaicSourceKey = key;
    });
  });

  // ─────────────── 주황 잔상(R-CAP-26) ───────────────

  function reducedMotion() {
    try {
      return typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
    } catch {
      return false;
    }
  }

  function clearAfterimage() {
    if (afterimageTimer !== null) clearTimeout(afterimageTimer);
    afterimageTimer = null;
    afterimage = null;
  }

  /**
   * 확정 직후 고른 자리에 0.4초. 창은 Rust가 클릭 통과로 띄우고 400 ms 뒤 숨긴다.
   * prefers-reduced-motion: reduce면 그리지 않는다(10 §6).
   * @param {import('@screen-ink/client').AfterimageEvent} ev
   */
  function showAfterimage(ev) {
    if (!ev?.rect) return;
    if (typeof ev.monitorId === 'number' && ev.monitorId !== monitorId) return;
    // 이미 새 세션이 떠 있으면 지난 잔상은 버린다
    if (activeSession !== null && ev.session !== activeSession) return;
    clearAfterimage();
    if (reducedMotion()) return;
    const mon = monitor ?? lastMonitor;
    const css = ev.css
      ? { x: ev.css.x, y: ev.css.y, width: ev.css.w, height: ev.css.h }
      : mon && isPhysicalRectOverlappingMonitor(ev.rect, mon)
        ? physicalRectToClient(ev.rect, mon)
        : null;
    if (!css) return;
    // 고르기 화면(어둡게·툴바)이 아직 남아 있으면 걷고 잔상만
    if (activeSession !== null) clearOverlay();
    const token = ++afterimageSeq;
    afterimage = { token, ...css };
    afterimageTimer = setTimeout(
      () => {
        afterimageTimer = null;
        if (afterimage?.token === token) afterimage = null;
      },
      Math.max(0, ev.durationMs ?? 400) + 40,
    );
  }

  function handleMainImgLoad() {
    if (activeSession === null || nativeFrame) return;
    const loadedAt = nowMs();
    scheduleReady(activeSession, {
      isNative: false,
      hasImage: true,
      loadMs: loadedAt - sessionReceivedAt,
      paintStartAt: loadedAt,
    });
  }

  function handleMainImgError() {
    if (activeSession === null || nativeFrame) return;
    scheduleReady(activeSession, {
      isNative: false,
      hasImage: false,
      loadMs: 0,
      paintStartAt: sessionReceivedAt,
    });
  }

  $effect(() => {
    const targetMonitorId = monitorId;
    const off = client.subscribe((e) => {
      if (e.type === 'overlay') {
        // 새 세션: 지난 주석·잔상은 버린다
        dropAnnot(false);
        clearAfterimage();
        cancelPendingWork();
        releaseOffscreen();
        patch = null;
        rgb = [255, 255, 255];
        magnifierReady = false;
        deferredLoad = false;
        deferredLoadedSession = null;
        colorFormat = 'hex';

        const mon = findMonitorForOverlay(e.overlay, targetMonitorId);
        if (!mon) {
          clearOverlay();
          return;
        }

        const isNative = Boolean(e.overlay.nativeFrame);
        const nextUrl = mon.frameUrl || '';
        if (imgEl && (isNative || frameUrl !== nextUrl)) {
          imgEl.removeAttribute('src');
        }
        if (loaderImgEl) {
          loaderImgEl.removeAttribute('src');
        }

        sessionReceivedAt = nowMs();
        lastMonitor = mon;
        activeSession = e.overlay.session;
        sessionKind = e.overlay.kind;
        nativeFrame = isNative;
        sessionHighlight = e.overlay.highlight ?? null;
        captureView = null;
        monitor = mon;
        sessionMonitors = Array.isArray(e.overlay.monitors) ? e.overlay.monitors : [];
        overToolbar = false;
        frameUrl = nextUrl;
        readyScheduledSession = null;
        readySentSession = null;

        if (e.overlay.cursor && isPhysicalPointInMonitor(e.overlay.cursor.x, e.overlay.cursor.y, mon)) {
          cursorPhys = { x: Math.round(e.overlay.cursor.x), y: Math.round(e.overlay.cursor.y) };
          cursorCss = physicalToClient(e.overlay.cursor.x, e.overlay.cursor.y, mon);
        } else {
          cursorPhys = null;
          cursorCss = null;
        }

        if (isNative) {
          scheduleReady(e.overlay.session, {
            isNative: true,
            hasImage: false,
            loadMs: 0,
            paintStartAt: sessionReceivedAt,
          });
        } else if (!nextUrl) {
          scheduleReady(e.overlay.session, {
            isNative: false,
            hasImage: false,
            loadMs: 0,
            paintStartAt: sessionReceivedAt,
          });
        }
      } else if (e.type === 'capture') {
        if (activeSession !== null && e.view?.session === activeSession) {
          captureView = e.view;
          // 우클릭으로 범위를 버리면(다시 고르기) 주석도 버린다
          if (e.view.phase === 'hover' && annot) dropAnnot(true);
        }
      } else if (e.type === 'captureCommit') {
        handleCommitRequest(e.request);
      } else if (e.type === 'afterimage') {
        showAfterimage(e.afterimage);
      } else if (e.type === 'draw') {
        // 잔상 0.4초 중에 판서가 열림([판서로]·Alt+1) — 판서 화면 위에 주황 틀을 남기지 않는다
        clearAfterimage();
      } else if (e.type === 'state') {
        applyStateKeys(e.state);
        const mode = e.state?.mode;
        if (!mode || mode.mode !== 'capture') {
          clearOverlay();
        } else if (activeSession !== null && mode.session !== activeSession) {
          clearOverlay();
        }
      }
    });

    return () => {
      off();
      clearOverlay();
      clearAfterimage();
    };
  });

  // 이미 브라우저 캐시에 들어 있던 이미지라 onload가 앞서 지나간 경우에도 overlayReady를 보장한다.
  $effect(() => {
    const session = activeSession;
    const isNative = nativeFrame;
    const url = frameUrl;
    const el = imgEl;
    if (
      !isNative &&
      session !== null &&
      url &&
      el &&
      el.complete &&
      el.naturalWidth > 0 &&
      readyScheduledSession !== session
    ) {
      const loadedAt = nowMs();
      scheduleReady(session, {
        isNative: false,
        hasImage: true,
        loadMs: loadedAt - sessionReceivedAt,
        paintStartAt: loadedAt,
      });
    }
  });

  // nativeFrame 늦은 읽기 이미지가 이미 캐시에 있어 onload를 놓친 경우를 처리한다.
  $effect(() => {
    const session = activeSession;
    const isNative = nativeFrame;
    const loading = deferredLoad;
    const el = loaderImgEl;
    if (
      isNative &&
      loading &&
      session !== null &&
      el &&
      el.complete &&
      el.naturalWidth > 0 &&
      deferredLoadedSession !== session
    ) {
      handleDeferredImgLoad(session);
    }
  });

  /**
   * 설정 상태에서 툴팁에 쓸 지금 키·플랫폼을 꺼낸다.
   * @param {any} state
   */
  function applyStateKeys(state) {
    if (!state) return;
    syncLang(state);
    if (typeof location !== 'undefined') {
      const qLang = new URLSearchParams(location.search).get('lang');
      if (qLang) setLang(qLang);
    }
    if (Array.isArray(state.shortcuts)) combos = comboMap(state.shortcuts);
    if (typeof state.platform === 'string') platform = state.platform === 'macos' || state.platform === 'mac' ? 'macos' : 'windows';
    // 캡처 주석: 판서 설정(색·굵기)·판서 키(실행 취소·다시 실행) + annotate.style
    if (state.settings && typeof state.settings === 'object') {
      drawSettings = state.settings;
      const rawStyle = state.settings['annotate.style'];
      if (typeof rawStyle === 'string' && rawStyle !== lastSyncedStyleRaw) {
        lastSyncedStyleRaw = rawStyle;
        annotStyle = parseAnnotateStyle(rawStyle);
        if (annotTool === 'mosaic') annotBlock = annotStyle.tools.mosaic?.size ?? annotBlock;
      }
    }
    if (Array.isArray(state.shortcuts)) {
      shortcutRows = state.shortcuts;
      annot?.engine.setShortcuts(state.shortcuts);
    }
  }

  $effect(() => {
    if (typeof client?.getState !== 'function') return;
    let alive = true;
    Promise.resolve(client.getState())
      .then((s) => {
        if (alive) applyStateKeys(s);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  });

  /** @param {import('@screen-ink/client').CaptureInput | null} input */
  function sendCaptureInput(input) {
    if (!input || activeSession === null || typeof client.captureInput !== 'function') return;
    client.captureInput(input);
  }

  function flushPointerMove() {
    moveRaf = 0;
    const pt = pendingPointer;
    pendingPointer = null;
    if (!pt || !monitor || activeSession === null) return;

    const rect = rootEl?.getBoundingClientRect();
    const localX = rect ? pt.clientX - rect.left : pt.clientX;
    const localY = rect ? pt.clientY - rect.top : pt.clientY;
    if (rect) {
      viewportW = rect.width || viewportW;
      viewportH = rect.height || viewportH;
    }

    cursorCss = { x: localX, y: localY };
    cursorPhys = clientToPhysical(localX, localY, monitor);

    if (magnifierReady && offscreenCtx) {
      sampleAtCss(localX, localY);
    }

    sendCaptureInput(toCapturePointerInput('move', pt, monitor, rect));
  }

  /** @param {PointerEvent} e */
  function handlePointerMove(e) {
    if (activeSession === null || !monitor) return;
    if (eyedropperActive) {
      const rect = rootEl?.getBoundingClientRect();
      const localX = rect ? e.clientX - rect.left : e.clientX;
      const localY = rect ? e.clientY - rect.top : e.clientY;
      cursorCss = { x: localX, y: localY };
      cursorPhys = clientToPhysical(localX, localY, monitor);
      if (magnifierReady && offscreenCtx) sampleAtCss(localX, localY);
      return;
    }
    // 주석 도구를 고른 동안 범위 밖 움직임은 고르기에 보내지 않는다(범위 안은 주석 층이 받음)
    if (annotTool) return;
    pendingPointer = { clientX: e.clientX, clientY: e.clientY };
    if (!moveRaf && typeof requestAnimationFrame === 'function') {
      moveRaf = requestAnimationFrame(flushPointerMove);
    } else if (typeof requestAnimationFrame !== 'function') {
      flushPointerMove();
    }
  }

  /** @param {PointerEvent} e */
  function handlePointerDown(e) {
    if (activeSession === null || !monitor) return;
    if (eyedropperActive) {
      e.preventDefault();
      e.stopPropagation();
      if (e.button === 2) {
        eyedropperActive = false;
        return;
      }
      if (e.button === 0) {
        const rect = rootEl?.getBoundingClientRect();
        const localX = rect ? e.clientX - rect.left : e.clientX;
        const localY = rect ? e.clientY - rect.top : e.clientY;
        if (hasOffscreenPixels && offscreenCtx) sampleAtCss(localX, localY);
        const hex = rgbToHex(rgb);
        setAnnotColor(hex, { recordRecent: true });
        eyedropperActive = false;
        panelOpen = true;
      }
      return;
    }
    if (annotTool) {
      // 도구를 고른 동안 범위 밖 클릭은 범위를 바꾸지 않는다. 우클릭 = 도구 해제(범위 조정으로)
      if (e.button === 2) {
        e.preventDefault();
        selectTool(annotTool);
      }
      return;
    }
    const rect = rootEl?.getBoundingClientRect();
    const input = toCapturePointerInput('down', e, monitor, rect);
    if (!input) return;
    if (e.button === 2) {
      e.preventDefault();
    }
    // 끌어서 옆 모니터로 나가도 이동·놓기를 이 창이 계속 받는다(모니터를 넘는 범위)
    if (e.button === 0) {
      try {
        rootEl?.setPointerCapture?.(e.pointerId);
      } catch {
        // 포인터가 이미 없어짐 — 창 안에서만 끌린다
      }
    }
    const localX = rect ? e.clientX - rect.left : e.clientX;
    const localY = rect ? e.clientY - rect.top : e.clientY;
    cursorCss = { x: localX, y: localY };
    cursorPhys = { x: input.x, y: input.y };
    if (magnifierReady && offscreenCtx) {
      sampleAtCss(localX, localY);
    }
    sendCaptureInput(input);
  }

  /** @param {PointerEvent} e */
  function handlePointerUp(e) {
    if (activeSession === null || !monitor) return;
    if (annotTool) return;
    if (pendingPointer) {
      if (moveRaf && typeof cancelAnimationFrame === 'function') {
        cancelAnimationFrame(moveRaf);
        moveRaf = 0;
      }
      flushPointerMove();
    }
    const rect = rootEl?.getBoundingClientRect();
    const input = toCapturePointerInput('up', e, monitor, rect);
    if (!input) return;
    sendCaptureInput(input);
  }

  /** @param {MouseEvent} e */
  function handleDblClick(e) {
    if (activeSession === null || !monitor) return;
    if (annotTool) return; // 도구를 고른 동안 더블클릭은 복사가 아니다
    const rect = rootEl?.getBoundingClientRect();
    const input = toCapturePointerInput('dblclick', e, monitor, rect);
    if (!input) return;
    sendCaptureInput(input);
  }

  /** @param {WheelEvent} e */
  function handleWheel(e) {
    if (activeSession === null || !monitor) return;
    if (annotTool) {
      // 주석 중 휠: Ctrl(맥 Cmd)+휠 = 굵기·모자이크 칸, 그냥 휠은 삼킨다(확정 뒤 범위는 휠로 바뀌지 않음)
      e.preventDefault();
      if (e.ctrlKey || e.metaKey) annotWheelStep(e);
      return;
    }
    const input = toCaptureWheelInput(e);
    if (!input) return;
    e.preventDefault();
    sendCaptureInput(input);
  }

  $effect(() => {
    const el = rootEl;
    if (!el) return;
    el.addEventListener('wheel', handleWheel, { passive: false });
    return () => el.removeEventListener('wheel', handleWheel);
  });

  function handlePointerLeave() {
    if (moveRaf && typeof cancelAnimationFrame === 'function') {
      cancelAnimationFrame(moveRaf);
      moveRaf = 0;
    }
    pendingPointer = null;
    cursorCss = null;
    cursorPhys = null;
  }

  /** @param {MouseEvent} e */
  function handleContextMenu(e) {
    if (activeSession === null) return;
    e.preventDefault();
  }

  /** @param {KeyboardEvent} e */
  function handleKeydown(e) {
    if (activeSession === null) return;
    // 주석 글자 상자 안 키는 상자가 맡는다(Enter 복사·Esc 취소로 새지 않게)
    if (typeof HTMLTextAreaElement !== 'undefined' && e.target instanceof HTMLTextAreaElement) return;
    const annotKey = matchCaptureAnnotKey(e, {
      editing: annotEditing,
      eyedropper: eyedropperActive,
      confirmed: captureView?.phase === 'confirmed',
      toolActive: Boolean(annotTool),
      panelOpen: toolbarVisible && panelOpen,
    });
    if (annotKey) {
      if (annotKey.type === 'text') return;
      e.preventDefault();
      if (annotKey.type === 'cancelEyedropper') {
        eyedropperActive = false;
      } else if (annotKey.type === 'tool') {
        selectTool(annotKey.id);
      } else if (annotKey.type === 'color') {
        setAnnotColor(annotKey.code);
      } else if (annotKey.type === 'size') {
        const key = annotTool ?? 'pen';
        const spec = getToolSizeSpec(key);
        const cur = key === 'mosaic' ? annotBlock : annotStyle.tools[key]?.size ?? spec.defaultVal;
        setAnnotSize(stepSize(cur, annotKey.dir, spec, { large: annotKey.large }));
      }
      return;
    }
    if (annot) {
      const route = annotKeyRoute(e, { editing: annotEditing, toolActive: Boolean(annotTool), keymap: annot.engine.keymap, platform });
      if (route === 'text') return;
      if (route === 'release') {
        // 도구를 고른 상태의 첫 Esc = 도구만 해제(주석 유지). 도구가 없을 때 Esc = 캡처 취소(Rust)
        e.preventDefault();
        if (annotTool) selectTool(annotTool);
        return;
      }
      if (route === 'undo' || route === 'redo') {
        e.preventDefault();
        annotHistory(route);
        return;
      }
      if (route === 'swallow') {
        e.preventDefault();
        return;
      }
    }
    if (e.key === 'Shift' && !e.repeat && !e.ctrlKey && !e.altKey && !e.metaKey) {
      colorFormat = toggleColorFormat(colorFormat);
    }
    if (shouldPreventCaptureKey(e)) {
      e.preventDefault();
    }
    sendCaptureInput(toCaptureKeyInput(e));
  }

  function handleResize() {
    if (typeof window === 'undefined') return;
    viewportW = window.innerWidth;
    viewportH = window.innerHeight;
  }

  let activeFrame = $derived.by(() => {
    if (selection) {
      return {
        x: selection.x,
        y: selection.y,
        width: selection.width,
        height: selection.height,
        state: selection.state ?? 'picking',
        instant: false,
      };
    }
    return resolveCaptureFrame(captureView, sessionHighlight, monitor);
  });

  let activeGuides = $derived(resolveMonitorGuides(captureView?.guides, monitor));
  // 숨은 "인식 근거 보기"(capture.evidence)
  let evidence = $derived(evidenceBoxes(captureView?.evidence, monitor));

  // 선택 툴바(R-CAP-27): 확정한 범위의 오른쪽 아래 귀가 있는 모니터에만
  let toolbarVisible = $derived(
    Boolean(
      captureView?.phase === 'confirmed' &&
        monitor &&
        activeFrame &&
        toolbarOwner(captureView.rect, sessionMonitors.length ? sessionMonitors : [monitor]) === monitor.id,
    ),
  );
  let toolbarTips = $derived.by(() => {
    /** @type {Record<string, string>} */
    const tips = {};
    for (const item of [...TOOLBAR_ACTIONS, ...TOOLBAR_HISTORY]) {
      tips[item.id] = toolbarTooltip(item, combos, platform, item.labelKey ? t(item.labelKey) : item.label);
    }
    for (const tool of TOOLBAR_ANNOTATE_TOOLS) {
      tips[tool.id] = toolbarTooltip(tool, combos, platform, tool.labelKey ? t(tool.labelKey) : tool.label);
    }
    tips[TOOLBAR_STYLE_TRIGGER.id] = t(TOOLBAR_STYLE_TRIGGER.labelKey);
    return tips;
  });
  /** @type {{ w: number, h: number }} */
  let toolbarSize = $derived({
    w: toolbarW || estimateToolbarWidth([TOOLBAR_ANNOTATE_TOOLS, [TOOLBAR_STYLE_TRIGGER], TOOLBAR_HISTORY, TOOLBAR_ACTIONS]),
    h: toolbarH || 40,
  });
  let toolbarPos = $derived(
    activeFrame
      ? placeToolbar(activeFrame, { w: viewportW, h: viewportH }, toolbarSize)
      : /** @type {{ x: number, y: number, side: 'below' | 'above' | 'inside' }} */ ({ x: 0, y: 0, side: 'below' }),
  );
  // 색·굵기 패널(R2-FIX-11): 고른 도구 단추 위에 붙는 작은 카드
  let activeToolSize = $derived.by(() => {
    const key = annotTool ?? 'pen';
    const spec = getToolSizeSpec(key);
    if (key === 'mosaic') return clampToolSize(annotBlock, spec);
    return clampToolSize(annotStyle.tools[key]?.size ?? spec.defaultVal, spec);
  });
  let activeToolColor = $derived.by(() => {
    const key = annotTool ?? 'pen';
    if (key === 'mosaic' || key === 'eraser') return annotStyle.lastColor;
    return annotStyle.tools[key]?.color ?? annotColor ?? annotStyle.lastColor;
  });
  let optsPos = $derived(
    placeToolOptions(toolbarPos, toolbarSize, { w: optsW || 284, h: optsH || 216 }, { w: viewportW, h: viewportH }, toolbarAnchorX),
  );
  let sizeBadgeText = $derived(activeFrame && captureView?.rect ? formatSizeBadge(captureView.rect) : '');
  let sizeBadgePos = $derived(
    activeFrame ? placeSizeBadge(activeFrame, { w: viewportW, h: viewportH }, { w: badgeW, h: badgeH }) : { x: 0, y: 0, inside: false },
  );
  let stageHintText = $derived.by(() => {
    if (eyedropperActive) return t('capture.hint.eyedropper');
    if (annotTool) return t('capture.hint.annotating');
    if (captureView?.phase === 'confirmed') {
      const copyKey = formatComboReadable(combos['c-copy'] || 'Enter', platform) || 'Enter';
      return t('capture.hint.confirmed', { copyKey });
    }
    return t('capture.hint.picking');
  });
  let hintTopPos = $derived(toolbarVisible && toolbarPos.y > viewportH - 96);
  // 주석 층: 이 창 주석이 있고 범위가 정해진 동안(다시 끄는 중에도 같은 화면 자리에 남아 지금 범위로 잘린다)
  let annotVisible = $derived(
    Boolean(annot && annot.session === activeSession && monitor && activeFrame && (captureView?.phase === 'confirmed' || captureView?.phase === 'drag')),
  );
  let annotAnchorCss = $derived(annot && monitor ? anchorToCss(annot.anchor, monitor) : { x: 0, y: 0 });

  let magBoxW = $derived(colorFormat === 'rgb' ? 214 : 186);
  let magPos = $derived(
    cursorCss
      ? computeMagnifierPosition(cursorCss.x, cursorCss.y, viewportW, viewportH, magBoxW, 111)
      : { x: 0, y: 0, flipX: false, flipY: false },
  );
</script>

<svelte:window onkeydown={handleKeydown} onresize={handleResize} />

{#if activeSession !== null && monitor}
  <div
    bind:this={rootEl}
    class="ink-overlay {className}"
    data-kind={sessionKind}
    data-session={activeSession}
    data-phase={captureView?.phase ?? 'hover'}
    data-native-frame={nativeFrame ? 'true' : undefined}
    data-annot-tool={annotTool ?? undefined}
    data-eyedropper={eyedropperActive ? 'true' : undefined}
    role="application"
    aria-label={t('capture.overlay.label')}
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointerleave={handlePointerLeave}
    ondblclick={handleDblClick}
    oncontextmenu={handleContextMenu}
  >
    {#if !nativeFrame && frameUrl}
      <img
        bind:this={imgEl}
        class="ink-overlay-img"
        src={frameUrl}
        crossorigin="anonymous"
        alt=""
        decoding="sync"
        draggable="false"
        onload={handleMainImgLoad}
        onerror={handleMainImgError}
      />
    {/if}

    {#if nativeFrame && deferredLoad && frameUrl}
      <img
        bind:this={loaderImgEl}
        class="ink-overlay-loader"
        src={frameUrl}
        crossorigin="anonymous"
        alt=""
        decoding="async"
        aria-hidden="true"
        onload={() => handleDeferredImgLoad(activeSession)}
        onerror={() => handleDeferredImgError(activeSession)}
      />
    {/if}

    {#if activeFrame}
      <SelectionFrame
        x={activeFrame.x}
        y={activeFrame.y}
        width={activeFrame.width}
        height={activeFrame.height}
        state={activeFrame.state}
        instant={activeFrame.instant}
        scrim={!nativeFrame}
      />
    {:else if !nativeFrame}
      <div class="ink-overlay-scrim" aria-hidden="true"></div>
    {/if}

    {#if annotVisible && annot && monitor && activeFrame}
      <AnnotateLayer
        engine={annot.engine}
        tool={eyedropperActive ? null : annotTool}
        frame={activeFrame}
        anchorCss={annotAnchorCss}
        scale={monitor.scale > 0 ? monitor.scale : 1}
        {platform}
        mosaicBlock={annotBlock}
        mosaicImage={mosaicSource}
        onRightClick={() => annotTool && selectTool(annotTool)}
      />
    {/if}

    {#if activeGuides.length > 0}
      <svg class="ink-overlay-guides" aria-hidden="true">
        {#each activeGuides as g, i (i)}
          <line
            x1={g.x1}
            y1={g.y1}
            x2={g.x2}
            y2={g.y2}
            stroke="var(--ink-guide)"
            stroke-width="1"
            shape-rendering="crispEdges"
          />
        {/each}
      </svg>
    {/if}

    {#if evidence.length > 0}
      <svg class="ink-overlay-evidence" aria-hidden="true" data-evidence={evidence.length}>
        {#each evidence as b, i (i)}
          <rect
            x={b.x + 0.5}
            y={b.y + 0.5}
            width={Math.max(0, b.width - 1)}
            height={Math.max(0, b.height - 1)}
            fill="none"
            stroke={b.color}
            stroke-width={b.current ? 2 : 1}
            stroke-dasharray={b.hidden ? '4 3' : undefined}
          />
          <text class="ink-overlay-evidence-tag" x={b.x + 4} y={b.y + 13} fill={b.color}>{b.tag}</text>
        {/each}
      </svg>
    {/if}

    {#if sizeBadgeText}
      <div
        class="ink-size-badge ink-mono"
        data-inside={sizeBadgePos.inside ? 'true' : undefined}
        style:transform={`translate3d(${sizeBadgePos.x}px, ${sizeBadgePos.y}px, 0)`}
        bind:offsetWidth={badgeW}
        bind:offsetHeight={badgeH}
        aria-hidden="true"
      >
        {sizeBadgeText}
      </div>
    {/if}

    {#if toolbarVisible}
      <CaptureToolbar
        actions={TOOLBAR_ACTIONS}
        tools={TOOLBAR_ANNOTATE_TOOLS}
        history={TOOLBAR_HISTORY}
        activeTool={annotTool}
        {panelOpen}
        activeColor={activeToolColor}
        activeSize={activeToolSize}
        isMarker={annotTool === 'marker'}
        canUndo={annotCanUndo}
        canRedo={annotCanRedo}
        tooltips={toolbarTips}
        x={toolbarPos.x}
        y={toolbarPos.y}
        bind:width={toolbarW}
        bind:height={toolbarH}
        onHover={(v) => (overToolbar = v)}
        onAnchorChange={(cx) => (toolbarAnchorX = cx)}
        onStyleToggle={toggleStylePanel}
        onTool={selectTool}
        onCommand={handleToolbarCommand}
      />
      {#if panelOpen && annotTool}
        <ColorWidthPanel
          toolId={annotTool}
          color={activeToolColor}
          size={activeToolSize}
          recentColors={annotStyle.recentColors}
          {eyedropperActive}
          showEyedropper={hasOffscreenPixels}
          displayScale={monitor.scale > 0 ? monitor.scale : 1}
          zoomScale={1}
          x={optsPos.x}
          y={optsPos.y}
          side={optsPos.side}
          arrowX={optsPos.arrowX}
          bind:boxWidth={optsW}
          bind:boxHeight={optsH}
          onColor={setAnnotColor}
          onSize={setAnnotSize}
          onEyedropper={toggleEyedropper}
          onHover={(v) => (overOpts = v)}
        />
      {/if}
    {/if}

    {#if stageHintText}
      <div class="ink-stage-hint" data-top={hintTopPos ? 'true' : undefined} aria-live="polite">
        {stageHintText}
      </div>
    {/if}

    {#if magnifierReady && cursorCss && cursorPhys && (!annotTool || eyedropperActive) && !(toolbarVisible && (overToolbar || overOpts))}
      <div
        class="ink-overlay-mag"
        style:transform={`translate3d(${magPos.x}px, ${magPos.y}px, 0)`}
      >
        <Magnifier
          {rgb}
          coord={cursorPhys}
          format={colorFormat}
          flipX={magPos.flipX}
          flipY={magPos.flipY}
          {patch}
        />
      </div>
    {/if}
  </div>
{/if}

{#if afterimage}
  <!-- 확정 직후 고른 자리 주황 잔상 0.4초(R-CAP-26). 창은 클릭 통과(Rust), 여기도 포인터를 받지 않는다 -->
  <div class="ink-afterimage-layer" aria-hidden="true">
    <SelectionFrame
      x={afterimage.x}
      y={afterimage.y}
      width={afterimage.width}
      height={afterimage.height}
      state="afterimage"
      scrim={false}
      instant
      label={t('capture.afterimage.label')}
    />
  </div>
{/if}

<style>
  .ink-overlay {
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: transparent;
    cursor: crosshair;
    user-select: none;
    -webkit-user-select: none;
    z-index: 40;
  }

  /* 주석 도구를 고른 동안 범위 밖은 고르기 십자가 아니다 */
  .ink-overlay[data-annot-tool] {
    cursor: default;
  }

  .ink-overlay[data-eyedropper='true'] {
    cursor: crosshair;
  }

  .ink-size-badge {
    position: absolute;
    left: 0;
    top: 0;
    padding: 3px 8px;
    border-radius: 7px;
    background: rgba(16, 17, 20, 0.84);
    color: #f7f8fa;
    font-size: 11px;
    font-weight: 500;
    line-height: 1.3;
    border: 1px solid rgba(255, 255, 255, 0.14);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.32);
    backdrop-filter: blur(14px);
    -webkit-backdrop-filter: blur(14px);
    pointer-events: none;
    white-space: nowrap;
    z-index: 5;
  }

  :global([data-theme='light']) .ink-size-badge {
    background: rgba(250, 250, 248, 0.9);
    color: #141414;
    border-color: rgba(0, 0, 0, 0.12);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.14);
  }

  .ink-stage-hint {
    position: fixed;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    padding: 5px 12px;
    border-radius: 999px;
    background: rgba(16, 17, 20, 0.78);
    color: #e6e8ee;
    font-size: 12px;
    line-height: 1.3;
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.3);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    pointer-events: none;
    white-space: nowrap;
    z-index: 4;
  }

  .ink-stage-hint[data-top='true'] {
    bottom: auto;
    top: 16px;
  }

  :global([data-theme='light']) .ink-stage-hint {
    background: rgba(250, 250, 248, 0.88);
    color: #1c1d21;
    border-color: rgba(0, 0, 0, 0.12);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.12);
  }

  .ink-afterimage-layer {
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    pointer-events: none;
    background: transparent;
    z-index: 41;
  }

  .ink-overlay-img {
    position: absolute;
    inset: 0;
    width: 100vw;
    height: 100vh;
    display: block;
    object-fit: fill;
    pointer-events: none;
  }

  .ink-overlay-loader {
    position: absolute;
    left: 0;
    top: 0;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  .ink-overlay-scrim {
    position: absolute;
    inset: 0;
    background: var(--ink-scrim);
    pointer-events: none;
  }

  .ink-overlay-guides {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
  }

  .ink-overlay-evidence {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
  }

  .ink-overlay-evidence-tag {
    font-family: var(--ink-font-mono);
    font-size: 11px;
    paint-order: stroke;
    stroke: rgba(0, 0, 0, 0.75);
    stroke-width: 3px;
  }

  .ink-overlay-mag {
    position: absolute;
    left: 0;
    top: 0;
    pointer-events: none;
    will-change: transform;
  }
</style>
