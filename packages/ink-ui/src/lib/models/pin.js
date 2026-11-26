// 캡처 핀(R-CAP-29) 화면 순수 모델 — 투명도 단계, 두 번 누르기 판정, 커서 자리 비율.

export const OPACITY_MIN = 20;
export const OPACITY_MAX = 100;
/** 두 번 누르기로 볼 간격(ms)·거리(CSS px) */
export const DOUBLE_MS = 400;
export const DOUBLE_PX = 4;

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
 * 끌기를 OS에 넘기면 dblclick 사건이 오지 않는다 → 누름 두 번을 직접 본다.
 * @param {{ t: number, x: number, y: number } | null} prev
 * @param {{ t: number, x: number, y: number }} now
 */
export function isDoublePress(prev, now) {
  if (!prev) return false;
  return now.t - prev.t <= DOUBLE_MS && Math.abs(now.x - prev.x) <= DOUBLE_PX && Math.abs(now.y - prev.y) <= DOUBLE_PX;
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
