<script>
  import { syncLang, t } from '../i18n/index.svelte.js';
  import { DRAG_START_PX, anchorOf, nextOpacity } from '../models/pin.js';

  /**
   * 캡처 화면 고정 창(R-CAP-29, 04 S08): 창 = 그림. 끌기 = 옮기기 · 휠 = 확대·축소(커서 기준) ·
   * Ctrl/⌘+휠 = 투명도 · 두 번 클릭·Esc = 닫기 · Ctrl/⌘+C 복사 · Ctrl/⌘+S 저장 · 오른쪽 클릭 = 메뉴.
   * 끌기는 누른 채 3px 넘게 움직일 때만 OS에 넘긴다 — 누르자마자 넘기면 두 번째 클릭을 OS가 삼킨다(실측 2026-11-26).
   * @type {{ client: import('@screen-ink/client').InkClient, pinId: number }}
   */
  let { client, pinId } = $props();

  /** @type {{ w: number, h: number, url: string, zoom: number } | null} */
  let info = $state(null);
  let opacity = $state(100);
  let badge = $state('');
  let hover = $state(false);
  /** @type {ReturnType<typeof setTimeout> | null} */
  let badgeTimer = null;
  /** @type {{ x: number, y: number } | null} */
  let pressAt = null;
  let dragging = false;

  $effect(() => {
    let alive = true;
    client.getState?.().then((s) => {
      if (alive && s) syncLang(s);
    });
    const off = client.subscribe?.((e) => {
      if (e.type === 'state') syncLang(e.state);
    });
    Promise.resolve(client.pinInfo(pinId)).then((i) => {
      if (alive) info = i;
    });
    return () => {
      alive = false;
      off?.();
      if (badgeTimer) clearTimeout(badgeTimer);
    };
  });

  /** @param {string} text */
  function showBadge(text) {
    badge = text;
    if (badgeTimer) clearTimeout(badgeTimer);
    badgeTimer = setTimeout(() => (badge = ''), 800);
  }

  /** @param {WheelEvent} e */
  function onWheel(e) {
    e.preventDefault();
    const dy = Math.sign(e.deltaY);
    if (!dy) return;
    if (e.ctrlKey || e.metaKey) {
      opacity = nextOpacity(opacity, dy);
      showBadge(t('pin.view.opacity', { pct: opacity }));
      return;
    }
    const { fx, fy } = anchorOf(e.clientX, e.clientY, window.innerWidth, window.innerHeight);
    Promise.resolve(client.pinZoom(pinId, dy < 0 ? -1 : 1, fx, fy)).then((z) => {
      if (typeof z === 'number') showBadge(`${z}%`);
    });
  }

  /** @param {PointerEvent} e */
  function onPointerDown(e) {
    if (e.button !== 0) return;
    pressAt = { x: e.clientX, y: e.clientY };
    dragging = false;
  }

  /** @param {PointerEvent} e */
  function onPointerMove(e) {
    if (!pressAt || dragging || (e.buttons & 1) === 0) return;
    if (Math.abs(e.clientX - pressAt.x) > DRAG_START_PX || Math.abs(e.clientY - pressAt.y) > DRAG_START_PX) {
      dragging = true;
      client.pinAction(pinId, 'drag');
    }
  }

  function onPointerUp() {
    pressAt = null;
    dragging = false;
  }

  /** @param {KeyboardEvent} e */
  function onKey(e) {
    const mod = e.ctrlKey || e.metaKey;
    if (e.key === 'Escape') client.pinAction(pinId, 'close');
    else if (mod && e.code === 'KeyC') client.pinAction(pinId, 'copy');
    else if (mod && e.code === 'KeyS') {
      e.preventDefault();
      client.pinAction(pinId, 'save');
    }
  }

  $effect(() => {
    const el = document.getElementById('ink-pin');
    if (!el) return;
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  });
</script>

<svelte:window onkeydown={onKey} />

<div
  id="ink-pin"
  class="ink-pin"
  class:hover
  role="img"
  aria-label={t('pin.view.aria')}
  data-pin={pinId}
  style:opacity={opacity / 100}
  onpointerenter={() => (hover = true)}
  onpointerleave={() => (hover = false)}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  ondblclick={() => client.pinAction(pinId, 'close')}
  oncontextmenu={(e) => {
    e.preventDefault();
    client.pinAction(pinId, 'menu');
  }}
>
  {#if info}
    <img src={info.url} alt="" draggable="false" />
  {/if}
  {#if hover}
    <button
      type="button"
      class="ink-pin-close ink-focus"
      aria-label={t('pin.view.closeAria')}
      onpointerdown={(e) => e.stopPropagation()}
      onclick={(e) => {
        e.stopPropagation();
        client.pinAction(pinId, 'close');
      }}
    >
      ×
    </button>
    {#if !badge}
      <span class="ink-pin-hint">{t('pin.view.hint')}</span>
    {/if}
  {/if}
  {#if badge}
    <span class="ink-pin-badge ink-mono">{badge}</span>
  {/if}
</div>

<style>
  :global(html[data-view='pin']),
  :global(html[data-view='pin'] body) {
    margin: 0;
    background: transparent;
    overflow: hidden;
  }

  .ink-pin {
    position: fixed;
    inset: 0;
    cursor: move;
    user-select: none;
    -webkit-user-select: none;
    /* 1px 주황 테두리 35% — 올리면 100%(04 S08) */
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink-accent) 35%, transparent);
  }

  .ink-pin.hover {
    box-shadow: inset 0 0 0 1px var(--ink-accent);
  }

  .ink-pin img {
    display: block;
    width: 100vw;
    height: 100vh;
    object-fit: fill;
    pointer-events: none;
  }

  .ink-pin-close {
    position: absolute;
    top: 6px;
    right: 6px;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--ink-island);
    color: var(--ink-island-text);
    font-size: 14px;
    line-height: 1;
    display: grid;
    place-items: center;
    cursor: pointer;
    opacity: 0.9;
  }

  .ink-pin-close:hover {
    opacity: 1;
    background: var(--ink-danger);
  }

  .ink-pin-hint {
    position: absolute;
    left: 50%;
    bottom: 8px;
    transform: translateX(-50%);
    max-width: calc(100% - 16px);
    padding: 3px 9px;
    border-radius: var(--ink-r-island);
    background: var(--ink-island);
    color: var(--ink-island-text-2);
    font-size: 10.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
  }

  .ink-pin-badge {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    padding: 4px 10px;
    border-radius: var(--ink-r-island);
    background: var(--ink-island);
    color: var(--ink-island-text);
    font-size: 12px;
    pointer-events: none;
    white-space: nowrap;
  }
</style>
