// Ramer-Douglas-Peucker 점 줄이기(A §3.3, 02 §3.2). ε = 0.5 문서 px.
// 길이 6px 미만 획이나 점 3개 이하는 건너뛴다(한글 자음의 짧은 꺾임·마침표 보존).
// 살아남은 점은 원본 객체의 필압 p·시각 t를 그대로 갖는다. DOM 의존 0.

/** @typedef {{ x: number, y: number, p?: number, t?: number }} InkPoint */

export const RDP_EPSILON = 0.5;
/** 이 길이(문서 px) 미만 획은 떨림 보정·점 줄이기를 건너뛴다. */
export const SHORT_STROKE_LEN = 6;

/**
 * 누적 경로 길이.
 * @param {ReadonlyArray<InkPoint>} points
 */
export function pathLength(points) {
  let len = 0;
  for (let i = 1; i < points.length; i++) {
    len += Math.hypot(points[i].x - points[i - 1].x, points[i].y - points[i - 1].y);
  }
  return len;
}

/**
 * 점 p와 선분 ab의 거리(ab가 한 점이면 점 거리).
 * @param {InkPoint} p
 * @param {InkPoint} a
 * @param {InkPoint} b
 */
function segmentDistance(p, a, b) {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len2 = dx * dx + dy * dy;
  if (len2 === 0) return Math.hypot(p.x - a.x, p.y - a.y);
  let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2;
  t = t < 0 ? 0 : t > 1 ? 1 : t;
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
}

/**
 * RDP — 남길 점의 인덱스(오름차순). 재귀 대신 스택으로 돌린다.
 * @param {ReadonlyArray<InkPoint>} points
 * @param {number} [epsilon]
 * @returns {number[]}
 */
export function rdpIndices(points, epsilon = RDP_EPSILON) {
  const n = points.length;
  if (n <= 2) return points.map((_, i) => i);
  const keep = new Uint8Array(n);
  keep[0] = 1;
  keep[n - 1] = 1;
  /** @type {Array<[number, number]>} */
  const stack = [[0, n - 1]];
  while (stack.length) {
    const [lo, hi] = /** @type {[number, number]} */ (stack.pop());
    let maxD = -1;
    let idx = -1;
    for (let i = lo + 1; i < hi; i++) {
      const d = segmentDistance(points[i], points[lo], points[hi]);
      if (d > maxD) {
        maxD = d;
        idx = i;
      }
    }
    if (idx !== -1 && maxD > epsilon) {
      keep[idx] = 1;
      stack.push([lo, idx], [idx, hi]);
    }
  }
  /** @type {number[]} */
  const out = [];
  for (let i = 0; i < n; i++) if (keep[i]) out.push(i);
  return out;
}

/**
 * RDP로 줄인 점 배열(원본 점의 복사본).
 * @template {InkPoint} P
 * @param {ReadonlyArray<P>} points
 * @param {number} [epsilon]
 * @returns {P[]}
 */
export function rdp(points, epsilon = RDP_EPSILON) {
  return rdpIndices(points, epsilon).map((i) => ({ ...points[i] }));
}

/**
 * 획 마무리용 점 줄이기: 짧은 획(길이 < 6px)이나 점 3개 이하는 그대로 둔다.
 * @template {InkPoint} P
 * @param {ReadonlyArray<P>} points
 * @param {number} [epsilon]
 * @returns {P[]}
 */
export function simplifyStroke(points, epsilon = RDP_EPSILON) {
  if (points.length <= 3 || pathLength(points) < SHORT_STROKE_LEN) return points.map((p) => ({ ...p }));
  return rdp(points, epsilon);
}
