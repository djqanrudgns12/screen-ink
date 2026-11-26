// 지우개·고르기(S)용 충돌 판정(A §2.4) — 항목 bbox, 점/선분 거리, 지우개 쓸기, 모서리 핸들. 문서 좌표. DOM 의존 0.

/** @typedef {{ x: number, y: number, w: number, h: number }} Rect */
/** @typedef {'nw' | 'ne' | 'sw' | 'se'} Handle */
/** @typedef {import('./inkEngine.js').Item} Item */

/** 화살표 머리 날개 각도(도). */
export const ARROW_HEAD_DEG = 28;
/** 펜 필압 최대 배수(0.6 + 0.8 × 1). */
const MAX_PRESSURE_MUL = 1.4;

/**
 * @param {number} px
 * @param {number} py
 * @param {number} ax
 * @param {number} ay
 * @param {number} bx
 * @param {number} by
 */
export function distPointToSegment(px, py, ax, ay, bx, by) {
  const dx = bx - ax;
  const dy = by - ay;
  const len2 = dx * dx + dy * dy;
  if (len2 === 0) return Math.hypot(px - ax, py - ay);
  let t = ((px - ax) * dx + (py - ay) * dy) / len2;
  t = t < 0 ? 0 : t > 1 ? 1 : t;
  return Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
}

/**
 * @param {number} ax @param {number} ay @param {number} bx @param {number} by
 * @param {number} cx @param {number} cy
 */
function orient(ax, ay, bx, by, cx, cy) {
  return (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
}

/**
 * 선분 ab와 선분 cd의 최소 거리(교차하면 0).
 * @param {number} ax @param {number} ay @param {number} bx @param {number} by
 * @param {number} cx @param {number} cy @param {number} dx @param {number} dy
 */
export function segmentToSegment(ax, ay, bx, by, cx, cy, dx, dy) {
  const o1 = orient(ax, ay, bx, by, cx, cy);
  const o2 = orient(ax, ay, bx, by, dx, dy);
  const o3 = orient(cx, cy, dx, dy, ax, ay);
  const o4 = orient(cx, cy, dx, dy, bx, by);
  if (((o1 > 0 && o2 < 0) || (o1 < 0 && o2 > 0)) && ((o3 > 0 && o4 < 0) || (o3 < 0 && o4 > 0))) return 0;
  return Math.min(
    distPointToSegment(ax, ay, cx, cy, dx, dy),
    distPointToSegment(bx, by, cx, cy, dx, dy),
    distPointToSegment(cx, cy, ax, ay, bx, by),
    distPointToSegment(dx, dy, ax, ay, bx, by),
  );
}

/**
 * @param {Rect} r
 * @param {number} d
 * @returns {Rect}
 */
export function inflateRect(r, d) {
  return { x: r.x - d, y: r.y - d, w: r.w + d * 2, h: r.h + d * 2 };
}

/**
 * @param {Rect} a
 * @param {Rect} b
 */
export function rectsIntersect(a, b) {
  return a.x <= b.x + b.w && b.x <= a.x + a.w && a.y <= b.y + b.h && b.y <= a.y + a.h;
}

/**
 * @param {Rect} r
 * @param {number} x
 * @param {number} y
 */
export function pointInRect(r, x, y) {
  return x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
}

/**
 * 여러 사각형을 감싸는 사각형(없으면 null).
 * @param {ReadonlyArray<Rect | null | undefined>} rects
 * @returns {Rect | null}
 */
export function unionRect(rects) {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const r of rects) {
    if (!r) continue;
    if (r.x < minX) minX = r.x;
    if (r.y < minY) minY = r.y;
    if (r.x + r.w > maxX) maxX = r.x + r.w;
    if (r.y + r.h > maxY) maxY = r.y + r.h;
  }
  return minX === Infinity ? null : { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
}

/**
 * 두 점이 만드는 정규화 사각형.
 * @param {number} x1 @param {number} y1 @param {number} x2 @param {number} y2
 * @returns {Rect}
 */
export function rectFromPoints(x1, y1, x2, y2) {
  return { x: Math.min(x1, x2), y: Math.min(y1, y2), w: Math.abs(x2 - x1), h: Math.abs(y2 - y1) };
}

/**
 * 화살표 머리 두 날개 끝점.
 * @param {number} x1 @param {number} y1 @param {number} x2 @param {number} y2
 * @param {number} headLen
 */
export function arrowHeadPoints(x1, y1, x2, y2, headLen) {
  const a = Math.atan2(y2 - y1, x2 - x1);
  const w = (ARROW_HEAD_DEG * Math.PI) / 180;
  return [
    { x: x2 - headLen * Math.cos(a - w), y: y2 - headLen * Math.sin(a - w) },
    { x: x2 - headLen * Math.cos(a + w), y: y2 - headLen * Math.sin(a + w) },
  ];
}

/**
 * 타원 윤곽을 꺾은선으로(판정용).
 * @param {number} x1 @param {number} y1 @param {number} x2 @param {number} y2
 * @param {number} [n]
 */
export function ellipsePolyline(x1, y1, x2, y2, n = 48) {
  const cx = (x1 + x2) / 2;
  const cy = (y1 + y2) / 2;
  const rx = Math.abs(x2 - x1) / 2;
  const ry = Math.abs(y2 - y1) / 2;
  const out = [];
  for (let i = 0; i <= n; i++) {
    const t = (i / n) * Math.PI * 2;
    out.push({ x: cx + rx * Math.cos(t), y: cy + ry * Math.sin(t) });
  }
  return out;
}

/**
 * 항목의 경계 상자(굵기 반·화살표 머리 포함).
 * @param {Item} item
 * @returns {Rect}
 */
export function itemBBox(item) {
  switch (item.kind) {
    case 'stroke': {
      const half = (item.width * (item.tool === 'pen' ? MAX_PRESSURE_MUL : 1)) / 2;
      let minX = Infinity;
      let minY = Infinity;
      let maxX = -Infinity;
      let maxY = -Infinity;
      for (const p of item.points) {
        if (p.x < minX) minX = p.x;
        if (p.y < minY) minY = p.y;
        if (p.x > maxX) maxX = p.x;
        if (p.y > maxY) maxY = p.y;
      }
      if (minX === Infinity) return { x: 0, y: 0, w: 0, h: 0 };
      return inflateRect({ x: minX, y: minY, w: maxX - minX, h: maxY - minY }, half);
    }
    case 'shape': {
      const half = item.width / 2;
      /** @type {Array<{x:number,y:number}>} */
      const pts = [
        { x: item.x1, y: item.y1 },
        { x: item.x2, y: item.y2 },
      ];
      if (item.shape === 'arrow') pts.push(...arrowHeadPoints(item.x1, item.y1, item.x2, item.y2, item.headLen ?? 0));
      let minX = Infinity;
      let minY = Infinity;
      let maxX = -Infinity;
      let maxY = -Infinity;
      for (const p of pts) {
        if (p.x < minX) minX = p.x;
        if (p.y < minY) minY = p.y;
        if (p.x > maxX) maxX = p.x;
        if (p.y > maxY) maxY = p.y;
      }
      return inflateRect({ x: minX, y: minY, w: maxX - minX, h: maxY - minY }, half);
    }
    case 'text':
    case 'mosaic':
      return { x: item.x, y: item.y, w: item.w, h: item.h };
    default:
      return { x: 0, y: 0, w: 0, h: 0 };
  }
}

/**
 * bbox를 새로 계산해 붙인 복사본.
 * @template {Item} T
 * @param {T} item
 * @returns {T}
 */
export function withBBox(item) {
  return { ...item, bbox: itemBBox(item) };
}

/**
 * 판정용 윤곽선 목록(꺾은선 여러 개). 글자·모자이크는 상자 안 전체가 판정 영역이라 빈 배열.
 * @param {Item} item
 * @returns {Array<Array<{x:number,y:number}>>}
 */
function outlines(item) {
  if (item.kind === 'stroke') return [item.points];
  if (item.kind !== 'shape') return [];
  const { x1, y1, x2, y2 } = item;
  if (item.shape === 'line') return [[{ x: x1, y: y1 }, { x: x2, y: y2 }]];
  if (item.shape === 'arrow') {
    const [l, r] = arrowHeadPoints(x1, y1, x2, y2, item.headLen ?? 0);
    return [
      [{ x: x1, y: y1 }, { x: x2, y: y2 }],
      [l, { x: x2, y: y2 }, r],
    ];
  }
  if (item.shape === 'rect') {
    return [
      [
        { x: x1, y: y1 },
        { x: x2, y: y1 },
        { x: x2, y: y2 },
        { x: x1, y: y2 },
        { x: x1, y: y1 },
      ],
    ];
  }
  return [ellipsePolyline(x1, y1, x2, y2)];
}

/**
 * 그려진 선의 반굵기(필압 포함 최대).
 * @param {Item} item
 */
function halfWidth(item) {
  if (item.kind === 'stroke') return (item.width * (item.tool === 'pen' ? MAX_PRESSURE_MUL : 1)) / 2;
  if (item.kind === 'shape') return item.width / 2;
  return 0;
}

/**
 * 한 점(반경 tol)이 항목에 닿는가.
 * @param {Item} item
 * @param {number} x
 * @param {number} y
 * @param {number} tol
 */
export function hitItemAtPoint(item, x, y, tol) {
  const bb = item.bbox ?? itemBBox(item);
  if (!pointInRect(inflateRect(bb, tol), x, y)) return false;
  if (item.kind === 'text' || item.kind === 'mosaic') return true;
  const reach = tol + halfWidth(item);
  for (const line of outlines(item)) {
    if (line.length === 1 && Math.hypot(line[0].x - x, line[0].y - y) <= reach) return true;
    for (let i = 1; i < line.length; i++) {
      if (distPointToSegment(x, y, line[i - 1].x, line[i - 1].y, line[i].x, line[i].y) <= reach) return true;
    }
  }
  return false;
}

/**
 * 지우개 한 걸음(선분 a→b, 반경 r)이 항목에 닿는가.
 * @param {Item} item
 * @param {number} ax @param {number} ay @param {number} bx @param {number} by
 * @param {number} radius
 */
export function hitItemBySegment(item, ax, ay, bx, by, radius) {
  const bb = item.bbox ?? itemBBox(item);
  const sweep = inflateRect(rectFromPoints(ax, ay, bx, by), radius);
  if (!rectsIntersect(sweep, bb)) return false;
  if (item.kind === 'text' || item.kind === 'mosaic') {
    const box = inflateRect(bb, radius);
    if (pointInRect(box, ax, ay) || pointInRect(box, bx, by)) return true;
    const corners = [
      [box.x, box.y, box.x + box.w, box.y],
      [box.x + box.w, box.y, box.x + box.w, box.y + box.h],
      [box.x + box.w, box.y + box.h, box.x, box.y + box.h],
      [box.x, box.y + box.h, box.x, box.y],
    ];
    return corners.some(([cx, cy, dx, dy]) => segmentToSegment(ax, ay, bx, by, cx, cy, dx, dy) === 0);
  }
  const reach = radius + halfWidth(item);
  for (const line of outlines(item)) {
    if (line.length === 1) {
      if (distPointToSegment(line[0].x, line[0].y, ax, ay, bx, by) <= reach) return true;
      continue;
    }
    for (let i = 1; i < line.length; i++) {
      const p = line[i - 1];
      const q = line[i];
      if (segmentToSegment(ax, ay, bx, by, p.x, p.y, q.x, q.y) <= reach) return true;
    }
  }
  return false;
}

/**
 * 점 아래 맨 위 항목의 인덱스(없으면 -1).
 * @param {ReadonlyArray<Item>} items
 * @param {number} x
 * @param {number} y
 * @param {number} tol
 */
export function topItemAt(items, x, y, tol) {
  for (let i = items.length - 1; i >= 0; i--) if (hitItemAtPoint(items[i], x, y, tol)) return i;
  return -1;
}

/**
 * 끌어 고르기 사각형과 겹치는 항목 id.
 * @param {ReadonlyArray<Item>} items
 * @param {Rect} rect
 * @returns {string[]}
 */
export function itemsInRect(items, rect) {
  return items.filter((it) => rectsIntersect(rect, it.bbox ?? itemBBox(it))).map((it) => it.id);
}

/**
 * 고른 상자의 모서리 핸들 판정.
 * @param {Rect} box
 * @param {number} x
 * @param {number} y
 * @param {number} radius  문서 px(화면 핸들 크기 / 배율)
 * @returns {Handle | null}
 */
export function hitHandle(box, x, y, radius) {
  /** @type {Array<[Handle, number, number]>} */
  const corners = [
    ['nw', box.x, box.y],
    ['ne', box.x + box.w, box.y],
    ['sw', box.x, box.y + box.h],
    ['se', box.x + box.w, box.y + box.h],
  ];
  for (const [h, cx, cy] of corners) if (Math.abs(x - cx) <= radius && Math.abs(y - cy) <= radius) return h;
  return null;
}
