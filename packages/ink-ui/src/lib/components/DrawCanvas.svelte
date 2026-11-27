<script>
  import { onMount, untrack } from 'svelte';
  import { docToView } from '../ink/coords.js';
  import { TEXT_LINE_HEIGHT } from '../ink/inkEngine.js';
  import {
    bakeItem,
    createCanvasTextMeasurer,
    drawBackground,
    renderActive,
    renderItems,
    resolveInkColor,
  } from '../ink/canvasRender.js';
  import {
    DRAW_TOOLBAR_TOGGLE_KEY,
    isDrawToolbarToggleKey,
    matchDrawSizeBracketKey,
  } from '../ink/drawKeys.js';
  import { createPenLatencyMeter } from '../ink/penMetrics.js';
  import { setLang, setOs, t } from '../i18n/index.svelte.js';
  import { comboMap } from '../models/captureToolbar.js';
  import { computeCursorRing } from '../models/drawUi.js';
  import {
    DRAW_TOOLBAR_IDLE_MS,
    DRAW_TOOLBAR_PROXIMITY_PX,
    activeDrawToolId,
    drawToolSizeToEngineWidth,
    engineWidthToDrawToolSize,
    isNearRect,
    panelToolIdForDraw,
    placeDrawColorPanel,
    resolveDrawToolbarVisibility,
  } from '../models/drawToolbar.js';
  import { normalizeColorValue, pushRecentColor, rgbToHex } from '../models/palette.js';
  import {
    clampToolSize,
    createDefaultAnnotateStyle,
    getToolSizeSpec,
    stepSize,
  } from '../models/inkSize.js';
  import ColorWidthPanel from './ColorWidthPanel.svelte';
  import DrawToolbar from './DrawToolbar.svelte';

  /**
   * 그리기·칠판 캔버스(A §3.1·§3.5, 4.2 · R2-DRAW-TB · R2-ANN-5):
   * - 3층 canvas(0 배경 · 1 굳힌 그리기 · 2 지금 획) + DOM 층(글자 입력 상자 · 펜 고리)
   * - 아래 가운데 유리 알약 도구 막대(DrawToolbar) + 색·굵기 패널(ColorWidthPanel) + 화면 색 따기(스포이트)
   * @type {{
   *   engine: import('../ink/inkEngine.js').InkEngine,
   *   backgroundImage?: CanvasImageSource | null,
   *   desynchronized?: boolean,
   *   keyboard?: boolean,
   *   cursorRing?: boolean,
   *   toolbar?: boolean,
   *   initialCursor?: { x: number, y: number } | null,
   *   styleState?: import('../models/inkSize.js').AnnotateStyleState | null,
   *   onStyleChange?: (next: import('../models/inkSize.js').AnnotateStyleState, meta: { kind: 'color' | 'size', toolId: string, color?: string, size?: number }) => void,
   *   onEffect?: (effect: import('../ink/inkEngine.js').InkEffect) => void,
   *   onPenMetrics?: (summary: import('../ink/penMetrics.js').PenLatencySummary) => void,
   *   onFirstPaint?: (at: number, renderMs: number) => void,
   *   class?: string,
   * }}
   */
  let {
    engine,
    backgroundImage = null,
    desynchronized = true,
    keyboard = true,
    cursorRing = true,
    toolbar = true,
    initialCursor = null,
    styleState = null,
    onStyleChange,
    onEffect,
    onPenMetrics,
    onFirstPaint,
    class: className = '',
  } = $props();

  /** @type {HTMLDivElement | null} */
  let rootEl = $state(null);
  /** @type {HTMLCanvasElement | null} */
  let bgEl = $state(null);
  /** @type {HTMLCanvasElement | null} */
  let bakedEl = $state(null);
  /** @type {HTMLCanvasElement | null} */
  let activeEl = $state(null);
  /** @type {HTMLDivElement | null} */
  let ringEl = $state(null);
  /** @type {HTMLTextAreaElement | null} */
  let taEl = $state(null);
  /** @type {any} */
  let toolbarRef = $state(null);

  // DOM 층이 쓰는 엔진 상태(바뀔 때만 다시 넣는다)
  /** @type {import('../ink/inkEngine.js').InkEngine['editing']} */
  let editing = $state.raw(null);
  let viewState = $state.raw({ s: 1, tx: 0, ty: 0 });
  /** @type {import('../ink/inkEngine.js').InkTool} */
  let toolState = $state('pen');
  /** @type {string | null} */
  let shapeToolState = $state(null);
  let widthPx = $state(4);
  let colorCode = $state('R');
  /** @type {import('../ink/inkEngine.js').InkBackground} */
  let bgState = $state('frame');
  let eraserCss = $state(16);
  let canUndo = $state(false);
  let canRedo = $state(false);
  let hasItems = $state(false);
  let platformState = $state(/** @type {'windows' | 'macos'} */ ('windows'));
  /** @type {Record<string, string>} */
  let shortcutsMap = $state.raw({});
  let ringVisible = $state(false);
  let pressing = $state(false);

  // 도구 막대 · 색·굵기 패널 상태(R2-DRAW-TB · R2-ANN-5)
  let toolbarHidden = $state(false);
  let toolbarIdleDim = $state(false);
  let overToolbar = $state(false);
  let overPanel = $state(false);
  let nearToolbar = $state(false);
  let panelOpen = $state(false);
  let eyedropperActive = $state(false);
  let canEyedropper = $state(false);
  /** @type {number | null} */
  let styleAnchorX = $state(null);
  let panelW = $state(284);
  let panelH = $state(216);
  let viewportW = $state(1280);
  let viewportH = $state(720);
  let barTick = $state(0);
  let localStyle = $state(createDefaultAnnotateStyle());
  /** @type {import('../models/inkSize.js').AnnotateStyleState | null} */
  let appliedExternalStyle = null;
  /** @type {unknown} */
  let lastShortcutsRef = null;
  /** @type {ReturnType<typeof setTimeout> | null} */
  let toolbarIdleTimer = null;

  // 그리기 상태(반응형 아님 — rAF 안에서만)
  /** @type {CanvasRenderingContext2D | null} */
  let bgCtx = null;
  /** @type {CanvasRenderingContext2D | null} */
  let bakedCtx = null;
  /** @type {CanvasRenderingContext2D | null} */
  let activeCtx = null;
  let dpr = 1;
  let cssW = 0;
  let cssH = 0;
  let canvasW = 0;
  let canvasH = 0;
  let sizeDirty = true;
  /** @type {{ view: number, doc: number, hidden: number, active: number, bg: string, image: CanvasImageSource | null | undefined }} */
  let drawn = { view: -1, doc: -1, hidden: -1, active: -1, bg: '', image: undefined };
  /** @type {{ x: number, y: number, w: number, h: number } | null} */
  let activeDirty = null;
  let raf = 0;
  let firstPainted = false;
  /** @type {DOMRect | { left: number, top: number }} */
  let rootRect = { left: 0, top: 0 };
  /** @type {string | null} */
  let openedKey = null;
  let hasRingPos = false;
  let ringX = 0;
  let ringY = 0;
  const meter = createPenLatencyMeter();

  /** @param {number} w @param {number} h */
  function createCanvas(w, h) {
    const c = document.createElement('canvas');
    c.width = w;
    c.height = h;
    return c;
  }

  function schedule() {
    if (!raf && typeof requestAnimationFrame === 'function') raf = requestAnimationFrame(frame);
  }

  /** @param {number} now */
  function frame(now) {
    raf = 0;
    // tick이 내는 효과(제안 4초 만료 → suggestEnd, 멈춤 판정 등)를 버리지 않는다
    emit(engine.tick(now));
    paint();
    if (meter.pendingCount) meter.painted(performance.now());
    if (engine.needsTick()) schedule();
  }

  function refreshEyedropperCapability() {
    if (!bgCtx || bgState !== 'frame' || !backgroundImage || canvasW <= 0 || canvasH <= 0) {
      canEyedropper = false;
      if (eyedropperActive) eyedropperActive = false;
      return;
    }
    try {
      bgCtx.getImageData(0, 0, 1, 1);
      canEyedropper = true;
    } catch {
      canEyedropper = false;
      if (eyedropperActive) eyedropperActive = false;
    }
  }

  function paint() {
    if (!bgCtx || !bakedCtx || !activeCtx || canvasW === 0) return;
    const paintStart = firstPainted ? 0 : performance.now();
    const e = engine;
    const bg = e.background;
    const image = backgroundImage;
    const viewChanged = sizeDirty || e.revs.view !== drawn.view;
    const bgChanged = bg !== drawn.bg;
    const imageW = /** @type {any} */ (image)?.width;
    const base = {
      view: e.view,
      dpr,
      canvasW,
      canvasH,
      background: bg,
      image,
      imageScale: typeof imageW === 'number' && imageW > 0 ? imageW / e.doc.docSize.w : 1,
      createCanvas,
    };
    if (viewChanged || bgChanged || image !== drawn.image) {
      drawBackground(bgCtx, { background: bg, image, view: e.view, dpr, docSize: e.doc.docSize, canvasW, canvasH });
      refreshEyedropperCapability();
    }
    const docChanged = e.revs.doc !== drawn.doc;
    const hiddenChanged = e.revs.hidden !== drawn.hidden;
    if (viewChanged || bgChanged || docChanged || hiddenChanged) {
      const appendOnly =
        !viewChanged && !bgChanged && !hiddenChanged && e.revs.doc === drawn.doc + 1 && e.lastChange.kind === 'append';
      if (appendOnly && e.lastChange.kind === 'append') bakeItem(bakedCtx, e.lastChange.item, base);
      else renderItems(bakedCtx, e.items, { ...base, hidden: e.hidden });
    }
    if (viewChanged || bgChanged || docChanged || hiddenChanged || e.revs.active !== drawn.active) {
      const pv = e.getPreview();
      activeDirty = renderActive(
        activeCtx,
        { items: pv.items, marquee: pv.marquee, selectionBox: e.selectionBox(), handles: e.selection.length === 1 },
        base,
        activeDirty,
      );
    }
    drawn = { view: e.revs.view, doc: e.revs.doc, hidden: e.revs.hidden, active: e.revs.active, bg, image };
    sizeDirty = false;
    if (!firstPainted && (bg !== 'frame' || image)) {
      firstPainted = true;
      const at = performance.now();
      onFirstPaint?.(at, at - paintStart);
    }
  }

  function syncDom() {
    const e = engine;
    if (e.editing !== editing) editing = e.editing;
    if (e.view !== viewState) viewState = e.view;
    toolState = e.tool;
    shapeToolState = e.shapeTool;
    widthPx = e.widthPx;
    colorCode = e.color;
    bgState = e.background;
    eraserCss = e.eraserRadius * e.view.s * 2;
    canUndo = e.canUndo;
    canRedo = e.canRedo;
    hasItems = e.items.length > 0;
    platformState = e.platform;
    if (e.shortcuts !== lastShortcutsRef) {
      lastShortcutsRef = e.shortcuts;
      shortcutsMap = comboMap(e.shortcuts);
    }
    setOs(e.platform);
    if (e.lang) setLang(e.lang);

    const uiId = activeDrawToolId(e.tool, e.shapeTool);
    const pId = panelToolIdForDraw(uiId);
    const syncedSize = engineWidthToDrawToolSize(pId, e.widthPx);
    if (localStyle.tools[pId]?.size !== syncedSize) {
      untrack(() => {
        const prevEntry = localStyle.tools[pId] ?? { size: syncedSize };
        localStyle = {
          ...localStyle,
          tools: { ...localStyle.tools, [pId]: { ...prevEntry, size: syncedSize } },
        };
      });
    }
  }

  function resize() {
    if (!rootEl || !bgEl || !bakedEl || !activeEl) return;
    const r = rootEl.getBoundingClientRect();
    rootRect = r;
    dpr = window.devicePixelRatio || 1;
    cssW = r.width;
    cssH = r.height;
    viewportW = cssW;
    viewportH = cssH;
    barTick += 1;
    canvasW = Math.max(1, Math.round(cssW * dpr));
    canvasH = Math.max(1, Math.round(cssH * dpr));
    for (const c of [bgEl, bakedEl, activeEl]) {
      c.width = canvasW;
      c.height = canvasH;
    }
    activeDirty = null;
    sizeDirty = true;
    if (cssW > 0 && cssH > 0) engine.setViewport({ w: cssW, h: cssH });
    schedule();
  }

  function resetToolbarIdle() {
    if (toolbarIdleTimer !== null) clearTimeout(toolbarIdleTimer);
    toolbarIdleDim = false;
    if (!toolbar || toolbarHidden || overToolbar || overPanel || panelOpen || nearToolbar) return;
    toolbarIdleTimer = setTimeout(() => {
      toolbarIdleTimer = null;
      if (!overToolbar && !overPanel && !panelOpen && !nearToolbar) {
        toolbarIdleDim = true;
      }
    }, DRAW_TOOLBAR_IDLE_MS);
  }

  /** @param {number} clientX @param {number} clientY */
  function updateToolbarProximity(clientX, clientY) {
    if (!toolbar || toolbarHidden) {
      nearToolbar = false;
      return;
    }
    const rect = toolbarRef?.getBarRect?.() ?? null;
    const next = isNearRect({ x: clientX, y: clientY }, rect, DRAW_TOOLBAR_PROXIMITY_PX);
    if (next !== nearToolbar) {
      nearToolbar = next;
      resetToolbarIdle();
    }
  }

  /** @param {EventTarget | null} target */
  function isUiTarget(target) {
    return typeof Element === 'function' && target instanceof Element && Boolean(target.closest('.ink-draw-ui'));
  }

  /** @param {ReadonlyArray<import('../ink/inkEngine.js').InkEffect>} effects */
  function emit(effects) {
    for (const fx of effects) onEffect?.(fx);
  }

  /**
   * @param {PointerEvent} ev
   * @param {PointerEvent} [parent]
   * @returns {import('../ink/inkEngine.js').PointerSample}
   */
  function sampleOf(ev, parent = ev) {
    return {
      x: ev.clientX - rootRect.left,
      y: ev.clientY - rootRect.top,
      t: ev.timeStamp,
      button: parent.button,
      pressure: ev.pressure,
      pointerType: parent.pointerType,
      shiftKey: parent.shiftKey,
      ctrlKey: parent.ctrlKey,
      altKey: parent.altKey,
      metaKey: parent.metaKey,
    };
  }

  /** @param {PointerEvent | MouseEvent} ev */
  function moveRing(ev) {
    ringX = ev.clientX - rootRect.left;
    ringY = ev.clientY - rootRect.top;
    hasRingPos = true;
    if (!ringEl) return;
    ringEl.style.transform = `translate(${ringX}px, ${ringY}px)`;
  }

  /** @param {number} clientX @param {number} clientY */
  function pickEyedropperAt(clientX, clientY) {
    if (!bgCtx || !canEyedropper || canvasW <= 0 || canvasH <= 0) return false;
    const x = clientX - rootRect.left;
    const y = clientY - rootRect.top;
    const px = Math.max(0, Math.min(canvasW - 1, Math.round(x * dpr)));
    const py = Math.max(0, Math.min(canvasH - 1, Math.round(y * dpr)));
    try {
      const data = bgCtx.getImageData(px, py, 1, 1).data;
      const hex = rgbToHex([data[0], data[1], data[2]]);
      handlePanelColor(hex, { recordRecent: true });
      eyedropperActive = false;
      return true;
    } catch {
      canEyedropper = false;
      eyedropperActive = false;
      return false;
    }
  }

  /** @param {PointerEvent} ev */
  function handlePointerDown(ev) {
    if (ev.target === taEl || isUiTarget(ev.target)) return;
    if (rootEl) rootRect = rootEl.getBoundingClientRect();
    if (eyedropperActive) {
      ev.preventDefault();
      if (ev.button === 2) {
        eyedropperActive = false;
      } else if (ev.button === 0) {
        pickEyedropperAt(ev.clientX, ev.clientY);
      }
      resetToolbarIdle();
      return;
    }
    if (ev.button === 0 || ev.button === 2) {
      try {
        rootEl?.setPointerCapture(ev.pointerId);
      } catch {
        // 포인터가 이미 사라짐
      }
    }
    ev.preventDefault();
    pressing = ev.button === 0;
    emit(engine.pointerDown(sampleOf(ev)));
    if (engine.gesture?.kind === 'draw') meter.mark(ev.timeStamp);
    ringVisible = true;
    moveRing(ev);
    schedule();
  }

  /** @param {PointerEvent} ev */
  function handlePointerMove(ev) {
    updateToolbarProximity(ev.clientX, ev.clientY);
    if (isUiTarget(ev.target) && !engine.gesture) {
      ringVisible = false;
      return;
    }
    if (ev.target === taEl && !engine.gesture) return;
    const list = typeof ev.getCoalescedEvents === 'function' ? ev.getCoalescedEvents() : [];
    const samples = list.length ? list : [ev];
    for (const s of samples) emit(engine.pointerMove(sampleOf(s, ev)));
    if (engine.gesture?.kind === 'draw') meter.mark(ev.timeStamp);
    moveRing(ev);
    if (!overToolbar && !overPanel && !eyedropperActive) ringVisible = true;
    if (engine.needsTick()) schedule();
  }

  /** @param {PointerEvent} ev */
  function handlePointerUp(ev) {
    pressing = false;
    resetToolbarIdle();
    if (isUiTarget(ev.target) && !engine.gesture) return;
    if (ev.target === taEl && !engine.gesture) return;
    emit(engine.pointerUp(sampleOf(ev)));
    try {
      rootEl?.releasePointerCapture(ev.pointerId);
    } catch {
      // 캡처가 이미 풀림
    }
    schedule();
  }

  function handlePointerCancel() {
    pressing = false;
    engine.pointerCancel();
    resetToolbarIdle();
    schedule();
  }

  function handlePointerLeave() {
    engine.pointerLeave();
    nearToolbar = false;
    ringVisible = false;
    resetToolbarIdle();
  }

  /** @param {PointerEvent} ev */
  function handlePointerEnter(ev) {
    if (rootEl) rootRect = rootEl.getBoundingClientRect();
    updateToolbarProximity(ev.clientX, ev.clientY);
    moveRing(ev);
    if (!isUiTarget(ev.target) && !eyedropperActive) ringVisible = true;
  }

  /** @param {MouseEvent} ev */
  function handleDblClick(ev) {
    if (ev.target === taEl || isUiTarget(ev.target)) return;
    emit(
      engine.doubleClick({
        x: ev.clientX - rootRect.left,
        y: ev.clientY - rootRect.top,
        t: ev.timeStamp,
        button: 0,
      }),
    );
    schedule();
  }

  /** @param {MouseEvent} ev */
  function handleContextMenu(ev) {
    ev.preventDefault();
  }

  /** @param {WheelEvent} ev */
  function handleWheel(ev) {
    if (isUiTarget(ev.target)) return;
    ev.preventDefault();
    if (rootEl) rootRect = rootEl.getBoundingClientRect();
    const k = ev.deltaMode === 1 ? 16 : ev.deltaMode === 2 ? cssH || 800 : 1;
    emit(
      engine.wheel({
        x: ev.clientX - rootRect.left,
        y: ev.clientY - rootRect.top,
        deltaY: ev.deltaY * k,
        ctrlKey: ev.ctrlKey,
        metaKey: ev.metaKey,
      }),
    );
    schedule();
  }

  /** @param {KeyboardEvent} ev */
  function keyInput(ev) {
    return {
      code: ev.code,
      key: ev.key,
      ctrlKey: ev.ctrlKey,
      altKey: ev.altKey,
      shiftKey: ev.shiftKey,
      metaKey: ev.metaKey,
      isComposing: ev.isComposing,
      keyCode: ev.keyCode,
      repeat: ev.repeat,
      t: ev.timeStamp,
    };
  }

  function toggleToolbarHidden() {
    toolbarHidden = !toolbarHidden;
    if (toolbarHidden) {
      panelOpen = false;
      eyedropperActive = false;
      overToolbar = false;
      overPanel = false;
      emit([{ type: 'notice', text: t('draw.notice.toolbarHidden', { key: DRAW_TOOLBAR_TOGGLE_KEY }) }]);
    } else {
      barTick += 1;
      resetToolbarIdle();
    }
  }

  /** @param {KeyboardEvent} ev */
  function handleKeyDown(ev) {
    if (!keyboard) return;
    if (toolbar && !ev.isComposing) {
      const esc = ev.code === 'Escape' || ev.key === 'Escape';
      if (eyedropperActive && esc && !ev.ctrlKey && !ev.altKey && !ev.metaKey && !ev.shiftKey) {
        ev.preventDefault();
        eyedropperActive = false;
        resetToolbarIdle();
        return;
      }
      if (panelOpen && esc && !ev.ctrlKey && !ev.altKey && !ev.metaKey && !ev.shiftKey) {
        ev.preventDefault();
        panelOpen = false;
        overPanel = false;
        resetToolbarIdle();
        return;
      }
      if (!engine.editing && !engine.gesture && isDrawToolbarToggleKey(ev, engine.keymap, engine.platform)) {
        ev.preventDefault();
        toggleToolbarHidden();
        return;
      }
      if (!engine.editing && !engine.gesture) {
        const bracket = matchDrawSizeBracketKey(ev, engine.keymap, engine.platform);
        if (bracket) {
          ev.preventDefault();
          const pId = panelToolIdForDraw(activeDrawToolId(engine.tool, engine.shapeTool));
          const spec = getToolSizeSpec(pId);
          const cur = localStyle.tools[pId]?.size ?? engineWidthToDrawToolSize(pId, engine.widthPx);
          handlePanelSize(stepSize(cur, bracket.dir, spec, { large: bracket.large }));
          resetToolbarIdle();
          return;
        }
      }
    }
    const r = engine.keyDown(keyInput(ev));
    if (r.handled || ev.code === 'Tab') ev.preventDefault();
    emit(r.effects);
    resetToolbarIdle();
    schedule();
  }

  /** @param {KeyboardEvent} ev */
  function handleKeyUp(ev) {
    if (!keyboard) return;
    const r = engine.keyUp(keyInput(ev));
    if (r.handled) ev.preventDefault();
    schedule();
  }

  function handleBlur() {
    // 창을 떠나면 누르고 있던 Tab·수정 키를 놓은 것으로
    engine.keyUp({ code: 'Tab', shiftKey: false, ctrlKey: false, altKey: false, metaKey: false });
  }

  /** @param {Event} ev */
  function handleTextInput(ev) {
    engine.textInput(/** @type {HTMLTextAreaElement} */ (ev.currentTarget).value);
  }

  // ── 도구 막대 · 색·굵기 패널 동작 ──

  /** @param {any} toolItem */
  function handleToolbarTool(toolItem) {
    eyedropperActive = false;
    if (toolItem.kind === 'shape' && toolItem.shape) {
      if (engine.tool === 'pen' && engine.shapeTool === toolItem.shape) {
        engine.setShapeTool(null);
      } else {
        engine.setShapeTool(toolItem.shape);
      }
    } else if (toolItem.tool) {
      engine.setTool(toolItem.tool);
    }
    const nextUiId = activeDrawToolId(engine.tool, engine.shapeTool);
    const pId = panelToolIdForDraw(nextUiId);
    const savedSize = localStyle.tools[pId]?.size;
    if (typeof savedSize === 'number') {
      engine.setWidth(drawToolSizeToEngineWidth(pId, savedSize));
    }
    engine.notify();
    schedule();
    resetToolbarIdle();
  }

  function handleStyleToggle() {
    panelOpen = !panelOpen;
    if (panelOpen) {
      barTick += 1;
    } else {
      eyedropperActive = false;
      overPanel = false;
    }
    resetToolbarIdle();
  }

  /**
   * @param {string} codeOrHex
   * @param {{ recordRecent?: boolean }} [opts]
   */
  function handlePanelColor(codeOrHex, { recordRecent = false } = {}) {
    const norm = normalizeColorValue(codeOrHex);
    if (!norm) return;
    engine.setColor(norm);
    engine.notify();
    const pId = panelToolIdForDraw(activeDrawToolId(engine.tool, engine.shapeTool));
    const key = pId === 'eraser' ? 'pen' : pId;
    const prev = localStyle.tools[key] ?? { size: getToolSizeSpec(key).defaultVal };
    const recentColors =
      recordRecent || norm.startsWith('#') ? pushRecentColor(localStyle.recentColors, norm) : localStyle.recentColors;
    const nextStyle = {
      ...localStyle,
      lastColor: norm,
      recentColors,
      tools: { ...localStyle.tools, [key]: { ...prev, color: norm } },
    };
    localStyle = nextStyle;
    onStyleChange?.(nextStyle, { kind: 'color', toolId: activeDrawToolId(engine.tool, engine.shapeTool), color: norm });
    schedule();
    resetToolbarIdle();
  }

  /** @param {number} val */
  function handlePanelSize(val) {
    const uiId = activeDrawToolId(engine.tool, engine.shapeTool);
    const pId = panelToolIdForDraw(uiId);
    const spec = getToolSizeSpec(pId);
    const clamped = clampToolSize(val, spec);
    engine.setWidth(drawToolSizeToEngineWidth(pId, clamped));
    engine.notify();
    const prev = localStyle.tools[pId] ?? { size: spec.defaultVal };
    const nextStyle = {
      ...localStyle,
      tools: { ...localStyle.tools, [pId]: { ...prev, size: clamped } },
    };
    localStyle = nextStyle;
    onStyleChange?.(nextStyle, { kind: 'size', toolId: uiId, size: clamped });
    schedule();
    resetToolbarIdle();
  }

  function handleEyedropperToggle() {
    if (!canEyedropper) return;
    eyedropperActive = !eyedropperActive;
    resetToolbarIdle();
  }

  /** @param {'frame' | 'white' | 'black'} nextBg */
  function handleBoardToggle(nextBg) {
    eyedropperActive = false;
    engine.setBackground(nextBg);
    schedule();
    resetToolbarIdle();
  }

  function handleToolbarUndo() {
    emit(engine.runAction('d-undo'));
    engine.notify();
    schedule();
    resetToolbarIdle();
  }

  function handleToolbarRedo() {
    emit(engine.runAction('d-redo'));
    engine.notify();
    schedule();
    resetToolbarIdle();
  }

  function handleToolbarClear() {
    emit(engine.runAction('d-clear'));
    engine.notify();
    schedule();
    resetToolbarIdle();
  }

  function handleToolbarClose() {
    panelOpen = false;
    overPanel = false;
    eyedropperActive = false;
    emit([{ type: 'exit', reason: 'esc' }]);
  }

  /**
   * M5 펜 지연 요약을 내보내고 비운다(그리기를 끝내기 직전 DrawOverlay가 부른다 → drawPenMetrics).
   * @returns {import('../ink/penMetrics.js').PenLatencySummary}
   */
  export function flushPenMetrics() {
    if (meter.pendingCount) meter.painted(performance.now());
    const s = meter.summary();
    meter.reset();
    if (s.n) onPenMetrics?.(s);
    return s;
  }

  /** 지금까지의 펜 지연 요약(비우지 않음). */
  export function getPenMetrics() {
    return meter.summary();
  }

  /** 세 층을 모두 다시 그린다. */
  export function redraw() {
    sizeDirty = true;
    schedule();
  }

  onMount(() => {
    if (!bgEl || !bakedEl || !activeEl || !rootEl) return;
    bgCtx = bgEl.getContext('2d', { alpha: false, willReadFrequently: true });
    bakedCtx = bakedEl.getContext('2d', { alpha: true });
    activeCtx = activeEl.getContext('2d', { alpha: true, desynchronized });
    const measureCtx = createCanvas(1, 1).getContext('2d');
    if (measureCtx) engine.measureText = createCanvasTextMeasurer(measureCtx);
    const unsubscribe = engine.subscribe(() => {
      syncDom();
      schedule();
    });
    syncDom();
    const root = rootEl;
    root.addEventListener('wheel', handleWheel, { passive: false });
    const ro = typeof ResizeObserver === 'function' ? new ResizeObserver(() => resize()) : null;
    ro?.observe(root);
    /** @type {MediaQueryList | null} */
    let dprQuery = null;
    const onDpr = () => {
      resize();
      watchDpr();
    };
    const watchDpr = () => {
      dprQuery?.removeEventListener('change', onDpr);
      dprQuery = typeof matchMedia === 'function' ? matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`) : null;
      dprQuery?.addEventListener('change', onDpr);
    };
    watchDpr();
    resize();
    resetToolbarIdle();
    return () => {
      unsubscribe();
      root.removeEventListener('wheel', handleWheel);
      ro?.disconnect();
      dprQuery?.removeEventListener('change', onDpr);
      if (toolbarIdleTimer !== null) {
        clearTimeout(toolbarIdleTimer);
        toolbarIdleTimer = null;
      }
      if (raf) cancelAnimationFrame(raf);
      raf = 0;
      flushPenMetrics();
    };
  });

  $effect(() => {
    void backgroundImage;
    schedule();
  });

  $effect(() => {
    if (styleState && styleState !== appliedExternalStyle) {
      appliedExternalStyle = styleState;
      localStyle = styleState;
    }
  });

  // 글자 입력 상자가 열리면 값을 넣고 포커스(고치기면 원래 글자, 커서는 끝으로)
  $effect(() => {
    const ed = editing;
    const ta = taEl;
    if (!ed) {
      openedKey = null;
      return;
    }
    const key = `${ed.id ?? 'new'}|${ed.x}|${ed.y}`;
    if (!ta || openedKey === key) return;
    openedKey = key;
    ta.value = ed.text;
    ta.focus();
    ta.setSelectionRange(ta.value.length, ta.value.length);
    // R2-TXT-0: 맥 웹뷰 입력 문맥 바로잡기(캡처 뒤 [판서로] 글자도 같은 버그였다)
    if (typeof window !== 'undefined') window.dispatchEvent(new CustomEvent('ink-text-focus', { detail: 'draw-text' }));
  });

  let textBox = $derived.by(() => {
    if (!editing) return null;
    const p = docToView(editing.x, editing.y, viewState);
    const fs = editing.fontSize * viewState.s;
    const m = engine.measureText(editing.text || ' ', editing.fontSize, editing.fontFamily);
    return {
      left: p.vx,
      top: p.vy,
      fontSize: fs,
      width: m.w * viewState.s + fs,
      height: Math.max(m.h, editing.fontSize * TEXT_LINE_HEIGHT) * viewState.s,
      color: resolveInkColor(editing.color, bgState),
      family: editing.fontFamily,
    };
  });

  // 진입 순간 커서 자리 및 도구 전환 후 고리 재마운트 때 (0,0) 번쩍임 방지
  $effect(() => {
    if (initialCursor && !hasRingPos) {
      ringX = initialCursor.x;
      ringY = initialCursor.y;
      hasRingPos = true;
      ringVisible = true;
    }
    if (ringEl && hasRingPos) {
      ringEl.style.transform = `translate(${ringX}px, ${ringY}px)`;
    }
  });

  let ring = $derived(
    computeCursorRing({
      tool: toolState,
      color: colorCode,
      widthPx,
      background: bgState,
      eraserCss,
    }),
  );

  let activeUiTool = $derived(activeDrawToolId(toolState, shapeToolState));
  let panelToolId = $derived(panelToolIdForDraw(activeUiTool));
  let panelSpec = $derived(getToolSizeSpec(panelToolId));
  let activeToolSize = $derived(engineWidthToDrawToolSize(panelToolId, widthPx));
  let toolbarDrawing = $derived(
    pressing && (toolState === 'pen' || toolState === 'marker' || toolState === 'eraser'),
  );
  let toolbarVis = $derived(
    resolveDrawToolbarVisibility({
      hidden: toolbarHidden,
      drawing: toolbarDrawing,
      idleDim: toolbarIdleDim,
      hovering: overToolbar || overPanel,
      panelOpen,
      nearToolbar,
    }),
  );
  let panelPos = $derived.by(() => {
    void barTick;
    void panelOpen;
    const vw = viewportW || 1280;
    const vh = viewportH || 720;
    const rect = toolbarRef?.getBarRect?.();
    const barRect = rect && rect.width > 0
      ? { left: rect.left - rootRect.left, top: rect.top - rootRect.top, width: rect.width, height: rect.height }
      : { left: Math.max(8, (vw - 440) / 2), top: Math.max(8, vh - 60), width: 440, height: 42 };
    const anchor = typeof styleAnchorX === 'number' ? styleAnchorX - rootRect.left : null;
    return placeDrawColorPanel(
      barRect,
      { w: panelW || 284, h: panelH || 216 },
      { w: vw, h: vh },
      anchor,
    );
  });
</script>

<svelte:window onkeydown={handleKeyDown} onkeyup={handleKeyUp} onblur={handleBlur} />

<div
  bind:this={rootEl}
  class="ink-draw {className}"
  data-tool={toolState}
  data-shape={shapeToolState ?? undefined}
  data-eyedropper={eyedropperActive ? 'true' : undefined}
  data-ring={cursorRing && ring && !overToolbar && !overPanel && !eyedropperActive ? 'true' : undefined}
  data-pressing={pressing ? 'true' : undefined}
  role="application"
  aria-label={t('draw.canvas.label')}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerCancel}
  onpointerleave={handlePointerLeave}
  onpointerenter={handlePointerEnter}
  ondblclick={handleDblClick}
  oncontextmenu={handleContextMenu}
>
  <canvas bind:this={bgEl} class="ink-draw-layer ink-draw-bg"></canvas>
  <canvas bind:this={bakedEl} class="ink-draw-layer ink-draw-baked"></canvas>
  <canvas bind:this={activeEl} class="ink-draw-layer ink-draw-active"></canvas>

  {#if textBox}
    <textarea
      bind:this={taEl}
      class="ink-draw-text"
      aria-label={t('draw.canvas.textLabel')}
      spellcheck="false"
      autocomplete="off"
      rows="1"
      style:left={`${textBox.left}px`}
      style:top={`${textBox.top}px`}
      style:width={`${textBox.width}px`}
      style:height={`${textBox.height}px`}
      style:font-size={`${textBox.fontSize}px`}
      style:font-family={`"${textBox.family}", sans-serif`}
      style:color={textBox.color}
      oninput={handleTextInput}
      oncompositionstart={() => engine.compositionStart()}
      oncompositionend={(ev) => engine.compositionEnd(ev.timeStamp)}
    ></textarea>
  {/if}

  {#if cursorRing && ring && !overToolbar && !overPanel && !eyedropperActive}
    <div
      bind:this={ringEl}
      class="ink-draw-ring"
      data-kind={ring.kind}
      data-eraser={ring.kind === 'eraser' ? 'true' : undefined}
      data-dashed={ring.dashed ? 'true' : undefined}
      data-visible={ringVisible ? 'true' : undefined}
      style:width={`${ring.diameter}px`}
      style:height={`${ring.diameter}px`}
      style:margin-left={`${-ring.diameter / 2}px`}
      style:margin-top={`${-ring.diameter / 2}px`}
      style:border-color={ring.color}
    >
      {#if ring.dot > 0}
        <span
          class="ink-draw-ring-dot"
          style:width={`${ring.dot}px`}
          style:height={`${ring.dot}px`}
          style:background={ring.color}
          style:opacity={ring.dotOpacity}
        ></span>
      {/if}
    </div>
  {/if}

  {#if toolbar}
    {#if toolbarVis.visible}
      <DrawToolbar
        bind:this={toolbarRef}
        activeTool={activeUiTool}
        color={colorCode}
        sizeValue={activeToolSize}
        sizeUnit={panelSpec.unit}
        background={bgState}
        mode={engine.doc.mode}
        {canUndo}
        {canRedo}
        {hasItems}
        {panelOpen}
        dimmed={toolbarVis.dimmed}
        interactive={toolbarVis.interactive}
        platform={platformState}
        combos={shortcutsMap}
        onTool={handleToolbarTool}
        onStyleToggle={handleStyleToggle}
        onUndo={handleToolbarUndo}
        onRedo={handleToolbarRedo}
        onBoardToggle={handleBoardToggle}
        onClear={handleToolbarClear}
        onClose={handleToolbarClose}
        onHover={(v) => {
          overToolbar = v;
          if (v) ringVisible = false;
          resetToolbarIdle();
        }}
        onStyleAnchorChange={(cx) => (styleAnchorX = cx)}
      />

      {#if panelOpen}
        <div
          class="ink-draw-panel-wrap ink-draw-ui"
          role="presentation"
          onpointerdown={(e) => e.stopPropagation()}
          onpointermove={(e) => e.stopPropagation()}
          onpointerup={(e) => e.stopPropagation()}
          ondblclick={(e) => e.stopPropagation()}
          onwheel={(e) => e.stopPropagation()}
        >
          <ColorWidthPanel
            toolId={panelToolId}
            color={colorCode}
            size={activeToolSize}
            recentColors={localStyle.recentColors}
            {eyedropperActive}
            showEyedropper={canEyedropper}
            displayScale={engine.doc.scale || 1}
            zoomScale={viewState.s}
            x={panelPos.x}
            y={panelPos.y}
            side={panelPos.side}
            arrowX={panelPos.arrowX}
            bind:boxWidth={panelW}
            bind:boxHeight={panelH}
            onColor={handlePanelColor}
            onSize={handlePanelSize}
            onEyedropper={handleEyedropperToggle}
            onHover={(v) => {
              overPanel = v;
              if (v) ringVisible = false;
              resetToolbarIdle();
            }}
          />
        </div>
      {/if}
    {:else}
      <button
        type="button"
        class="ink-draw-restore-chip ink-draw-ui"
        aria-label={`${t('draw.toolbar.show')} (${DRAW_TOOLBAR_TOGGLE_KEY})`}
        onpointerdown={(e) => e.stopPropagation()}
        onclick={(e) => {
          e.stopPropagation();
          toggleToolbarHidden();
        }}
      >
        <span>{t('draw.toolbar.show')}</span>
        <span class="ink-draw-restore-key ink-mono">{DRAW_TOOLBAR_TOGGLE_KEY}</span>
      </button>
    {/if}

    {#if eyedropperActive}
      <div class="ink-draw-eye-hint ink-draw-ui" aria-live="polite">
        {t('draw.eyedropper.hint')}
      </div>
    {/if}
  {/if}
</div>

<style>
  .ink-draw {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    touch-action: none;
    user-select: none;
    -webkit-user-select: none;
    cursor: crosshair;
    background: var(--ink-island);
  }

  .ink-draw[data-ring='true'] {
    cursor: none;
  }

  .ink-draw[data-eyedropper='true'] {
    cursor: crosshair;
  }

  .ink-draw[data-tool='text'] {
    cursor: text;
  }

  .ink-draw[data-tool='select'] {
    cursor: default;
  }

  .ink-draw[data-tool='pan'] {
    cursor: grab;
  }

  .ink-draw[data-tool='pan'][data-pressing='true'] {
    cursor: grabbing;
  }

  .ink-draw-layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .ink-draw-text {
    position: absolute;
    margin: 0;
    padding: 0;
    border: none;
    outline: 1px dashed var(--ink-annot-outline);
    outline-offset: 2px;
    background: transparent;
    resize: none;
    overflow: hidden;
    white-space: pre;
    line-height: 1.3;
    caret-color: currentColor;
    pointer-events: auto;
  }

  .ink-draw-ring {
    position: absolute;
    left: 0;
    top: 0;
    box-sizing: border-box;
    border: 1.5px solid;
    border-radius: 50%;
    box-shadow: var(--ink-draw-ring-shadow);
    pointer-events: none;
    display: none;
    will-change: transform;
  }

  .ink-draw-ring[data-visible='true'] {
    display: block;
  }

  .ink-draw-ring[data-dashed='true'] {
    border-style: dashed;
  }

  .ink-draw-ring[data-eraser='true'] {
    border-width: 1.5px;
  }

  .ink-draw-ring-dot {
    position: absolute;
    left: 50%;
    top: 50%;
    border-radius: 50%;
    box-shadow: var(--ink-draw-dot-ring);
    transform: translate(-50%, -50%);
  }

  .ink-draw-panel-wrap {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 24;
  }

  .ink-draw-panel-wrap :global(.ink-cw-panel) {
    z-index: 25;
  }

  .ink-draw-restore-chip {
    position: absolute;
    left: 50%;
    bottom: 14px;
    transform: translateX(-50%);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 11px;
    border-radius: 999px;
    border: 1px solid var(--ink-hud-border);
    background: var(--ink-hud-bg-soft);
    color: var(--ink-island-text);
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    opacity: 0.48;
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    transition: opacity 150ms ease-out;
    z-index: 22;
  }

  .ink-draw-restore-chip:hover,
  .ink-draw-restore-chip:focus-visible {
    opacity: 1;
  }

  .ink-draw-restore-chip:focus-visible {
    outline: 2px solid var(--ink-accent);
    outline-offset: 2px;
  }

  .ink-draw-restore-key {
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--ink-hud-key-bg);
    font-size: 10.5px;
  }

  .ink-draw-eye-hint {
    position: absolute;
    left: 50%;
    bottom: 70px;
    transform: translateX(-50%);
    padding: 5px 12px;
    border-radius: 999px;
    background: var(--ink-hud-bg-eye);
    color: var(--ink-island-text);
    border: 1px solid var(--ink-hud-border);
    font-size: 12px;
    line-height: 1.3;
    pointer-events: none;
    white-space: nowrap;
    z-index: 26;
  }
</style>
