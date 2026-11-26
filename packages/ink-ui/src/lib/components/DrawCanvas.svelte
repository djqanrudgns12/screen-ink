<script>
  import { onMount } from 'svelte';
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
  import { createPenLatencyMeter } from '../ink/penMetrics.js';

  /**
   * 판서 캔버스(A §3.1·§3.5, 4.2) — 3층 canvas(0 배경 · 1 굳힌 판서 · 2 지금 획) + DOM 층(글자 입력 상자 · 펜 고리).
   * 상태는 모두 InkEngine이 갖는다. 이 부품은 DOM 사건 → 엔진 입력(화면 CSS 좌표 + event.timeStamp),
   * 엔진 상태 → 그리기(rAF 한 번에 tick·배경·증분·dirty rect)만 한다.
   * - 포인터: getCoalescedEvents 표본 전부 · 펜 필압 · 우클릭 메뉴 막기 · 포인터 캡처
   * - 휠: passive:false로 받아 확대 단계 / Ctrl(맥 Cmd)+휠 굵기(브라우저 확대 막음)
   * - 키: keyboard=true면 window keydown/keyup을 엔진에(처리한 키만 preventDefault)
   * - M5: 그리는 중 사건 시각을 모아 그린 직후 performance.now()와 비교 → flushPenMetrics()·화면을 떠날 때 onPenMetrics
   * - 글자 크기 측정: 마운트 때 engine.measureText를 캔버스 측정기로 바꾼다
   * - desynchronized: 맨 위(지금 획) 층 getContext 옵션 — 마운트 때 한 번 읽는다
   * @type {{
   *   engine: import('../ink/inkEngine.js').InkEngine,
   *   backgroundImage?: CanvasImageSource | null,
   *   desynchronized?: boolean,
   *   keyboard?: boolean,
   *   cursorRing?: boolean,
   *   onEffect?: (effect: import('../ink/inkEngine.js').InkEffect) => void,
   *   onPenMetrics?: (summary: import('../ink/penMetrics.js').PenLatencySummary) => void,
   *   onFirstPaint?: (at: number) => void,
   *   class?: string,
   * }}
   */
  let {
    engine,
    backgroundImage = null,
    desynchronized = true,
    keyboard = true,
    cursorRing = true,
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

  // DOM 층이 쓰는 엔진 상태(바뀔 때만 다시 넣는다)
  /** @type {import('../ink/inkEngine.js').InkEngine['editing']} */
  let editing = $state.raw(null);
  let viewState = $state.raw({ s: 1, tx: 0, ty: 0 });
  /** @type {import('../ink/inkEngine.js').InkTool} */
  let toolState = $state('pen');
  let widthPx = $state(4);
  let colorCode = $state('R');
  /** @type {import('../ink/inkEngine.js').InkBackground} */
  let bgState = $state('frame');
  let eraserCss = $state(16);
  let ringVisible = $state(false);
  let pressing = $state(false);

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

  function paint() {
    if (!bgCtx || !bakedCtx || !activeCtx || canvasW === 0) return;
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
      onFirstPaint?.(performance.now());
    }
  }

  function syncDom() {
    const e = engine;
    if (e.editing !== editing) editing = e.editing;
    if (e.view !== viewState) viewState = e.view;
    toolState = e.tool;
    widthPx = e.widthPx;
    colorCode = e.color;
    bgState = e.background;
    eraserCss = e.eraserRadius * e.view.s * 2;
  }

  function resize() {
    if (!rootEl || !bgEl || !bakedEl || !activeEl) return;
    const r = rootEl.getBoundingClientRect();
    rootRect = r;
    dpr = window.devicePixelRatio || 1;
    cssW = r.width;
    cssH = r.height;
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
    if (!ringEl) return;
    ringEl.style.transform = `translate(${ev.clientX - rootRect.left}px, ${ev.clientY - rootRect.top}px)`;
  }

  /** @param {PointerEvent} ev */
  function handlePointerDown(ev) {
    if (ev.target === taEl) return;
    if (rootEl) rootRect = rootEl.getBoundingClientRect();
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
    if (ev.target === taEl && !engine.gesture) return;
    const list = typeof ev.getCoalescedEvents === 'function' ? ev.getCoalescedEvents() : [];
    const samples = list.length ? list : [ev];
    for (const s of samples) emit(engine.pointerMove(sampleOf(s, ev)));
    if (engine.gesture?.kind === 'draw') meter.mark(ev.timeStamp);
    moveRing(ev);
    if (engine.needsTick()) schedule();
  }

  /** @param {PointerEvent} ev */
  function handlePointerUp(ev) {
    pressing = false;
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
    schedule();
  }

  function handlePointerLeave() {
    engine.pointerLeave();
    ringVisible = false;
  }

  function handlePointerEnter() {
    ringVisible = true;
  }

  /** @param {MouseEvent} ev */
  function handleDblClick(ev) {
    if (ev.target === taEl) return;
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

  /** @param {KeyboardEvent} ev */
  function handleKeyDown(ev) {
    if (!keyboard) return;
    const r = engine.keyDown(keyInput(ev));
    if (r.handled || ev.code === 'Tab') ev.preventDefault();
    emit(r.effects);
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

  /**
   * M5 펜 지연 요약을 내보내고 비운다(판서를 끝내기 직전 DrawOverlay가 부른다 → drawPenMetrics).
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
    bgCtx = bgEl.getContext('2d', { alpha: false });
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
    return () => {
      unsubscribe();
      root.removeEventListener('wheel', handleWheel);
      ro?.disconnect();
      dprQuery?.removeEventListener('change', onDpr);
      if (raf) cancelAnimationFrame(raf);
      raf = 0;
      flushPenMetrics();
    };
  });

  $effect(() => {
    void backgroundImage;
    schedule();
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

  let ring = $derived.by(() => {
    if (toolState === 'eraser') return { d: Math.max(10, eraserCss), dot: 0, color: '#FFFFFF' };
    if (toolState === 'pen' || toolState === 'marker') {
      return { d: Math.max(14, widthPx + 10), dot: Math.max(2, widthPx), color: resolveInkColor(colorCode, bgState) };
    }
    return null;
  });
</script>

<svelte:window onkeydown={handleKeyDown} onkeyup={handleKeyUp} onblur={handleBlur} />

<div
  bind:this={rootEl}
  class="ink-draw {className}"
  data-tool={toolState}
  data-ring={cursorRing && ring ? 'true' : undefined}
  data-pressing={pressing ? 'true' : undefined}
  role="application"
  aria-label="판서 화면"
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
      aria-label="판서 글자"
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

  {#if cursorRing && ring}
    <div
      bind:this={ringEl}
      class="ink-draw-ring"
      data-eraser={toolState === 'eraser' ? 'true' : undefined}
      data-visible={ringVisible ? 'true' : undefined}
      style:width={`${ring.d}px`}
      style:height={`${ring.d}px`}
      style:margin-left={`${-ring.d / 2}px`}
      style:margin-top={`${-ring.d / 2}px`}
      style:border-color={ring.color}
    >
      {#if ring.dot}
        <span
          class="ink-draw-ring-dot"
          style:width={`${ring.dot}px`}
          style:height={`${ring.dot}px`}
          style:background={ring.color}
        ></span>
      {/if}
    </div>
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
    background: #000;
  }

  .ink-draw[data-ring='true'] {
    cursor: none;
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
    outline: 1px dashed rgba(255, 159, 10, 0.7);
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
    border: 2px solid;
    border-radius: 50%;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.45),
      inset 0 0 0 1px rgba(0, 0, 0, 0.35);
    pointer-events: none;
    display: none;
    will-change: transform;
  }

  .ink-draw-ring[data-visible='true'] {
    display: block;
  }

  .ink-draw-ring[data-eraser='true'] {
    border-width: 1.5px;
  }

  .ink-draw-ring-dot {
    position: absolute;
    left: 50%;
    top: 50%;
    border-radius: 50%;
    transform: translate(-50%, -50%);
  }
</style>
