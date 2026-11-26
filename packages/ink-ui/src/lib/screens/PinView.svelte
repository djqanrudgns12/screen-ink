<script>
  import { anchorOf, isDoublePress, nextOpacity } from '../models/pin.js';

  /**
   * 캡처 핀 창(R-CAP-29, 04 S08): 창 = 그림. 끌기 = 옮기기 · 휠 = 확대·축소(커서 기준) · Ctrl+휠 = 투명도 ·
   * 두 번 누르기·Esc = 닫기 · Ctrl+C 복사 · Ctrl+S 저장 · 오른쪽 클릭 = 메뉴(앱이 띄움).
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
  /** @type {{ t: number, x: number, y: number } | null} */
  let lastPress = null;

  $effect(() => {
    let alive = true;
    Promise.resolve(client.pinInfo(pinId)).then((i) => {
      if (alive) info = i;
    });
    return () => {
      alive = false;
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
    if (e.ctrlKey) {
      opacity = nextOpacity(opacity, dy);
      showBadge(`투명도 ${opacity}%`);
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
    const now = { t: e.timeStamp, x: e.clientX, y: e.clientY };
    if (isDoublePress(lastPress, now)) {
      lastPress = null;
      client.pinAction(pinId, 'close');
      return;
    }
    lastPress = now;
    client.pinAction(pinId, 'drag');
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
  aria-label="화면에 고정한 캡처"
  data-pin={pinId}
  style:opacity={opacity / 100}
  onpointerenter={() => (hover = true)}
  onpointerleave={() => (hover = false)}
  onpointerdown={onPointerDown}
  oncontextmenu={(e) => {
    e.preventDefault();
    client.pinAction(pinId, 'menu');
  }}
>
  {#if info}
    <img src={info.url} alt="" draggable="false" />
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
