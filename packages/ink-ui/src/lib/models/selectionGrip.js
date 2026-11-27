// 확정한 캡처 범위를 손볼 때 커서 모양(2026-11-28 사용자 요청: 안 = 옮기기, 변·귀 = 늘리기·줄이기).
// 판정 규칙은 Rust `ink_domain::selection::grip_at`과 같다(변·귀는 안팎 `tol` px, 그 밖의 안 = 옮기기, 밖 = 없음).

/** 변·귀를 잡는 거리(CSS px) — Rust `GRIP_LOGICAL`과 같은 값. */
export const GRIP_TOL = 6;

/**
 * @param {{ x: number, y: number, width: number, height: number }} frame 범위(CSS px, 캡처 화면 기준)
 * @param {number} px
 * @param {number} py
 * @param {number} [tol]
 * @returns {'' | 'move' | 'nwse-resize' | 'nesw-resize' | 'ew-resize' | 'ns-resize'}
 */
export function gripCursor(frame, px, py, tol = GRIP_TOL) {
  const { x, y, width, height } = frame;
  const right = x + width;
  const bottom = y + height;
  if (px < x - tol || px > right + tol || py < y - tol || py > bottom + tol) return '';
  /** @param {number} a @param {number} b */
  const pick = (a, b) => {
    const na = Math.abs(a) <= tol;
    const nb = Math.abs(b) <= tol;
    if (na && nb) return [Math.abs(a) <= Math.abs(b), Math.abs(a) > Math.abs(b)];
    return [na, nb];
  };
  const [left, rightEdge] = pick(px - x, px - right);
  const [top, bottomEdge] = pick(py - y, py - bottom);
  if ((left && top) || (rightEdge && bottomEdge)) return 'nwse-resize';
  if ((rightEdge && top) || (left && bottomEdge)) return 'nesw-resize';
  if (left || rightEdge) return 'ew-resize';
  if (top || bottomEdge) return 'ns-resize';
  return px >= x && px <= right && py >= y && py <= bottom ? 'move' : '';
}
