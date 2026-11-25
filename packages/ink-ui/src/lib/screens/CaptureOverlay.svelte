<script>
  import Magnifier from '../components/Magnifier.svelte';
  import SelectionFrame from '../components/SelectionFrame.svelte';
  import {
    buildMagnifierPatch,
    clientToImagePixel,
    clientToPhysical,
    computeMagnifierPosition,
    computeSampleRect,
    findMonitorForOverlay,
    isPhysicalPointInMonitor,
    physicalToClient,
    resolveMonitorHighlight,
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
  /** @type {import('@screen-ink/client').OverlayMonitor | null} */
  let monitor = $state(null);
  let frameUrl = $state('');
  let deferredLoad = $state(false);

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
  }

  function clearOverlay() {
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
    monitor = null;
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
        activeSession = e.overlay.session;
        sessionKind = e.overlay.kind;
        nativeFrame = isNative;
        sessionHighlight = e.overlay.highlight ?? null;
        monitor = mon;
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
      } else if (e.type === 'state') {
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
  }

  /** @param {PointerEvent} e */
  function handlePointerMove(e) {
    if (activeSession === null || !monitor) return;
    pendingPointer = { clientX: e.clientX, clientY: e.clientY };
    if (!moveRaf && typeof requestAnimationFrame === 'function') {
      moveRaf = requestAnimationFrame(flushPointerMove);
    } else if (typeof requestAnimationFrame !== 'function') {
      flushPointerMove();
    }
  }

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
    client.run('capture.cancel');
  }

  /** @param {KeyboardEvent} e */
  function handleKeydown(e) {
    if (activeSession === null) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      client.run('capture.cancel');
      return;
    }
    if (e.key === 'Shift' && !e.repeat && !e.ctrlKey && !e.altKey && !e.metaKey) {
      colorFormat = toggleColorFormat(colorFormat);
    }
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
      };
    }
    const hl = resolveMonitorHighlight(sessionHighlight, monitor);
    if (!hl) return null;
    return {
      x: hl.x,
      y: hl.y,
      width: hl.width,
      height: hl.height,
      state: /** @type {'picking'} */ ('picking'),
    };
  });

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
    data-native-frame={nativeFrame ? 'true' : undefined}
    role="application"
    aria-label="캡처 화면"
    onpointermove={handlePointerMove}
    onpointerleave={handlePointerLeave}
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
        scrim={!nativeFrame}
      />
    {:else if !nativeFrame}
      <div class="ink-overlay-scrim" aria-hidden="true"></div>
    {/if}

    {#if magnifierReady && cursorCss && cursorPhys}
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

  .ink-overlay-mag {
    position: absolute;
    left: 0;
    top: 0;
    pointer-events: none;
    will-change: transform;
  }
</style>
