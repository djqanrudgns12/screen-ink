// 실시간 확대 HUD(04 S13 · R-ZOOM-6) 순수 모델 — 커서 주황 후광(44px) 자리, 위치 지도 보임, 이벤트 → 상태.
// 좌표 계산(원본 사각형·배율·후광 CSS px)은 Rust(ink-domain::zoom)가 하고, 여기서는 그리기 값만 만든다.

/** 후광 지름(CSS px, 04 S13). */
export const HALO_SIZE = 44;
/** 위치 지도 여백(오른쪽 아래, CSS px — F14). */
export const MINIMAP_MARGIN = 20;

/**
 * @typedef {import('@screen-ink/client').LiveZoomState} LiveZoomState
 */

/**
 * 지금 모드가 실시간 확대인지(`InkState.mode`).
 * @param {{mode?: string} | null | undefined} mode
 */
export function isLiveZoomMode(mode) {
  return mode?.mode === 'liveZoom';
}

/**
 * 클라이언트 이벤트 하나 → 다음 HUD 상태. `zoom` 이벤트가 값(끝 = null)을 정하고,
 * 상태 이벤트에서 모드가 확대가 아니면 거둔다(이벤트 순서가 뒤바뀌어도 남지 않게).
 * @param {LiveZoomState | null} current
 * @param {{type: string, zoom?: LiveZoomState | null, state?: {mode?: {mode?: string}}}} event
 * @returns {LiveZoomState | null}
 */
export function applyZoomEvent(current, event) {
  if (!event) return current;
  if (event.type === 'zoom') return event.zoom ?? null;
  if (event.type === 'state' && event.state && !isLiveZoomMode(event.state.mode)) return null;
  return current;
}

/**
 * 후광 원의 왼쪽 위(CSS px) — 중심이 커서 끝. 커서가 확대 화면 밖이면 null.
 * @param {{x: number, y: number} | null | undefined} cursorCss
 * @param {number} [size]
 * @returns {{x: number, y: number, size: number} | null}
 */
export function haloBox(cursorCss, size = HALO_SIZE) {
  if (!cursorCss || !Number.isFinite(cursorCss.x) || !Number.isFinite(cursorCss.y)) return null;
  return { x: cursorCss.x - size / 2, y: cursorCss.y - size / 2, size };
}

/**
 * 위치 지도를 그릴지(설정 `zoom.minimap`).
 * @param {LiveZoomState | null | undefined} zoom
 */
export function showMinimap(zoom) {
  return Boolean(zoom && zoom.showMinimap);
}
