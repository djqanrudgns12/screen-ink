// 도형 보정(A §3.4, 02 §3.3) — 직선·화살표 수평·수직 ±4° 맞춤, 0.5초 멈춤 도형 판정(직선 → 타원 → 사각형).
// 판정 실패면 null(아무 변화 없음). DOM 의존 0.

import { pathLength, rdp } from './rdp.js';

/** @typedef {{ x: number, y: number, p?: number, t?: number }} InkPoint */
/** @typedef {'line' | 'rect' | 'ellipse' | 'arrow'} ShapeKind */
/**
 * @typedef {Object} FittedShape
 * @property {ShapeKind} shape
 * @property {number} x1
 * @property {number} y1
 * @property {number} x2
 * @property {number} y2
 * @property {boolean} [snapped]  수평·수직으로 맞췄는지
 * @property {boolean} [circle]   타원을 정원으로 다듬었는지
 */

export const SNAP_TOLERANCE_DEG = 4;
/** 멈춤 판정: 이 시간(ms) 동안 화면 2px 안에 머물고, 경로가 24 문서 px 이상이면 도형 후보를 본다. */
export const HOLD_MS = 500;
export const HOLD_MOVE_PX = 2;
export const HOLD_MIN_PATH = 24;
/** 직선: 최대 벗어남 < 길이 × 6%, 길이 ≥ 20. */
export const LINE_MAX_DEVIATION = 0.06;
export const LINE_MIN_LEN = 20;
/** 닫힘: 시작·끝 거리 < 큰 변 × 25%(또는 경로가 타원 둘레의 85% 이상). */
export const CLOSE_RATIO = 0.25;
export const ARC_COVER_RATIO = 0.85;
/** 타원: 정규화 반지름 평균 오차 < 8%, 두 반축 차 < 12%면 정원. */
export const ELLIPSE_MAX_ERROR = 0.08;
export const CIRCLE_RATIO = 0.12;
/** 사각형: RDP ε = 대각선 × 4%, 꼭짓점 4개, 내각 90° ± 12°, 기울기 ±8° 안이면 축 정렬. */
export const RECT_RDP_RATIO = 0.04;
export const RECT_ANGLE_TOL = 12;
export const RECT_TILT_TOL = 8;
/** 도형 판정에 쓰는 최소 크기(문서 px). */
const MIN_SHAPE_SIZE = 8;

const DEG = 180 / Math.PI;

/**
 * 직선·화살표 끝점을 수평·수직 ±tol° 안이면 맞춘다(y2 = y1 또는 x2 = x1).
 * @param {number} x1
 * @param {number} y1
 * @param {number} x2
 * @param {number} y2
 * @param {number} [tolDeg]
 * @returns {{ x2: number, y2: number, snapped: boolean }}
 */
export function snapAngle(x1, y1, x2, y2, tolDeg = SNAP_TOLERANCE_DEG) {
  const dx = x2 - x1;
  const dy = y2 - y1;
  if (dx === 0 && dy === 0) return { x2, y2, snapped: false };
  const a = Math.atan2(dy, dx) * DEG; // -180 ~ 180
  const nearH = Math.abs(a) <= tolDeg || Math.abs(Math.abs(a) - 180) <= tolDeg;
  const nearV = Math.abs(Math.abs(a) - 90) <= tolDeg;
  if (nearH) return { x2, y2: y1, snapped: dy !== 0 };
  if (nearV) return { x2: x1, y2, snapped: dx !== 0 };
  return { x2, y2, snapped: false };
}

/**
 * @param {ReadonlyArray<InkPoint>} points
 */
export function pointsBounds(points) {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const p of points) {
    if (p.x < minX) minX = p.x;
    if (p.y < minY) minY = p.y;
    if (p.x > maxX) maxX = p.x;
    if (p.y > maxY) maxY = p.y;
  }
  if (!points.length) return { x: 0, y: 0, w: 0, h: 0 };
  return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
}

/**
 * 점과 선분의 거리.
 * @param {InkPoint} p
 * @param {InkPoint} a
 * @param {InkPoint} b
 */
function distToSegment(p, a, b) {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len2 = dx * dx + dy * dy;
  if (len2 === 0) return Math.hypot(p.x - a.x, p.y - a.y);
  let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2;
  t = t < 0 ? 0 : t > 1 ? 1 : t;
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
}

/**
 * 시작·끝이 닫혔는지.
 * @param {ReadonlyArray<InkPoint>} points
 * @param {{ w: number, h: number }} [bbox]
 */
export function isClosed(points, bbox = pointsBounds(points)) {
  if (points.length < 3) return false;
  const a = points[0];
  const b = points[points.length - 1];
  return Math.hypot(b.x - a.x, b.y - a.y) < Math.max(bbox.w, bbox.h) * CLOSE_RATIO;
}

/**
 * 직선 판정: 시작–끝 선분에서 모든 점의 최대 벗어남이 길이의 6% 미만.
 * @param {ReadonlyArray<InkPoint>} points
 * @param {{ straighten?: boolean }} [opts]
 * @returns {FittedShape | null}
 */
export function fitLine(points, opts = {}) {
  if (points.length < 2) return null;
  const a = points[0];
  const b = points[points.length - 1];
  const chord = Math.hypot(b.x - a.x, b.y - a.y);
  if (chord < LINE_MIN_LEN) return null;
  let maxDist = 0;
  for (let i = 1; i < points.length - 1; i++) {
    const d = distToSegment(points[i], a, b);
    if (d > maxDist) maxDist = d;
  }
  if (maxDist >= chord * LINE_MAX_DEVIATION) return null;
  let x2 = b.x;
  let y2 = b.y;
  let snapped = false;
  if (opts.straighten !== false) ({ x2, y2, snapped } = snapAngle(a.x, a.y, b.x, b.y));
  return { shape: 'line', x1: a.x, y1: a.y, x2, y2, snapped };
}

/**
 * Ramanujan 근사 타원 둘레.
 * @param {number} rx
 * @param {number} ry
 */
function ellipsePerimeter(rx, ry) {
  return Math.PI * (3 * (rx + ry) - Math.sqrt((3 * rx + ry) * (rx + 3 * ry)));
}

/**
 * 타원·원 판정: bbox에 내접한 타원 기준 정규화 반지름 평균 오차 < 8%.
 * @param {ReadonlyArray<InkPoint>} points
 * @returns {FittedShape | null}
 */
export function fitEllipse(points) {
  if (points.length < 8) return null;
  const bb = pointsBounds(points);
  if (bb.w < MIN_SHAPE_SIZE || bb.h < MIN_SHAPE_SIZE) return null;
  let rx = bb.w / 2;
  let ry = bb.h / 2;
  const cx = bb.x + rx;
  const cy = bb.y + ry;
  const closed = isClosed(points, bb) || pathLength(points) >= ellipsePerimeter(rx, ry) * ARC_COVER_RATIO;
  if (!closed) return null;
  let err = 0;
  for (const p of points) err += Math.abs(Math.hypot((p.x - cx) / rx, (p.y - cy) / ry) - 1);
  err /= points.length;
  if (err >= ELLIPSE_MAX_ERROR) return null;
  let circle = false;
  if (Math.abs(rx - ry) / Math.max(rx, ry) < CIRCLE_RATIO) {
    rx = ry = (rx + ry) / 2;
    circle = true;
  }
  return { shape: 'ellipse', x1: cx - rx, y1: cy - ry, x2: cx + rx, y2: cy + ry, circle };
}

/**
 * 꼭짓점 v의 내각(도).
 * @param {InkPoint} prev
 * @param {InkPoint} v
 * @param {InkPoint} next
 */
function interiorAngle(prev, v, next) {
  const ax = prev.x - v.x;
  const ay = prev.y - v.y;
  const bx = next.x - v.x;
  const by = next.y - v.y;
  const la = Math.hypot(ax, ay);
  const lb = Math.hypot(bx, by);
  if (la === 0 || lb === 0) return 180;
  const c = (ax * bx + ay * by) / (la * lb);
  return Math.acos(c < -1 ? -1 : c > 1 ? 1 : c) * DEG;
}

/**
 * 사각형 판정: 닫힌 획 → RDP(ε = 대각선 4%) → 거의 곧은 꼭짓점 제거 → 4개·내각 90° ± 12° → 기울기 ±8° 안이면 축 정렬.
 * 기울어진 사각형(±8° 밖)은 축 정렬 도형으로 나타낼 수 없어 실패로 본다.
 * @param {ReadonlyArray<InkPoint>} points
 * @returns {FittedShape | null}
 */
export function fitRect(points) {
  if (points.length < 5) return null;
  const bb = pointsBounds(points);
  if (bb.w < MIN_SHAPE_SIZE || bb.h < MIN_SHAPE_SIZE || !isClosed(points, bb)) return null;
  const diag = Math.hypot(bb.w, bb.h);
  const simple = rdp(points, diag * RECT_RDP_RATIO);
  /** @type {InkPoint[]} */
  let poly = simple.slice();
  if (poly.length >= 2) {
    const a = poly[0];
    const b = poly[poly.length - 1];
    if (Math.hypot(b.x - a.x, b.y - a.y) < Math.max(bb.w, bb.h) * CLOSE_RATIO) poly.pop();
  }
  // 변 가운데에서 시작한 획처럼 거의 곧은(내각 ≥ 150°) 꼭짓점을 지운다.
  let changed = true;
  while (changed && poly.length > 4) {
    changed = false;
    for (let i = 0; i < poly.length; i++) {
      const prev = poly[(i - 1 + poly.length) % poly.length];
      const next = poly[(i + 1) % poly.length];
      if (interiorAngle(prev, poly[i], next) >= 150) {
        poly.splice(i, 1);
        changed = true;
        break;
      }
    }
  }
  if (poly.length !== 4) return null;
  let tilt = 0;
  for (let i = 0; i < 4; i++) {
    const prev = poly[(i + 3) % 4];
    const next = poly[(i + 1) % 4];
    if (Math.abs(interiorAngle(prev, poly[i], next) - 90) > RECT_ANGLE_TOL) return null;
    const ex = next.x - poly[i].x;
    const ey = next.y - poly[i].y;
    const a = Math.abs(Math.atan2(ey, ex) * DEG) % 90; // 0~90
    tilt += Math.min(a, 90 - a);
  }
  if (tilt / 4 > RECT_TILT_TOL) return null;
  const cb = pointsBounds(poly);
  return { shape: 'rect', x1: cb.x, y1: cb.y, x2: cb.x + cb.w, y2: cb.y + cb.h };
}

/**
 * 0.5초 멈춤 도형 판정 — 직선 → 타원·원 → 사각형 순서로 첫 통과 도형.
 * @param {ReadonlyArray<InkPoint>} points
 * @param {{ straighten?: boolean }} [opts]
 * @returns {FittedShape | null}
 */
export function fitShape(points, opts = {}) {
  if (points.length < 2 || pathLength(points) < HOLD_MIN_PATH) return null;
  return fitLine(points, opts) ?? fitEllipse(points) ?? fitRect(points);
}

/**
 * 수정 키 드래그 도형(Shift 직선 · Ctrl 사각 · Ctrl+Shift 화살표 · Tab 타원)의 끝점.
 * 직선·화살표는 맞춤 설정이 켜져 있으면 ±4° 맞춤 결과도 돌려준다(맞춤 전 끝점은 raw).
 * @param {ShapeKind} kind
 * @param {{ x: number, y: number }} start
 * @param {{ x: number, y: number }} end
 * @param {{ straighten?: boolean }} [opts]
 * @returns {{ raw: FittedShape, snapped: FittedShape | null }}
 */
export function shapeFromDrag(kind, start, end, opts = {}) {
  const raw = { shape: kind, x1: start.x, y1: start.y, x2: end.x, y2: end.y };
  if ((kind === 'line' || kind === 'arrow') && opts.straighten !== false) {
    const s = snapAngle(start.x, start.y, end.x, end.y);
    if (s.snapped) return { raw, snapped: { ...raw, x2: s.x2, y2: s.y2, snapped: true } };
  }
  return { raw, snapped: null };
}
