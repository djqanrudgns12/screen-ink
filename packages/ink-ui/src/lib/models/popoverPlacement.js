// 팝오버·패널 공통 배치 순수 모델(G19):
// - 상단 섬(결과·안내 섬: 화면 위 가운데 560×88)과 겹치지 않게 회피 영역 계산
// - 우선 방향(위/아래) 공간이 모자라거나 섬과 겹치면 반대쪽으로 뒤집어 열기
// - 양쪽 다 모자라면 화면(및 모니터 경계) 안쪽·섬 바깥으로 밀어 넣고, 접을 수 있는 팝오버는 높이 압축
// DOM 의존 0.

/** 상단 섬 창 기본 논리 폭(CSS px, Tauri ISLAND_SIZE.0). */
export const TOP_ISLAND_WIDTH = 560;
/** 상단 섬 창 기본 논리 높이(CSS px, Tauri ISLAND_SIZE.1). */
export const TOP_ISLAND_HEIGHT = 88;
/** 작업 영역 위쪽에서 섬 창까지 기본 간격(CSS px, Windows·macOS show_island 기준). */
export const TOP_ISLAND_TOP_OFFSET = 12;
/** 모니터 가장자리 기본 최소 여백(CSS px). */
export const POPOVER_EDGE = 8;

/**
 * @typedef {{
 *   left?: number,
 *   top?: number,
 *   right?: number,
 *   bottom?: number,
 *   x?: number,
 *   y?: number,
 *   width?: number,
 *   height?: number,
 *   w?: number,
 *   h?: number,
 * }} RectLike
 *
 * @typedef {{ left: number, top: number, right: number, bottom: number, width: number, height: number }} BoxRect
 *
 * @typedef {{
 *   bounds: { x: number, y: number, w: number, h: number },
 *   work?: { x: number, y: number, w: number, h: number },
 *   scale?: number,
 * }} MonitorBoundsLike
 */

/**
 * @param {number} v
 * @param {number} lo
 * @param {number} hi
 */
function clamp(v, lo, hi) {
  if (hi < lo) return lo;
  return Math.min(hi, Math.max(lo, v));
}

/**
 * 다양한 사각형 표현(`{ left, top, right, bottom }`, `{ left, top, width, height }`, `{ x, y, w, h }`)을 정규화한다.
 * 넓이가 0 이하이거나 유한하지 않으면 `null`.
 * @param {RectLike | null | undefined} rect
 * @returns {BoxRect | null}
 */
export function normalizeRect(rect) {
  if (!rect || typeof rect !== 'object') return null;
  const left = Number.isFinite(rect.left) ? /** @type {number} */ (rect.left) : rect.x;
  const top = Number.isFinite(rect.top) ? /** @type {number} */ (rect.top) : rect.y;
  if (typeof left !== 'number' || typeof top !== 'number' || !Number.isFinite(left) || !Number.isFinite(top)) {
    return null;
  }
  const width = Number.isFinite(rect.width)
    ? /** @type {number} */ (rect.width)
    : Number.isFinite(rect.w)
      ? /** @type {number} */ (rect.w)
      : Number.isFinite(rect.right)
        ? /** @type {number} */ (rect.right) - left
        : Number.NaN;
  const height = Number.isFinite(rect.height)
    ? /** @type {number} */ (rect.height)
    : Number.isFinite(rect.h)
      ? /** @type {number} */ (rect.h)
      : Number.isFinite(rect.bottom)
        ? /** @type {number} */ (rect.bottom) - top
        : Number.NaN;
  if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
    return null;
  }
  return {
    left,
    top,
    right: left + width,
    bottom: top + height,
    width,
    height,
  };
}

/**
 * 두 사각형이 겹치는지 판정한다(경계만 맞닿으면 false, `margin`만큼 확장 가능).
 * @param {RectLike | null | undefined} a
 * @param {RectLike | null | undefined} b
 * @param {number} [margin=0]
 * @returns {boolean}
 */
export function rectsOverlap(a, b, margin = 0) {
  const ra = normalizeRect(a);
  const rb = normalizeRect(b);
  if (!ra || !rb) return false;
  const m = Number.isFinite(margin) ? Math.max(0, margin) : 0;
  return (
    ra.left < rb.right + m &&
    ra.right > rb.left - m &&
    ra.top < rb.bottom + m &&
    ra.bottom > rb.top - m
  );
}

/**
 * 현재 모니터/뷰포트 기준 상단 섬(결과·안내 섬 560×88)의 CSS 회피 영역을 계산한다.
 * - `monitor.work`가 있으면 메뉴 막대·작업 표시줄을 반영한 작업 영역 위 가운데에 놓는다.
 * - 여러 모니터·혼합 배율(예: 오른쪽 모니터, 음수 좌표 모니터, 1.5x/2x DPI)에서도 현재 창 좌표로 변환해 잘라낸다.
 * @param {{ left?: number, top?: number, x?: number, y?: number, w?: number, h?: number, width?: number, height?: number } | null | undefined} viewport
 * @param {MonitorBoundsLike | null | undefined} [monitor=null]
 * @param {{ width?: number, height?: number, topOffset?: number, margin?: number }} [opts]
 * @returns {BoxRect | null}
 */
export function computeTopIslandRect(viewport, monitor = null, opts = {}) {
  if (!viewport) return null;
  const bx = Number.isFinite(viewport.left)
    ? /** @type {number} */ (viewport.left)
    : Number.isFinite(viewport.x)
      ? /** @type {number} */ (viewport.x)
      : 0;
  const by = Number.isFinite(viewport.top)
    ? /** @type {number} */ (viewport.top)
    : Number.isFinite(viewport.y)
      ? /** @type {number} */ (viewport.y)
      : 0;
  const vw = Number(viewport.w ?? viewport.width ?? 0);
  const vh = Number(viewport.h ?? viewport.height ?? 0);
  if (!Number.isFinite(vw) || !Number.isFinite(vh) || vw <= 0 || vh <= 0) return null;

  let workLeft = bx;
  let workTop = by;
  let workRight = bx + vw;
  let workBottom = by + vh;

  if (monitor?.bounds && monitor.bounds.w > 0 && monitor.bounds.h > 0) {
    const scale = Number.isFinite(monitor.scale) && /** @type {number} */ (monitor.scale) > 0
      ? /** @type {number} */ (monitor.scale)
      : 1;
    const mb = monitor.bounds;
    const mw = monitor.work && monitor.work.w > 0 && monitor.work.h > 0 ? monitor.work : mb;
    const rawL = bx + (mw.x - mb.x) / scale;
    const rawT = by + (mw.y - mb.y) / scale;
    const rawR = bx + (mw.x + mw.w - mb.x) / scale;
    const rawB = by + (mw.y + mw.h - mb.y) / scale;
    const clL = clamp(rawL, bx, bx + vw);
    const clT = clamp(rawT, by, by + vh);
    const clR = clamp(rawR, clL, bx + vw);
    const clB = clamp(rawB, clT, by + vh);
    if (clR > clL && clB > clT) {
      workLeft = clL;
      workTop = clT;
      workRight = clR;
      workBottom = clB;
    }
  }

  const workW = workRight - workLeft;
  const workH = workBottom - workTop;
  const targetW = Number.isFinite(opts.width) ? Math.max(0, /** @type {number} */ (opts.width)) : TOP_ISLAND_WIDTH;
  const targetH = Number.isFinite(opts.height) ? Math.max(0, /** @type {number} */ (opts.height)) : TOP_ISLAND_HEIGHT;
  const topOffset = Number.isFinite(opts.topOffset)
    ? Math.max(0, /** @type {number} */ (opts.topOffset))
    : TOP_ISLAND_TOP_OFFSET;
  const margin = Number.isFinite(opts.margin) ? Math.max(0, /** @type {number} */ (opts.margin)) : 0;

  const w = Math.min(targetW, workW);
  const h = Math.min(targetH, workH);
  if (w <= 0 || h <= 0) return null;

  const rawLeft = workLeft + (workW - w) / 2;
  const rawTop = workTop + Math.min(topOffset, Math.max(0, workH - h));
  const left = Math.max(bx, rawLeft - margin);
  const top = Math.max(by, rawTop - margin);
  const right = Math.min(bx + vw, rawLeft + w + margin);
  const bottom = Math.min(by + vh, rawTop + h + margin);
  return { left, top, right, bottom, width: right - left, height: bottom - top };
}

/**
 * `[minY, maxBottom]`에서 수평 겹침 장애물들을 제외한 자유 세로 구간 목록을 구한다.
 * @param {number} minY
 * @param {number} maxBottom
 * @param {ReadonlyArray<BoxRect>} hObstacles
 * @returns {Array<{ top: number, bottom: number }>}
 */
function freeVerticalBands(minY, maxBottom, hObstacles) {
  if (maxBottom <= minY) return [];
  const blocked = hObstacles
    .map((o) => ({ top: Math.max(minY, o.top), bottom: Math.min(maxBottom, o.bottom) }))
    .filter((o) => o.bottom > o.top)
    .sort((a, b) => a.top - b.top);

  /** @type {Array<{ top: number, bottom: number }>} */
  const merged = [];
  for (const b of blocked) {
    const prev = merged[merged.length - 1];
    if (prev && b.top <= prev.bottom) {
      prev.bottom = Math.max(prev.bottom, b.bottom);
    } else {
      merged.push({ ...b });
    }
  }

  /** @type {Array<{ top: number, bottom: number }>} */
  const free = [];
  let cursor = minY;
  for (const b of merged) {
    if (b.top > cursor) free.push({ top: cursor, bottom: b.top });
    cursor = Math.max(cursor, b.bottom);
  }
  if (cursor < maxBottom) free.push({ top: cursor, bottom: maxBottom });
  return free;
}

/**
 * 장애물을 피하면서 `preferredTop`에 가장 가까운 세로 위치를 찾는다.
 * 맞는 자유 구간이 없으면 전체 `[minY, maxBottom]` 안으로 민다.
 * @param {number} minY
 * @param {number} maxBottom
 * @param {ReadonlyArray<BoxRect>} hObstacles
 * @param {number} height
 * @param {number} preferredTop
 */
function clampIntoBestBand(minY, maxBottom, hObstacles, height, preferredTop) {
  const bands = freeVerticalBands(minY, maxBottom, hObstacles);
  let bestTop = null;
  let bestDist = Infinity;
  for (const band of bands) {
    if (band.bottom - band.top < height) continue;
    const candidate = clamp(preferredTop, band.top, band.bottom - height);
    const dist = Math.abs(candidate - preferredTop);
    if (dist < bestDist) {
      bestDist = dist;
      bestTop = candidate;
    }
  }
  if (bestTop !== null) return bestTop;
  return clamp(preferredTop, minY, Math.max(minY, maxBottom - height));
}

/**
 * 기준 사각형(`anchorRect`)에 붙는 팝오버·패널의 자리를 계산한다(CSS px).
 * 규칙:
 * 1) 가로는 정렬 기준(`start`/`center`/`end` 또는 `targetCenterX`)을 따라 모니터 경계(`[bx+edge, bx+bw-edge]`) 안으로 클램프.
 * 2) 우선 쪽(`preferredSide`)에 전체 높이가 들어오고 장애물(상단 섬 등)과 겹치지 않으면 그쪽에 연다.
 * 3) 모자라거나 장애물과 겹치면 반대쪽으로 뒤집어 연다.
 * 4) 둘 다 전체 높이가 안 들어오면 최소 높이(`minHeight`)가 들어오는 쪽 중 여유가 넓은 쪽을 고른다(접을 수 있으면 높이 압축).
 * 5) 양쪽 다 모자라면 여유가 넓은 쪽을 고르고, 장애물을 피한 화면 안쪽으로 밀어 넣는다.
 *
 * @param {RectLike} anchorRect
 * @param {{ width?: number, height?: number, w?: number, h?: number, minHeight?: number, minH?: number }} popoverSize
 * @param {{ left?: number, top?: number, x?: number, y?: number, w?: number, h?: number, width?: number, height?: number }} bounds
 * @param {{
 *   gap?: number,
 *   edge?: number,
 *   preferredSide?: 'above' | 'below',
 *   align?: 'start' | 'center' | 'end',
 *   targetCenterX?: number | null,
 *   avoidRects?: ReadonlyArray<RectLike | null | undefined>,
 *   avoidGap?: number,
 * }} [opts]
 * @returns {{
 *   left: number,
 *   top: number,
 *   right: number,
 *   bottom: number,
 *   width: number,
 *   maxHeight: number,
 *   side: 'above' | 'below',
 *   clamped: boolean,
 * }}
 */
export function placeAnchoredPopover(anchorRect, popoverSize, bounds, opts = {}) {
  const gap = Number.isFinite(opts.gap) ? Math.max(0, /** @type {number} */ (opts.gap)) : 6;
  const edge = Number.isFinite(opts.edge) ? Math.max(0, /** @type {number} */ (opts.edge)) : POPOVER_EDGE;
  const avoidGap = Number.isFinite(opts.avoidGap) ? Math.max(0, /** @type {number} */ (opts.avoidGap)) : 0;
  /** @type {'above' | 'below'} */
  const pref = opts.preferredSide === 'below' ? 'below' : 'above';
  /** @type {'above' | 'below'} */
  const alt = pref === 'above' ? 'below' : 'above';

  const width = Math.max(0, Number(popoverSize?.width ?? popoverSize?.w ?? 0));
  const height = Math.max(0, Number(popoverSize?.height ?? popoverSize?.h ?? 0));
  const rawMinH = popoverSize?.minHeight ?? popoverSize?.minH;
  const minHeight = Number.isFinite(rawMinH) ? Math.min(height, Math.max(0, /** @type {number} */ (rawMinH))) : height;
  const collapsible = minHeight < height;

  const bx = Number.isFinite(bounds?.left)
    ? /** @type {number} */ (bounds.left)
    : Number.isFinite(bounds?.x)
      ? /** @type {number} */ (bounds.x)
      : 0;
  const by = Number.isFinite(bounds?.top)
    ? /** @type {number} */ (bounds.top)
    : Number.isFinite(bounds?.y)
      ? /** @type {number} */ (bounds.y)
      : 0;
  const bw = Math.max(0, Number(bounds?.w ?? bounds?.width ?? 0));
  const bh = Math.max(0, Number(bounds?.h ?? bounds?.height ?? 0));

  const minX = bx + edge;
  const maxRight = bx + bw - edge;
  const minY = by + edge;
  const maxBottom = by + bh - edge;
  const innerH = Math.max(0, maxBottom - minY);

  const aLeft = Number.isFinite(anchorRect?.left)
    ? /** @type {number} */ (anchorRect.left)
    : Number.isFinite(anchorRect?.x)
      ? /** @type {number} */ (anchorRect.x)
      : bx;
  const aTop = Number.isFinite(anchorRect?.top)
    ? /** @type {number} */ (anchorRect.top)
    : Number.isFinite(anchorRect?.y)
      ? /** @type {number} */ (anchorRect.y)
      : by;
  const aW = Math.max(
    0,
    Number.isFinite(anchorRect?.width)
      ? /** @type {number} */ (anchorRect.width)
      : Number.isFinite(anchorRect?.w)
        ? /** @type {number} */ (anchorRect.w)
        : Number.isFinite(anchorRect?.right)
          ? /** @type {number} */ (anchorRect.right) - aLeft
          : 0,
  );
  const aH = Math.max(
    0,
    Number.isFinite(anchorRect?.height)
      ? /** @type {number} */ (anchorRect.height)
      : Number.isFinite(anchorRect?.h)
        ? /** @type {number} */ (anchorRect.h)
        : Number.isFinite(anchorRect?.bottom)
          ? /** @type {number} */ (anchorRect.bottom) - aTop
          : 0,
  );
  const aRight = aLeft + aW;
  const aBottom = aTop + aH;

  // 1) 가로 배치 및 모니터 경계 클램프
  let rawLeft = aLeft;
  if (typeof opts.targetCenterX === 'number' && Number.isFinite(opts.targetCenterX)) {
    rawLeft = opts.targetCenterX - width / 2;
  } else if (opts.align === 'end') {
    rawLeft = aRight - width;
  } else if (opts.align === 'center') {
    rawLeft = (aLeft + aRight) / 2 - width / 2;
  }
  const maxLeft = Math.max(minX, maxRight - width);
  const left = Math.round(clamp(rawLeft, minX, maxLeft));
  const right = left + width;

  // 2) 가로로 겹치는 장애물(상단 섬 등)만 추린다
  /** @type {BoxRect[]} */
  const hObstacles = [];
  if (Array.isArray(opts.avoidRects)) {
    for (const raw of opts.avoidRects) {
      const r = normalizeRect(raw);
      if (!r) continue;
      const inflated =
        avoidGap > 0
          ? {
              left: r.left - avoidGap,
              top: r.top - avoidGap,
              right: r.right + avoidGap,
              bottom: r.bottom + avoidGap,
              width: r.width + avoidGap * 2,
              height: r.height + avoidGap * 2,
            }
          : r;
      if (inflated.left < right && inflated.right > left) {
        hObstacles.push(inflated);
      }
    }
  }
  hObstacles.sort((a, b) => a.top - b.top);

  // 3) 위·아래 장애물 없는 여유 공간 계산
  const naturalAboveBottom = aTop - gap;
  const aboveBottom = Math.min(maxBottom, naturalAboveBottom);
  let aboveCeiling = minY;
  for (const obs of hObstacles) {
    if (obs.top < aboveBottom) {
      aboveCeiling = Math.max(aboveCeiling, obs.bottom);
    }
  }
  const availAboveViewport = Math.max(0, aboveBottom - minY);
  const availAboveClear = Math.max(0, aboveBottom - aboveCeiling);

  const naturalBelowTop = aBottom + gap;
  let belowStart = Math.max(minY, naturalBelowTop);
  let belowFloor = maxBottom;
  for (const obs of hObstacles) {
    if (obs.bottom <= belowStart) continue;
    if (obs.top < belowStart && obs.bottom > belowStart) {
      belowStart = Math.max(belowStart, obs.bottom);
    } else if (obs.top >= belowStart) {
      belowFloor = Math.min(belowFloor, obs.top);
    }
  }
  const availBelowViewport = Math.max(0, maxBottom - Math.max(minY, naturalBelowTop));
  const availBelowClear = Math.max(0, belowFloor - belowStart);

  const space = {
    above: { clear: availAboveClear, vp: availAboveViewport },
    below: { clear: availBelowClear, vp: availBelowViewport },
  };

  // 4) 열 쪽(side)과 높이(maxHeight) 결정
  /** @type {'above' | 'below'} */
  let side = pref;
  let maxHeight = height;
  let squeezedBoth = false;

  if (space[pref].clear >= height) {
    side = pref;
    maxHeight = height;
  } else if (space[alt].clear >= height) {
    side = alt;
    maxHeight = height;
  } else if (collapsible && (space[pref].clear >= minHeight || space[alt].clear >= minHeight)) {
    if (space[pref].clear >= minHeight && space[pref].clear >= space[alt].clear) {
      side = pref;
      maxHeight = Math.min(height, space[pref].clear);
    } else if (space[alt].clear >= minHeight) {
      side = alt;
      maxHeight = Math.min(height, space[alt].clear);
    } else {
      side = pref;
      maxHeight = Math.min(height, space[pref].clear);
    }
  } else {
    squeezedBoth = true;
    if (
      space[alt].clear > space[pref].clear ||
      (space[alt].clear === space[pref].clear && space[alt].vp > space[pref].vp)
    ) {
      side = alt;
    } else {
      side = pref;
    }

    if (collapsible) {
      const bands = freeVerticalBands(minY, maxBottom, hObstacles);
      const largestFreeH = bands.reduce((m, b) => Math.max(m, b.bottom - b.top), 0);
      const cap = largestFreeH >= minHeight ? largestFreeH : innerH;
      const desired = Math.max(Math.min(height, space[side].clear), Math.min(minHeight, cap));
      maxHeight = Math.min(height, Math.max(0, desired));
    } else {
      maxHeight = height;
    }
  }

  const roundedMaxH = Math.round(maxHeight);

  // 5) 최종 세로 위치 및 클램프 판정
  let top;
  let clamped = false;

  if (!squeezedBoth && side === 'above') {
    const rawTop = naturalAboveBottom - roundedMaxH;
    if (rawTop >= aboveCeiling && rawTop + roundedMaxH <= maxBottom) {
      top = Math.round(rawTop);
      clamped = false;
    } else {
      top = Math.round(clampIntoBestBand(minY, maxBottom, hObstacles, roundedMaxH, rawTop));
      clamped = true;
    }
  } else if (!squeezedBoth && side === 'below') {
    const rawTop = belowStart;
    if (rawTop >= minY && rawTop + roundedMaxH <= belowFloor) {
      top = Math.round(rawTop);
      clamped = top !== Math.round(naturalBelowTop);
    } else {
      top = Math.round(clampIntoBestBand(minY, maxBottom, hObstacles, roundedMaxH, naturalBelowTop));
      clamped = true;
    }
  } else {
    const preferredTop = side === 'above' ? naturalAboveBottom - roundedMaxH : naturalBelowTop;
    top = Math.round(clampIntoBestBand(minY, maxBottom, hObstacles, roundedMaxH, preferredTop));
    clamped = true;
  }

  return {
    left,
    top,
    right: left + width,
    bottom: top + roundedMaxH,
    width,
    maxHeight: roundedMaxH,
    side,
    clamped,
  };
}
