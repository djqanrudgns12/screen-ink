// 떨림 보정 — One Euro 필터(Casiez 2012, A §3.2, 02 §3.2). 끔 / 약(기본) / 강.
// 속도에 맞춰 컷오프가 올라가 빠른 획은 거의 지연 없이, 느린 획의 손떨림은 부드럽게.
// 길이 6px 미만 획(한글 점·짧은 획)은 원본 점을 그대로 쓴다. DOM 의존 0.

import { pathLength, SHORT_STROKE_LEN } from './rdp.js';

/** @typedef {'off' | 'light' | 'strong'} SmoothingLevel */
/** @typedef {{ minCutoff: number, beta: number, dCutoff: number }} OneEuroParams */
/** @typedef {{ x: number, y: number, p?: number, t?: number }} InkPoint */

/** @type {Readonly<Record<SmoothingLevel, Readonly<OneEuroParams> | null>>} */
export const SMOOTHING_PRESETS = Object.freeze({
  off: null,
  light: Object.freeze({ minCutoff: 1.0, beta: 0.007, dCutoff: 1.0 }),
  strong: Object.freeze({ minCutoff: 0.6, beta: 0.004, dCutoff: 1.0 }),
});

/**
 * 표본 간격(초). 0 나눔을 막고 긴 멈춤도 0.1초로 묶는다.
 * @param {number} dtMs
 */
export function clampDt(dtMs) {
  const s = dtMs / 1000;
  return s < 1 / 1000 ? 1 / 1000 : s > 0.1 ? 0.1 : s;
}

/**
 * @param {number} cutoff  Hz
 * @param {number} dt  초
 */
function smoothingAlpha(cutoff, dt) {
  const tau = 1 / (2 * Math.PI * cutoff);
  return 1 / (1 + tau / dt);
}

/**
 * 1차원 One Euro 필터.
 * @param {OneEuroParams} params
 */
export function createOneEuroFilter(params) {
  /** @type {number | null} */
  let prevX = null;
  let prevDx = 0;
  /** @type {number | null} */
  let prevT = null;
  return {
    /**
     * @param {number} x
     * @param {number} tMs
     */
    filter(x, tMs) {
      if (prevX === null || prevT === null) {
        prevX = x;
        prevT = tMs;
        prevDx = 0;
        return x;
      }
      const dt = clampDt(tMs - prevT);
      const dx = (x - prevX) / dt;
      const aD = smoothingAlpha(params.dCutoff, dt);
      const edx = aD * dx + (1 - aD) * prevDx;
      const cutoff = params.minCutoff + params.beta * Math.abs(edx);
      const a = smoothingAlpha(cutoff, dt);
      const out = a * x + (1 - a) * prevX;
      prevX = out;
      prevDx = edx;
      prevT = tMs;
      return out;
    },
    reset() {
      prevX = null;
      prevT = null;
      prevDx = 0;
    },
  };
}

/**
 * 점 보정기(x·y 각각 필터). 'off'면 그대로 돌려준다.
 * @param {SmoothingLevel} [level]
 */
export function createPointSmoother(level = 'light') {
  const params = SMOOTHING_PRESETS[level] ?? null;
  const fx = params ? createOneEuroFilter(params) : null;
  const fy = params ? createOneEuroFilter(params) : null;
  return {
    level: params ? level : 'off',
    /**
     * @template {InkPoint} P
     * @param {P} point  t = ms
     * @returns {P}
     */
    push(point) {
      if (!fx || !fy) return { ...point };
      const t = point.t ?? 0;
      return { ...point, x: fx.filter(point.x, t), y: fy.filter(point.y, t) };
    },
    reset() {
      fx?.reset();
      fy?.reset();
    },
  };
}

/**
 * 점 배열 전체를 보정(시험·일괄용). 길이 6px 미만 획은 원본 그대로.
 * @template {InkPoint} P
 * @param {ReadonlyArray<P>} points
 * @param {SmoothingLevel} [level]
 * @returns {P[]}
 */
export function smoothPoints(points, level = 'light') {
  if (pathLength(points) < SHORT_STROKE_LEN) return points.map((p) => ({ ...p }));
  const smoother = createPointSmoother(level);
  return points.map((p) => smoother.push(p));
}
