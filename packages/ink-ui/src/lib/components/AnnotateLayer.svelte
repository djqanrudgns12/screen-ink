<script>
  import { onMount } from 'svelte';
  import { t } from '../i18n/index.svelte.js';
  import { docToView } from '../ink/coords.js';
  import { TEXT_LINE_HEIGHT } from '../ink/inkEngine.js';
  import { bakeItem, createCanvasTextMeasurer, renderActive, renderItems, resolveInkColor } from '../ink/canvasRender.js';
  import { annotTarget, applyToolMods, layerView, mosaicItem } from '../models/captureAnnotate.js';
  import { DEFAULT_FONT_FAMILY, DEFAULT_FONT_WEIGHT, buildFontFamilyStack, formatCanvasFont, normalizeFontWeight } from '../models/fontPicker.js';

  /**
   * 캡처 주석 층(4.9, R-CAP-28) — 판서 엔진(InkEngine)·렌더러(canvasRender)를 그대로 쓰는 얇은 부품.
   * DrawCanvas와 달리 바탕이 투명(정지 화면은 네이티브 창·<img>가 이미 그림)하고, 지금 범위(frame)만 덮는다(범위 밖 자동 잘림).
   * - 문서 원점 = 주석을 시작한 범위 왼쪽 위(anchorCss), s = 1. 범위를 옮겨도 주석은 화면 같은 자리
   * - 도형 도구 = 엔진 손그림 + 수정 키 도형(applyToolMods), 모자이크 = 이 층이 끌어서 mosaic 항목을 add 명령으로
   * - 포인터 사건은 오버레이(범위 조정·Rust)로 보내지 않는다. 도구가 없으면 pointer-events: none
   * - 글자: 엔진 editing을 textarea로(Esc = 확정, 한글 조합 보호). 키는 상자 밖으로 새지 않게 막는다
   * @type {{
   *   engine: import('../ink/inkEngine.js').InkEngine,
   *   tool?: string | null,
   *   frame: { x: number, y: number, width: number, height: number },
   *   anchorCss: { x: number, y: number },
   *   scale?: number,
   *   platform?: 'windows' | 'macos',
   *   mosaicBlock?: number,
   *   mosaicImage?: CanvasImageSource | null,
   *   fontFamily?: string,
   *   fontWeight?: number,
   *   onFontFallback?: (family: string) => void,
   *   onRightClick?: () => void,
   *   class?: string,
   * }}
   */
  let {
    engine,
    tool = null,
    frame,
    anchorCss,
    scale = 1,
    platform = 'windows',
    mosaicBlock = 10,
    mosaicImage = null,
    fontFamily = '',
    fontWeight = DEFAULT_FONT_WEIGHT,
    onFontFallback = undefined,
    onRightClick = () => {},
    class: className = '',
  } = $props();

  /** @type {HTMLDivElement | null} */
  let rootEl = $state(null);
  /** @type {HTMLCanvasElement | null} */
  let bakedEl = $state(null);
  /** @type {HTMLCanvasElement | null} */
  let activeEl = $state(null);
  /** @type {HTMLDivElement | null} */
  let ringEl = $state(null);
  /** @type {HTMLTextAreaElement | null} */
  let taEl = $state(null);

  /** @type {import('../ink/inkEngine.js').InkEngine['editing']} */
  let editing = $state.raw(null);
  let colorCode = $state('R');
  let widthPx = $state(4);
  let eraserCss = $state(16);
  let ringVisible = $state(false);

  // 그리기 상태(반응형 아님 — rAF 안에서만)
  /** @type {CanvasRenderingContext2D | null} */
  let bakedCtx = null;
  /** @type {CanvasRenderingContext2D | null} */
  let activeCtx = null;
  let dpr = 1;
  let canvasW = 0;
  let canvasH = 0;
  /** @type {{ doc: number, hidden: number, active: number, layout: string, image: CanvasImageSource | null | undefined, mosaic: number }} */
  let drawn = { doc: -1, hidden: -1, active: -1, layout: '', image: undefined, mosaic: -1 };
  /** @type {{ x: number, y: number, w: number, h: number } | null} */
  let activeDirty = null;
  let raf = 0;
  /** @type {DOMRect | { left: number, top: number }} */
  let rootRect = { left: 0, top: 0 };
  /** @type {{ start: { x: number, y: number }, end: { x: number, y: number } } | null} */
  let mosaicDrag = null;
  let mosaicRev = 0;
  /** @type {string | null} */
  let openedKey = null;

  let view = $derived(layerView(anchorCss, frame));
  let layoutKey = $derived(`${frame.x}|${frame.y}|${frame.width}|${frame.height}|${anchorCss.x}|${anchorCss.y}|${scale}`);
  let activeFont = $derived(fontFamily || DEFAULT_FONT_FAMILY);
  let activeWeight = $derived(normalizeFontWeight(fontWeight));

  /** @param {number} w @param {number} h */
  function createCanvas(w, h) {
    const c = document.createElement('canvas');
    c.width = w;
    c.height = h;
    return c;
  }

  function schedule() {
    if (!raf && typeof requestAnimationFrame === 'function') raf = requestAnimationFrame(paint);
  }

  /** 백버퍼 = 범위 CSS × dpr(= 물리 px). 바뀌면 다시 만든다(캔버스가 비워짐). */
  function ensureSize() {
    if (!bakedEl || !activeEl) return;
    const d = (typeof window !== 'undefined' && window.devicePixelRatio) || scale || 1;
    const w = Math.max(1, Math.round(frame.width * d));
    const h = Math.max(1, Math.round(frame.height * d));
    if (w === canvasW && h === canvasH && d === dpr) return;
    dpr = d;
    canvasW = w;
    canvasH = h;
    for (const c of [bakedEl, activeEl]) {
      c.width = w;
      c.height = h;
    }
    activeDirty = null;
    drawn.layout = '';
  }

  function paint() {
    raf = 0;
    if (!bakedCtx || !activeCtx) return;
    ensureSize();
    const e = engine;
    const v = view;
    const base = {
      view: v,
      dpr,
      canvasW,
      canvasH,
      background: /** @type {'frame'} */ ('frame'),
      image: mosaicImage,
      imageScale: scale,
      createCanvas,
    };
    const layoutChanged = drawn.layout !== layoutKey;
    const imageChanged = drawn.image !== mosaicImage;
    const docChanged = e.revs.doc !== drawn.doc;
    const hiddenChanged = e.revs.hidden !== drawn.hidden;
    if (layoutChanged || imageChanged || docChanged || hiddenChanged) {
      const appendOnly = !layoutChanged && !imageChanged && !hiddenChanged && e.revs.doc === drawn.doc + 1 && e.lastChange.kind === 'append';
      if (appendOnly && e.lastChange.kind === 'append') bakeItem(bakedCtx, e.lastChange.item, base);
      else renderItems(bakedCtx, e.items, { ...base, hidden: e.hidden });
    }
    if (layoutChanged || imageChanged || docChanged || hiddenChanged || e.revs.active !== drawn.active || mosaicRev !== drawn.mosaic) {
      if (layoutChanged) {
        activeCtx.setTransform(1, 0, 0, 1, 0, 0);
        activeCtx.clearRect(0, 0, canvasW, canvasH);
        activeDirty = null;
      }
      const pv = e.getPreview();
      const items = pv.items.slice();
      const m = mosaicDrag ? mosaicItem(mosaicDrag.start, mosaicDrag.end, mosaicBlock, 'preview') : null;
      if (m) items.push(m);
      activeDirty = renderActive(activeCtx, { items, marquee: null, selectionBox: null }, base, activeDirty);
    }
    drawn = { doc: e.revs.doc, hidden: e.revs.hidden, active: e.revs.active, layout: layoutKey, image: mosaicImage, mosaic: mosaicRev };
  }

  function syncDom() {
    const e = engine;
    if (e.editing !== editing) editing = e.editing;
    colorCode = e.color;
    widthPx = e.widthPx;
    eraserCss = e.eraserRadius * 2;
  }

  /**
   * 사건 → 엔진 표본(문서 좌표 = 층 안 CSS − 보기 원점). 도형 도구면 수정 키를 그 도형으로 고정.
   * @param {PointerEvent} ev
   * @param {PointerEvent} [parent]
   */
  function sampleOf(ev, parent = ev) {
    const v = view;
    const { sample, tab } = applyToolMods(
      {
        x: ev.clientX - rootRect.left - v.tx,
        y: ev.clientY - rootRect.top - v.ty,
        t: ev.timeStamp,
        button: parent.button,
        pressure: ev.pressure,
        pointerType: parent.pointerType,
        shiftKey: parent.shiftKey,
        ctrlKey: parent.ctrlKey,
        altKey: parent.altKey,
        metaKey: parent.metaKey,
      },
      tool,
      platform,
    );
    engine.mods.tab = tab;
    return sample;
  }

  /** @param {PointerEvent | MouseEvent} ev */
  function moveRing(ev) {
    if (!ringEl) return;
    ringEl.style.transform = `translate(${ev.clientX - rootRect.left}px, ${ev.clientY - rootRect.top}px)`;
  }

  /** @param {PointerEvent} ev */
  function handlePointerDown(ev) {
    if (!tool || ev.target === taEl) return;
    ev.stopPropagation();
    ev.preventDefault();
    if (rootEl) rootRect = rootEl.getBoundingClientRect();
    if (ev.button === 2) {
      if (engine.editing) {
        engine.commitText();
        engine.notify();
      }
      onRightClick();
      return;
    }
    if (ev.button !== 0) return;
    try {
      rootEl?.setPointerCapture(ev.pointerId);
    } catch {
      // 포인터가 이미 사라짐
    }
    ringVisible = true;
    moveRing(ev);
    const s = sampleOf(ev);
    if (annotTarget(tool).mosaic) {
      if (engine.editing) {
        engine.commitText();
        engine.notify();
      }
      mosaicDrag = { start: { x: s.x, y: s.y }, end: { x: s.x, y: s.y } };
      mosaicRev++;
      schedule();
      return;
    }
    engine.pointerDown(s);
    if (tool === 'text' && engine.editing && !engine.editing.id) {
      engine.editing = { ...engine.editing, fontFamily: activeFont, fontWeight: activeWeight };
      engine.notify();
    }
    schedule();
  }

  /** @param {PointerEvent} ev */
  function handlePointerMove(ev) {
    if (!tool) return;
    ev.stopPropagation();
    moveRing(ev);
    if (mosaicDrag) {
      const s = sampleOf(ev);
      mosaicDrag.end = { x: s.x, y: s.y };
      mosaicRev++;
      schedule();
      return;
    }
    if (ev.target === taEl && !engine.gesture) return;
    const list = typeof ev.getCoalescedEvents === 'function' ? ev.getCoalescedEvents() : [];
    for (const c of list.length ? list : [ev]) engine.pointerMove(sampleOf(c, ev));
  }

  /** @param {PointerEvent} ev */
  function handlePointerUp(ev) {
    if (!tool) return;
    ev.stopPropagation();
    try {
      rootEl?.releasePointerCapture(ev.pointerId);
    } catch {
      // 캡처가 이미 풀림
    }
    if (mosaicDrag) {
      const s = sampleOf(ev);
      const item = mosaicItem(mosaicDrag.start, { x: s.x, y: s.y }, mosaicBlock, 'mosaic');
      mosaicDrag = null;
      mosaicRev++;
      if (item) {
        // 엔진 명령으로 쌓는다 — 실행 취소·다시 실행·지우개가 판서와 똑같이
        engine.push({ type: 'add', page: engine.doc.activePage, item: { ...item, id: engine.nextId() } });
        engine.notify();
      }
      schedule();
      return;
    }
    if (ev.target === taEl && !engine.gesture) return;
    engine.pointerUp(sampleOf(ev));
    schedule();
  }

  function handlePointerCancel() {
    mosaicDrag = null;
    mosaicRev++;
    engine.pointerCancel();
    schedule();
  }

  function handlePointerLeave() {
    engine.pointerLeave();
    ringVisible = false;
  }

  /** @param {MouseEvent} ev */
  function handleDblClick(ev) {
    // 도구를 고른 동안 더블클릭은 복사가 아니다(범위 안 그리기)
    if (tool) ev.stopPropagation();
  }

  /** @param {MouseEvent} ev */
  function handleContextMenu(ev) {
    ev.preventDefault();
    ev.stopPropagation();
  }

  /** @param {KeyboardEvent} ev */
  function handleTextKey(ev) {
    // 글자 상자 안 키는 캡처 키(Enter 복사·Esc 취소)로 새지 않는다
    ev.stopPropagation();
    if ((ev.key === 'Escape' || ev.code === 'Escape') && !ev.isComposing) {
      ev.preventDefault();
      engine.commitText();
      engine.notify();
    }
  }

  /** @param {Event} ev */
  function handleTextInput(ev) {
    engine.textInput(/** @type {HTMLTextAreaElement} */ (ev.currentTarget).value);
  }

  onMount(() => {
    if (!bakedEl || !activeEl) return;
    bakedCtx = bakedEl.getContext('2d', { alpha: true });
    activeCtx = activeEl.getContext('2d', { alpha: true });
    const measureCtx = createCanvas(1, 1).getContext('2d');
    if (measureCtx) engine.measureText = createCanvasTextMeasurer(measureCtx);
    const unsubscribe = engine.subscribe(() => {
      syncDom();
      schedule();
    });
    syncDom();
    schedule();
    return () => {
      unsubscribe();
      if (raf && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(raf);
      raf = 0;
      for (const c of [bakedEl, activeEl]) {
        if (c) {
          c.width = 0;
          c.height = 0;
        }
      }
    };
  });

  $effect(() => {
    void layoutKey;
    void mosaicImage;
    void tool;
    schedule();
  });

  $effect(() => {
    const fam = activeFont;
    const w = activeWeight;
    if (engine.editing) {
      const cur = /** @type {any} */ (engine.editing);
      if (cur.fontFamily !== fam || cur.fontWeight !== w) {
        engine.editing = { ...cur, fontFamily: fam, fontWeight: w };
        engine.revs.active++;
        engine.notify();
      }
    }
    if (typeof document !== 'undefined' && document.fonts && typeof document.fonts.load === 'function') {
      Promise.resolve(document.fonts.load(formatCanvasFont(24, fam, w), '가A1'))
        .then(() => {
          drawn.doc = -1;
          schedule();
        })
        .catch(() => {
          onFontFallback?.(fam);
        });
    }
  });

  // 글자 입력 상자가 열리면 포커스(커서는 끝으로)
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
    // R2-TXT-0: 앱(맥)이 이 창 웹뷰의 입력 문맥을 지금 입력 문맥으로 바로잡는다 — 한글 조합 글이 이 상자로 오게
    if (typeof window !== 'undefined') window.dispatchEvent(new CustomEvent('ink-text-focus', { detail: 'annot-text' }));
  });

  let textBox = $derived.by(() => {
    if (!editing) return null;
    const p = docToView(editing.x, editing.y, view);
    const fs = editing.fontSize;
    const fam = editing.fontFamily || activeFont;
    const weight = normalizeFontWeight(/** @type {any} */ (editing).fontWeight ?? activeWeight);
    const m = engine.measureText(editing.text || ' ', editing.fontSize, fam);
    return {
      left: p.vx,
      top: p.vy,
      fontSize: fs,
      width: m.w + fs,
      height: Math.max(m.h, editing.fontSize * TEXT_LINE_HEIGHT),
      color: resolveInkColor(editing.color, 'frame'),
      family: fam,
      familyStack: buildFontFamilyStack(fam),
      weight,
    };
  });

  let ring = $derived.by(() => {
    if (tool === 'eraser') return { d: Math.max(10, eraserCss), dot: 0, color: '#FFFFFF' };
    if (tool === 'pen' || tool === 'marker') {
      return { d: Math.max(14, widthPx + 10), dot: Math.max(2, widthPx), color: resolveInkColor(colorCode, 'frame') };
    }
    return null;
  });
</script>

<div
  bind:this={rootEl}
  class="ink-annot {className}"
  data-tool={tool ?? undefined}
  data-ring={ring ? 'true' : undefined}
  role="application"
  aria-label={t('capture.layer.label')}
  style:left={`${frame.x}px`}
  style:top={`${frame.y}px`}
  style:width={`${Math.max(0, frame.width)}px`}
  style:height={`${Math.max(0, frame.height)}px`}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerCancel}
  onpointerleave={handlePointerLeave}
  onpointerenter={() => (ringVisible = true)}
  ondblclick={handleDblClick}
  oncontextmenu={handleContextMenu}
>
  <canvas bind:this={bakedEl} class="ink-annot-layer"></canvas>
  <canvas bind:this={activeEl} class="ink-annot-layer"></canvas>

  {#if textBox}
    <textarea
      bind:this={taEl}
      class="ink-annot-text"
      aria-label={t('capture.layer.textLabel')}
      spellcheck="false"
      autocomplete="off"
      rows="1"
      style:left={`${textBox.left}px`}
      style:top={`${textBox.top}px`}
      style:width={`${textBox.width}px`}
      style:height={`${textBox.height}px`}
      style:font-size={`${textBox.fontSize}px`}
      style:font-family={textBox.familyStack}
      style:font-weight={textBox.weight}
      style:color={textBox.color}
      oninput={handleTextInput}
      onkeydown={(ev) => {
        if (import.meta.env?.DEV) console.error(`[ime] keydown code=${ev.code} composing=${ev.isComposing}`);
        handleTextKey(ev);
      }}
      onkeyup={(ev) => ev.stopPropagation()}
      onpointerdown={(ev) => ev.stopPropagation()}
      oncompositionstart={() => {
        if (import.meta.env?.DEV) console.error('[ime] compositionstart');
        engine.compositionStart();
      }}
      oncompositionend={(ev) => engine.compositionEnd(ev.timeStamp)}
    ></textarea>
  {/if}

  {#if ring}
    <div
      bind:this={ringEl}
      class="ink-annot-ring"
      data-visible={ringVisible ? 'true' : undefined}
      style:width={`${ring.d}px`}
      style:height={`${ring.d}px`}
      style:margin-left={`${-ring.d / 2}px`}
      style:margin-top={`${-ring.d / 2}px`}
      style:border-color={ring.color}
    >
      {#if ring.dot}
        <span class="ink-annot-ring-dot" style:width={`${ring.dot}px`} style:height={`${ring.dot}px`} style:background={ring.color}></span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .ink-annot {
    position: absolute;
    overflow: hidden;
    touch-action: none;
    user-select: none;
    -webkit-user-select: none;
    pointer-events: none;
    cursor: crosshair;
    z-index: 3;
  }

  .ink-annot[data-tool] {
    pointer-events: auto;
  }

  .ink-annot[data-ring='true'] {
    cursor: none;
  }

  .ink-annot[data-tool='text'] {
    cursor: text;
  }

  .ink-annot-layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .ink-annot-text {
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

  .ink-annot-ring {
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

  .ink-annot-ring[data-visible='true'] {
    display: block;
  }

  .ink-annot-ring-dot {
    position: absolute;
    left: 50%;
    top: 50%;
    border-radius: 50%;
    transform: translate(-50%, -50%);
  }
</style>
