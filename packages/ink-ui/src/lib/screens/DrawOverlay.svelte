<script>
  import { untrack } from 'svelte';
  import CheatSheet from './CheatSheet.svelte';

  /**
   * S12 Alt+1 판서 · S14 Alt+3 칠판 — 오버레이 창(모니터 1개) 껍데기(A §2.4·§3.7).
   * 이 껍데기: 배경(정지 화면 또는 흰 칠판) · 진입 계측(drawReady) · Esc·우클릭(1×) 끝내기(drawFinish) ·
   * `?` 단축키 한 장 · Rust가 먼저 끝낸 세션의 늦은 끝(보관) · 판서 섬·내보내기·복원·펜 지연 연결 함수.
   * 그리기(포인터·획·확대·Undo·Esc 단계)는 DrawCanvas(흐름 ink-engine)가 맡는다.
   *
   * DrawCanvas에 넘길 것(메인이 병합 때 아래 자리에 끼움):
   *   session                DrawSession(board · monitor.bounds/scale · initialView · restoreOnOpen · hasKept · keptCompatible · cursor)
   *   background             { kind: 'frame'|'white'|'black', image: HTMLImageElement|null, url, physW, physH }
   *                          (image는 crossorigin="anonymous"라 캔버스에 그려도 내보내기 getImageData가 막히지 않음)
   *   settings · shortcuts · platform   draw.* 값 · 칸 id → 지금 키(d-* 29칸, R-KEY-6) · 'windows'|'macos'
   *   provideDocument(fn)    끝낼 때 보관할 InkDocument를 꺼내는 함수 등록(fn: () => InkDocument | null)
   *   onView(view)           확대·이동이 바뀜 — 1×일 때만 우클릭이 끝내기(스마트 우클릭, 02 §3.4)
   *   onIsland(patch)        판서 섬 조각(색·도구·굵기·배율·쪽·바탕·안내) → drawIsland
   *   onExit()               Esc 단계 끝·1× 우클릭 → 판서 끝(보관)
   *   onRestore()            → Promise<DrawRestoreResult>(Ctrl+Shift+R · restoreOnOpen. 섬 문구는 Rust가 띄움)
   *   onExport(action, bytes, w, h) → Promise<{ok, name?}>(Ctrl+C · Ctrl+S, 합성한 RGBA 원본 바이트 —
   *                          확대 중이어도 늘 모니터 전체 = background.physW × physH. 다르면 Rust가 거절)
   *   onPenMetrics(sample)   세션 끝에 한 번 {n, p50, p95, max}
   *   onHelp()               `?` — 단축키 한 장 열기·닫기
   * DrawCanvas가 Esc·우클릭을 직접 처리했으면 preventDefault() — 이 껍데기는 defaultPrevented면 건너뛴다
   * (창 keydown은 `onkeydowncapture`로 받으면 껍데기보다 먼저 돈다).
   *
   * @type {{
   *   client: import('@screen-ink/client').InkClient,
   *   session: import('@screen-ink/client').DrawSession,
   *   monitorId?: number,
   *   class?: string,
   * }}
   */
  let { client, session, monitorId = 0, class: className = '' } = $props();

  /** @type {Record<string, any>} */
  let settings = $state({});
  /** @type {Record<string, string>} */
  let shortcuts = $state({});
  /** @type {'windows' | 'macos'} */
  let platform = $state('windows');
  /** @type {import('@screen-ink/client').DrawViewTransform} */
  let view = $state(untrack(() => ({ ...(session.initialView ?? { s: 1, tx: 0, ty: 0 }) })));
  let helpOpen = $state(false);
  /** @type {HTMLImageElement | null} */
  let bgImage = $state(null);
  let bgFailed = $state(false);

  let frameUrl = $derived(session.board ? '' : session.monitor?.frameUrl || '');
  /** @type {'frame' | 'white' | 'black'} */
  let bgKind = $derived(session.board || !frameUrl || bgFailed ? 'white' : 'frame');
  let background = $derived({
    kind: bgKind,
    image: bgKind === 'frame' ? bgImage : null,
    url: frameUrl,
    physW: session.monitor?.bounds?.w ?? 0,
    physH: session.monitor?.bounds?.h ?? 0,
  });

  // 세션 하나에 한 번만(이 화면은 세션마다 새로 만든다 — App.svelte {#key})
  let finished = false;
  let readySent = false;
  let readyScheduled = false;
  /** @type {(() => any) | null} */
  let documentProvider = null;
  let shownAt = nowMs();
  let raf1 = 0;
  let raf2 = 0;

  function nowMs() {
    return typeof performance !== 'undefined' && typeof performance.now === 'function' ? performance.now() : Date.now();
  }

  // ───────────── 설정·키(화면 안 키는 설정값에서, R-KEY-6) ─────────────

  /** @param {any} state */
  function applyState(state) {
    if (!state) return;
    if (state.settings && typeof state.settings === 'object') settings = state.settings;
    if (Array.isArray(state.shortcuts)) shortcuts = Object.fromEntries(state.shortcuts.map((/** @type {any} */ r) => [r.id, r.combo]));
    if (typeof state.platform === 'string') platform = state.platform === 'macos' ? 'macos' : 'windows';
  }

  $effect(() => {
    let alive = true;
    Promise.resolve(client.getState())
      .then((s) => {
        if (alive) applyState(s);
      })
      .catch(() => {});
    const off = client.subscribe((e) => {
      if (e.type === 'state') applyState(e.state);
    });
    return () => {
      alive = false;
      off();
    };
  });

  // ───────────── 진입 계측(M4) ─────────────

  /** 배경을 그린 다음 프레임에 drawReady 한 번. @param {number} paintStartAt */
  function scheduleReady(paintStartAt) {
    if (readyScheduled || readySent) return;
    readyScheduled = true;
    const done = () => {
      if (readySent) return;
      readySent = true;
      client.drawReady(session.session, Math.max(0, Math.round((nowMs() - paintStartAt) * 10) / 10));
    };
    if (typeof requestAnimationFrame !== 'function') {
      done();
      return;
    }
    raf1 = requestAnimationFrame(() => {
      raf1 = 0;
      raf2 = requestAnimationFrame(() => {
        raf2 = 0;
        done();
      });
    });
  }

  function handleBgLoad() {
    scheduleReady(nowMs());
  }

  function handleBgError() {
    // 배경을 못 읽어도 판서는 흰 바탕으로 이어 간다
    bgFailed = true;
    scheduleReady(shownAt);
  }

  // 칠판은 배경 이미지가 없다 · 캐시에 있던 이미지라 onload를 놓친 경우
  $effect(() => {
    const url = frameUrl;
    const el = bgImage;
    if (!url) {
      scheduleReady(shownAt);
    } else if (el && el.complete && el.naturalWidth > 0) {
      scheduleReady(nowMs());
    }
  });

  // ───────────── 끝내기 · 늦은 끝 ─────────────

  function currentDocument() {
    try {
      return documentProvider?.() ?? null;
    } catch {
      return null;
    }
  }

  /** 판서 끝(보관은 Rust가 draw.keep·항목 수로 정한다). 세션당 한 번. */
  function finish() {
    if (finished) return;
    finished = true;
    const doc = currentDocument();
    client.drawFinish({ session: session.session, keep: true, kept: doc ? { document: doc } : undefined });
  }

  // Rust가 먼저 끝냈으면(같은 키·다른 모드 키·잠금·화면 구성 변경) 상태가 바뀌어 이 화면이 내려간다 —
  // 그때 문서를 늦은 끝으로 보내 보관한다(문서는 웹뷰에만 있음).
  $effect(() => {
    return () => {
      if (raf1 && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(raf1);
      if (raf2 && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(raf2);
      finish();
    };
  });

  // ───────────── DrawCanvas 연결 함수 ─────────────

  /** @param {() => any} fn */
  function provideDocument(fn) {
    documentProvider = fn;
  }

  /** @param {import('@screen-ink/client').DrawViewTransform} next */
  function onView(next) {
    view = next;
  }

  /** @param {import('@screen-ink/client').DrawIslandPatch} patch */
  function onIsland(patch) {
    client.drawIsland({ ...patch, session: session.session });
  }

  function onExit() {
    finish();
  }

  function onRestore() {
    return client.drawRestore(session.session);
  }

  /**
   * @param {'copy' | 'save'} action
   * @param {ArrayBuffer | ArrayBufferView} bytes
   * @param {number} w
   * @param {number} h
   */
  function onExport(action, bytes, w, h) {
    return client.drawExport(action, bytes, { w, h, session: session.session });
  }

  /** @param {import('@screen-ink/client').PenSample} sample */
  function onPenMetrics(sample) {
    client.drawPenMetrics(sample);
  }

  function onHelp() {
    helpOpen = !helpOpen;
  }

  /** DrawCanvas를 끼울 때 한 번에 넘기는 묶음(위 설명과 같은 이름) — `<DrawCanvas {...canvasProps} />`. */
  let canvasProps = $derived({
    session,
    background,
    settings,
    shortcuts,
    platform,
    provideDocument,
    onView,
    onIsland,
    onExit,
    onRestore,
    onExport,
    onPenMetrics,
    onHelp,
  });

  // ───────────── 키 · 우클릭 ─────────────

  /** @param {KeyboardEvent} e */
  function handleKeydown(e) {
    // 도움말이 열려 있으면 도움말이 스스로 닫는다 · DrawCanvas가 이미 처리했으면 건너뜀 · 한글 조합 중 무시
    if (helpOpen || e.defaultPrevented || e.isComposing || e.keyCode === 229) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      finish();
    } else if (e.key === '?') {
      e.preventDefault();
      onHelp();
    }
  }

  /** @param {PointerEvent} e */
  function handlePointerDown(e) {
    if (e.button !== 2 || e.defaultPrevented) return;
    e.preventDefault();
    // 확대 중 우클릭은 손 모드(DrawCanvas 몫), 1×면 끝내기
    if (view.s <= 1) finish();
  }

  /** @param {MouseEvent} e */
  function handleContextMenu(e) {
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="ink-draw-overlay {className}"
  data-monitor={monitorId}
  data-bg={bgKind}
  data-zoomed={view.s > 1 ? 'true' : undefined}
  role="application"
  aria-label={session.board ? '칠판' : '판서'}
  onpointerdown={handlePointerDown}
  oncontextmenu={handleContextMenu}
>
  {#if frameUrl && !bgFailed}
    <img
      class="ink-draw-bg"
      src={frameUrl}
      alt=""
      crossorigin="anonymous"
      draggable="false"
      bind:this={bgImage}
      onload={handleBgLoad}
      onerror={handleBgError}
    />
  {/if}

  <!-- DrawCanvas 연결: 흐름 ink-engine이 만든 components/DrawCanvas.svelte를 메인이 병합 때 끼움 -->

  {#if helpOpen}
    <CheatSheet {client} mode="draw" bind:open={helpOpen} />
  {/if}
</div>

<style>
  .ink-draw-overlay {
    position: fixed;
    inset: 0;
    overflow: hidden;
    user-select: none;
    touch-action: none;
    cursor: crosshair;
    background: var(--ink-pen-W);
  }

  .ink-draw-overlay[data-bg='frame'] {
    background: transparent;
  }

  .ink-draw-overlay[data-bg='black'] {
    background: var(--ink-pen-K);
  }

  /* 정지 화면은 물리 px 그대로 → 창(같은 모니터 전체)에 꽉 맞춘다 */
  .ink-draw-bg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    pointer-events: none;
  }
</style>
