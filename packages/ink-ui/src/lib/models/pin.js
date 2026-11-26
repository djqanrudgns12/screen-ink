// 캡처 핀(R-CAP-29) 화면 순수 모델 — 투명도 단계, 끌기 시작 거리, 커서 자리 비율.

export const OPACITY_MIN = 20;
export const OPACITY_MAX = 100;
/** 누른 채 이만큼(CSS px) 넘게 움직이면 끌기 시작 */
export const DRAG_START_PX = 3;

/**
 * Ctrl+휠 한 칸: 위(dy<0) = 진하게 +10, 아래 = 옅게 -10, 20~100.
 * @param {number} opacity
 * @param {number} dy
 */
export function nextOpacity(opacity, dy) {
  const o = Math.min(OPACITY_MAX, Math.max(OPACITY_MIN, Math.round(opacity)));
  const step = dy < 0 ? 10 : dy > 0 ? -10 : 0;
  return Math.min(OPACITY_MAX, Math.max(OPACITY_MIN, o + step));
}

/**
 * 창 안 커서 자리 → 0~1 비율(확대 기준점).
 * @param {number} x
 * @param {number} y
 * @param {number} w
 * @param {number} h
 */
export function anchorOf(x, y, w, h) {
  const fx = w > 0 ? Math.min(1, Math.max(0, x / w)) : 0.5;
  const fy = h > 0 ? Math.min(1, Math.max(0, y / h)) : 0.5;
  return { fx, fy };
}
