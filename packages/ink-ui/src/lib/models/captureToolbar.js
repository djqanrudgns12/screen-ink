// 선택 툴바(R2-UI-TB, 04 S08) 순수 모델 — 어느 모니터에 띄울지, 어디에 붙일지, 단추·툴팁·크기 배지.
// 좌표: 사각형·모니터 = 가상 화면 물리 px, 배치 = 그 모니터 오버레이 창의 CSS px.

import { computeTopIslandRect, normalizeRect, placeAnchoredPopover, rectsOverlap } from './popoverPlacement.js';
import { defaultSlotCombo, formatComboReadable } from './shortcutDisplay.js';

/** 범위와 툴바 사이(CSS px) */
export const TOOLBAR_GAP = 8;
/** 모니터 가장자리에서 띄우는 거리(CSS px) */
export const TOOLBAR_EDGE = 8;

/**
 * 단추(왼→오): 닫기·화면에 고정·저장·복사(주황).
 * @type {ReadonlyArray<{ id: string, icon: string, label: string, labelKey: string, slot: string, command: string, primary?: boolean }>}
 */
export const TOOLBAR_ACTIONS = Object.freeze([
  { id: 'cancel', icon: 'close', label: '취소', labelKey: 'capture.action.cancel', slot: 'c-cancel', command: 'cancel' },
  { id: 'pin', icon: 'pin', label: '화면에 고정', labelKey: 'capture.action.pin', slot: '', command: 'pin' },
  { id: 'save', icon: 'save', label: '저장', labelKey: 'capture.action.save', slot: 'c-save', command: 'save' },
  { id: 'copy', icon: 'copy', label: '복사', labelKey: 'capture.action.copy', slot: 'c-copy', command: 'copy', primary: true },
]);

/**
 * @typedef {{ x: number, y: number, w: number, h: number }} PhysRect
 * @typedef {{ id: number, bounds: PhysRect, scale?: number }} MonitorLike
 */

/**
 * @param {PhysRect} b
 * @param {number} x
 * @param {number} y
 */
function contains(b, x, y) {
  return x >= b.x && x < b.x + b.w && y >= b.y && y < b.y + b.h;
}

/**
 * 툴바를 띄울 모니터: 범위 오른쪽 아래 귀가 있는 모니터 → 없으면 가운데가 있는 모니터 → 없으면 겹치는 면적이 가장 큰 모니터.
 * @param {PhysRect | null | undefined} rect
 * @param {MonitorLike[] | null | undefined} monitors
 * @returns {number | null}
 */
export function toolbarOwner(rect, monitors) {
  if (!rect || !Array.isArray(monitors) || monitors.length === 0 || !(rect.w > 0) || !(rect.h > 0)) return null;
  const corner = monitors.find((m) => contains(m.bounds, rect.x + rect.w - 1, rect.y + rect.h - 1));
  if (corner) return corner.id;
  const center = monitors.find((m) => contains(m.bounds, rect.x + rect.w / 2, rect.y + rect.h / 2));
  if (center) return center.id;
  let best = null;
  let bestArea = 0;
  for (const m of monitors) {
    const b = m.bounds;
    const w = Math.min(rect.x + rect.w, b.x + b.w) - Math.max(rect.x, b.x);
    const h = Math.min(rect.y + rect.h, b.y + b.h) - Math.max(rect.y, b.y);
    const area = w > 0 && h > 0 ? w * h : 0;
    if (area > bestArea) {
      bestArea = area;
      best = m.id;
    }
  }
  return best;
}

/**
 * 툴바 자리(CSS px, 왼쪽 위). 범위 오른쪽 아래 **바깥** → 모자라면 범위 **위** → 둘 다 없으면 범위 **안** 오른쪽 아래.
 * 가로는 범위 오른쪽 끝에 맞추고 화면 가장자리 안으로 밀어 넣는다.
 * 범위가 이 모니터 밖으로 나가 있어도(여러 모니터·혼합 배율) 보이는 부분 기준으로 자른 뒤 계산한다.
 * @param {{ x: number, y: number, width: number, height: number }} sel  이 모니터 CSS 좌표의 범위(잘리지 않은 값이어도 됨)
 * @param {{ w: number, h: number }} viewport  오버레이 창 CSS 크기
 * @param {{ w: number, h: number }} size  툴바 CSS 크기
 * @returns {{ x: number, y: number, side: 'below' | 'above' | 'inside' }}
 */
export function placeToolbar(sel, viewport, size) {
  const vw = Math.max(0, viewport.w);
  const vh = Math.max(0, viewport.h);
  const top = Math.max(0, sel.y);
  const right = Math.min(vw, sel.x + sel.width);
  const bottom = Math.min(vh, sel.y + sel.height);
  const maxX = Math.max(TOOLBAR_EDGE, vw - TOOLBAR_EDGE - size.w);
  const x = Math.min(maxX, Math.max(TOOLBAR_EDGE, right - size.w));
  const below = bottom + TOOLBAR_GAP;
  if (below + size.h <= vh - TOOLBAR_EDGE) {
    return { x, y: below, side: 'below' };
  }
  const above = top - TOOLBAR_GAP - size.h;
  if (above >= TOOLBAR_EDGE) {
    return { x, y: above, side: 'above' };
  }
  const inside = Math.max(TOOLBAR_EDGE, Math.min(vh - TOOLBAR_EDGE - size.h, bottom - TOOLBAR_GAP - size.h));
  return { x, y: inside, side: 'inside' };
}

/**
 * 범위 모서리 크기 배지 자리(CSS px):
 * - `corner === 'top-left'`(F1): 범위 왼쪽 위 바깥 → (`avoidRects`와 겹치거나 위가 모자라면) 범위 안쪽 왼쪽 위(여유 8px) →
 *   그래도 겹치면 범위 왼쪽 아래 바깥 → 겹치는 막대 바로 위 바깥 순으로 안 겹치는 자리를 고른다.
 * - `corner === 'top-right'`(F2): 창 안쪽 오른쪽 위 꼬리표(화면 가장자리 8px 안쪽 유지).
 * @param {{ x: number, y: number, width: number, height: number }} sel
 * @param {{ w: number, h: number }} viewport
 * @param {{ w: number, h: number }} badgeSize
 * @param {{
 *   corner?: 'top-left' | 'top-right',
 *   avoidRects?: ReadonlyArray<import('./popoverPlacement.js').RectLike | null | undefined>,
 * }} [opts]
 * @returns {{ x: number, y: number, inside: boolean }}
 */
export function placeSizeBadge(sel, viewport, badgeSize, opts = {}) {
  const vw = Math.max(0, viewport.w);
  const vh = Math.max(0, viewport.h);
  const maxX = Math.max(TOOLBAR_EDGE, vw - TOOLBAR_EDGE - badgeSize.w);
  const maxY = Math.max(TOOLBAR_EDGE, vh - TOOLBAR_EDGE - badgeSize.h);
  const clampX = (/** @type {number} */ v) => Math.min(maxX, Math.max(TOOLBAR_EDGE, v));
  const clampY = (/** @type {number} */ v) => Math.min(maxY, Math.max(TOOLBAR_EDGE, v));

  if (opts.corner === 'top-right') {
    const visRight = Math.min(vw, sel.x + sel.width);
    const visTop = Math.max(0, sel.y);
    return {
      x: Math.round(clampX(visRight - TOOLBAR_GAP - badgeSize.w)),
      y: Math.round(clampY(visTop + TOOLBAR_GAP)),
      inside: true,
    };
  }

  const left = Math.max(0, sel.x);
  const top = Math.max(0, sel.y);
  const bottom = Math.min(vh, sel.y + sel.height);
  const obstacles = Array.isArray(opts.avoidRects)
    ? opts.avoidRects.map((r) => normalizeRect(r)).filter((r) => r !== null)
    : [];
  const collides = (/** @type {number} */ x, /** @type {number} */ y) =>
    obstacles.some((obs) => rectsOverlap({ x, y, w: badgeSize.w, h: badgeSize.h }, obs, 4));

  const x1 = clampX(left);
  const above = top - TOOLBAR_GAP - badgeSize.h;
  if (above >= TOOLBAR_EDGE) {
    const y1 = Math.min(maxY, above);
    if (!collides(x1, y1)) {
      return { x: x1, y: y1, inside: false };
    }
  }

  const insetX =
    obstacles.length > 0 && left >= TOOLBAR_EDGE && sel.width >= badgeSize.w + TOOLBAR_GAP * 2
      ? TOOLBAR_GAP
      : 0;
  const x2 = clampX(left + insetX);
  const y2 = clampY(top + TOOLBAR_GAP);
  if (!collides(x2, y2)) {
    return { x: x2, y: y2, inside: true };
  }

  if (obstacles.length > 0) {
    const below = bottom + TOOLBAR_GAP;
    if (below + badgeSize.h <= vh - TOOLBAR_EDGE && !collides(x1, below)) {
      return { x: x1, y: below, inside: false };
    }
    const minObsTop = Math.min(...obstacles.map((o) => o.top));
    const aboveObs = minObsTop - TOOLBAR_GAP - badgeSize.h;
    if (aboveObs >= TOOLBAR_EDGE && !collides(x1, aboveObs)) {
      return { x: x1, y: aboveObs, inside: false };
    }
  }

  return { x: x2, y: y2, inside: true };
}

/**
 * 물리 px 사각형 크기를 `1280 × 720` 배지 문자열로 바꾼다.
 * @param {{ w: number, h: number } | null | undefined} rect
 * @returns {string}
 */
export function formatSizeBadge(rect) {
  if (!rect || !Number.isFinite(rect.w) || !Number.isFinite(rect.h)) return '';
  const w = Math.round(rect.w);
  const h = Math.round(rect.h);
  if (w <= 0 || h <= 0) return '';
  return `${w} × ${h}`;
}

/**
 * 칸 id → 지금 조합. 설정 상태의 단축키 행에서.
 * @param {Array<{ id: string, combo: string }> | null | undefined} rows
 * @returns {Record<string, string>}
 */
export function comboMap(rows) {
  /** @type {Record<string, string>} */
  const out = {};
  if (!Array.isArray(rows)) return out;
  for (const r of rows) {
    if (r && typeof r.id === 'string' && typeof r.combo === 'string') out[r.id] = r.combo;
  }
  return out;
}

// ─────────────── 캡처 주석 도구(R2-UI-TB · R2-ANN) ───────────────

/**
 * 주석 도구 8종(왼→오) + 한 글자 단축키(R·O·A·P·H·M·T·E).
 * @type {ReadonlyArray<{ id: string, icon: string, label: string, labelKey: string, key: string, tool: 'shape' | 'pen' | 'marker' | 'mosaic' | 'text' | 'eraser', shape?: 'rect' | 'ellipse' | 'arrow' }>}
 */
export const TOOLBAR_ANNOTATE_TOOLS = Object.freeze([
  { id: 'rect', icon: 'rect', label: '사각형', labelKey: 'capture.tool.rect', key: 'R', tool: 'shape', shape: 'rect' },
  { id: 'ellipse', icon: 'ellipse', label: '타원', labelKey: 'capture.tool.ellipse', key: 'O', tool: 'shape', shape: 'ellipse' },
  { id: 'arrow', icon: 'arrow', label: '화살표', labelKey: 'capture.tool.arrow', key: 'A', tool: 'shape', shape: 'arrow' },
  { id: 'pen', icon: 'pen', label: '펜', labelKey: 'capture.tool.pen', key: 'P', tool: 'pen' },
  { id: 'marker', icon: 'marker', label: '형광펜', labelKey: 'capture.tool.marker', key: 'H', tool: 'marker' },
  { id: 'mosaic', icon: 'mosaic', label: '모자이크', labelKey: 'capture.tool.mosaic', key: 'M', tool: 'mosaic' },
  { id: 'text', icon: 'text', label: '글자', labelKey: 'capture.tool.text', key: 'T', tool: 'text' },
  { id: 'eraser', icon: 'eraser', label: '지우개', labelKey: 'capture.tool.eraser', key: 'E', tool: 'eraser' },
]);

/** 색·굵기 패널 토글 단추 메타. */
export const TOOLBAR_STYLE_TRIGGER = Object.freeze({
  id: 'style',
  icon: 'slider',
  label: '색·굵기',
  labelKey: 'capture.action.style',
});

/**
 * 되돌리기·다시 하기.
 * @type {ReadonlyArray<{ id: string, icon: string, label: string, labelKey: string, slot: string, command: string }>}
 */
export const TOOLBAR_HISTORY = Object.freeze([
  { id: 'undo', icon: 'undo', label: '되돌리기', labelKey: 'capture.history.undo', slot: 'd-undo', command: 'annot.undo' },
  { id: 'redo', icon: 'redo', label: '다시 하기', labelKey: 'capture.history.redo', slot: 'd-redo', command: 'annot.redo' },
]);

/** 기존 보조 알약 호환용 상수(AnnotateOptions.svelte 참조). */
export const ANNOT_COLORS = Object.freeze(/** @type {const} */ (['R', 'Y', 'G', 'O', 'B', 'L']));
export const ANNOT_WIDTHS = Object.freeze([
  { px: 2, label: '가는' },
  { px: 4, label: '보통' },
  { px: 8, label: '굵은' },
]);
export const MOSAIC_BLOCKS = Object.freeze([
  { px: 6, label: '작게' },
  { px: 10, label: '보통' },
  { px: 16, label: '크게' },
]);

/** 단추 한 칸(32 + 사이 2)·구분선(1 + 양옆 4)·안쪽 여백 — CaptureToolbar.svelte 치수와 같다. */
export const TOOLBAR_BUTTON_W = 34;
export const TOOLBAR_DIVIDER_W = 9;
export const TOOLBAR_PAD_W = 8;
/** 패널/보조 옵션과 툴바 사이(CSS px). */
export const TOOLBAR_OPTIONS_GAP = 8;

/**
 * 툴바 너비 추정(실제 크기를 재기 전 첫 배치). 묶음 사이에 구분선.
 * @param {ReadonlyArray<ReadonlyArray<unknown>>} groups
 */
export function estimateToolbarWidth(groups) {
  const shown = groups.filter((g) => g.length > 0);
  const buttons = shown.reduce((n, g) => n + g.length, 0);
  return buttons * TOOLBAR_BUTTON_W + Math.max(0, shown.length - 1) * TOOLBAR_DIVIDER_W + TOOLBAR_PAD_W;
}

/**
 * 도구 단추: 같은 도구를 다시 누르면 선택 해제(= 범위 조정으로 돌아감, 04 S08).
 * @param {string | null} current
 * @param {string} id
 * @returns {string | null}
 */
export function toggleAnnotTool(current, id) {
  if (!TOOLBAR_ANNOTATE_TOOLS.some((t) => t.id === id)) return current;
  return current === id ? null : id;
}

/**
 * 도구가 고른 상태에서 보일 색·굵기 패널 종류(R2-ANN-1~2). 지우개도 굵기 슬라이더를 연다.
 * @param {string | null} toolId
 * @returns {'stroke' | 'marker' | 'text' | 'mosaic' | 'eraser' | 'none'}
 */
export function annotOptionsKind(toolId) {
  if (!toolId) return 'none';
  if (toolId === 'marker' || toolId === 'text' || toolId === 'mosaic' || toolId === 'eraser') return toolId;
  if (TOOLBAR_ANNOTATE_TOOLS.some((t) => t.id === toolId)) return 'stroke';
  return 'none';
}

/**
 * 두 사각형이 겹치는 면적(CSS px²).
 * @param {import('./popoverPlacement.js').BoxRect} a
 * @param {import('./popoverPlacement.js').BoxRect} b
 */
function overlapArea(a, b) {
  const w = Math.min(a.right, b.right) - Math.max(a.left, b.left);
  const h = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
  return w > 0 && h > 0 ? w * h : 0;
}

/**
 * 색·굵기 패널 자리(CSS px, R2-FIX-11 · G19 · #15):
 * - `opts.selection`이 없으면 기존 툴바 위(`above`) 우선 → 상단 섬·화면 부족 시 아래(`below`) 배치.
 * - `opts.selection`이 있으면 선택 범위(그릴 자리)와 겹치지 않는 자리를 우선 고른다:
 *   툴바 반대편 바깥 → 범위 오른쪽/왼쪽 바깥 → 범위 아래/위 바깥 → 불가피하면 겹침이 가장 작은 자리.
 * - 어느 경우든 모니터 가장자리 `TOOLBAR_EDGE`(8px) 안쪽을 유지하고 상단 섬·툴바를 피한다.
 * @param {{ x: number, y: number, side: 'below' | 'above' | 'inside' }} bar  placeToolbar 결과
 * @param {{ w: number, h: number }} barSize
 * @param {{ w: number, h: number }} optsSize
 * @param {{ w: number, h: number, left?: number, top?: number }} viewport
 * @param {number | null} [anchorX]  툴바 왼쪽 기준 단추 중심 X
 * @param {{
 *   avoidTopIsland?: boolean,
 *   avoidRects?: ReadonlyArray<import('./popoverPlacement.js').RectLike | null | undefined>,
 *   monitor?: import('./popoverPlacement.js').MonitorBoundsLike | null,
 *   selection?: import('./popoverPlacement.js').RectLike | null,
 * }} [opts]
 * @returns {{ x: number, y: number, side: 'above' | 'below' | 'left' | 'right', arrowX: number, showArrow: boolean }}
 */
export function placeToolOptions(bar, barSize, optsSize, viewport, anchorX = null, opts = {}) {
  const vw = Math.max(0, viewport.w);
  const vh = Math.max(0, viewport.h);
  const bounds = {
    left: Number.isFinite(viewport.left) ? /** @type {number} */ (viewport.left) : 0,
    top: Number.isFinite(viewport.top) ? /** @type {number} */ (viewport.top) : 0,
    w: vw,
    h: vh,
  };
  const avoidRects =
    opts.avoidRects !== undefined
      ? opts.avoidRects
      : opts.avoidTopIsland === false
        ? []
        : (() => {
            const island = computeTopIslandRect(bounds, opts.monitor ?? null);
            return island ? [island] : [];
          })();

  const hasAnchorX = typeof anchorX === 'number' && Number.isFinite(anchorX);
  const targetCenter = hasAnchorX ? bar.x + anchorX : bar.x + barSize.w - 20;
  const arrowPad = Math.min(18, Math.max(8, optsSize.w / 4));
  const computeArrowX = (/** @type {number} */ left) =>
    Math.round(
      Math.min(Math.max(arrowPad, targetCenter - left), Math.max(arrowPad, optsSize.w - arrowPad)),
    );

  const rawSel = opts.selection ? normalizeRect(opts.selection) : null;
  const selRect = rawSel
    ? normalizeRect({
        left: Math.max(bounds.left, rawSel.left),
        top: Math.max(bounds.top, rawSel.top),
        right: Math.min(bounds.left + vw, rawSel.right),
        bottom: Math.min(bounds.top + vh, rawSel.bottom),
      })
    : null;

  if (!selRect) {
    const placed = placeAnchoredPopover(
      { left: bar.x, top: bar.y, width: barSize.w, height: barSize.h },
      { width: optsSize.w, height: optsSize.h, minHeight: optsSize.h },
      bounds,
      {
        gap: TOOLBAR_OPTIONS_GAP,
        edge: TOOLBAR_EDGE,
        align: hasAnchorX ? 'center' : 'end',
        targetCenterX: hasAnchorX ? bar.x + anchorX : null,
        preferredSide: 'above',
        avoidRects,
      },
    );
    return {
      x: placed.left,
      y: placed.top,
      side: placed.side,
      arrowX: computeArrowX(placed.left),
      showArrow: true,
    };
  }

  const gap = TOOLBAR_OPTIONS_GAP;
  const edge = TOOLBAR_EDGE;
  const pw = Math.max(0, optsSize.w);
  const ph = Math.max(0, optsSize.h);
  const minX = bounds.left + edge;
  const maxLeft = Math.max(minX, bounds.left + vw - edge - pw);
  const minY = bounds.top + edge;
  const maxTop = Math.max(minY, bounds.top + vh - edge - ph);
  const clampX = (/** @type {number} */ x) => Math.round(Math.min(maxLeft, Math.max(minX, x)));
  const clampY = (/** @type {number} */ y) => Math.round(Math.min(maxTop, Math.max(minY, y)));

  const barRect = {
    left: bar.x,
    top: bar.y,
    right: bar.x + barSize.w,
    bottom: bar.y + barSize.h,
    width: barSize.w,
    height: barSize.h,
  };
  const obstacles = Array.isArray(avoidRects)
    ? avoidRects.map((r) => normalizeRect(r)).filter((r) => r !== null)
    : [];

  /** @type {Array<{ x: number, y: number, side: 'above' | 'below' | 'left' | 'right', priority: number }>} */
  const candidates = [];
  const pushCand = (
    /** @type {number} */ rawX,
    /** @type {number} */ rawY,
    /** @type {'above' | 'below' | 'left' | 'right'} */ side,
    /** @type {number} */ priority,
  ) => {
    candidates.push({ x: clampX(rawX), y: clampY(rawY), side, priority });
  };

  const baseAnchorX = hasAnchorX ? targetCenter - pw / 2 : barRect.right - pw;
  const xVariants = [
    baseAnchorX,
    selRect.right - pw,
    selRect.left,
    ...obstacles.flatMap((obs) => [obs.left - gap - pw, obs.right + gap]),
  ];
  const sideYVariants = [
    selRect.bottom - ph,
    barRect.bottom - ph,
    selRect.top,
    (selRect.top + selRect.bottom - ph) / 2,
    barRect.top - gap - ph,
    barRect.bottom + gap,
  ];

  // 0) 툴바 반대편 밀착(세로 잘림 없이 툴바 바로 아래/위에 붙고 툴바와 가로로 겹칠 때만 우선)
  if (bar.side === 'below' || bar.side === 'above') {
    const rawY = bar.side === 'below' ? barRect.bottom + gap : barRect.top - gap - ph;
    if (rawY >= minY && rawY <= maxTop) {
      const side = bar.side === 'below' ? 'below' : 'above';
      for (const xv of xVariants) {
        const cx = clampX(xv);
        if (cx + pw > barRect.left && cx < barRect.right) {
          candidates.push({ x: cx, y: Math.round(rawY), side, priority: 0 });
        }
      }
    }
  }

  // 1) 범위 오른쪽 바깥(오른쪽에 실제로 들어갈 때만)
  const rightX = selRect.right + gap;
  if (rightX <= maxLeft) {
    for (const yv of sideYVariants) {
      candidates.push({ x: Math.round(rightX), y: clampY(yv), side: 'right', priority: 1 });
    }
  }

  // 2) 범위 왼쪽 바깥(왼쪽에 실제로 들어갈 때만) + 툴바 좌우 바깥
  const leftX = selRect.left - gap - pw;
  if (leftX >= minX) {
    for (const yv of sideYVariants) {
      candidates.push({ x: Math.round(leftX), y: clampY(yv), side: 'left', priority: 2 });
    }
  }
  if (barRect.right + gap <= maxLeft) {
    for (const yv of sideYVariants) {
      candidates.push({ x: Math.round(barRect.right + gap), y: clampY(yv), side: 'right', priority: 2 });
    }
  }
  if (barRect.left - gap - pw >= minX) {
    for (const yv of sideYVariants) {
      candidates.push({ x: Math.round(barRect.left - gap - pw), y: clampY(yv), side: 'left', priority: 2 });
    }
  }

  // 3) 범위 반대편 세로 바깥(실제로 아래/위에 들어갈 때만)
  if (selRect.bottom + gap <= maxTop) {
    for (const xv of xVariants) {
      candidates.push({ x: clampX(xv), y: Math.round(selRect.bottom + gap), side: 'below', priority: 3 });
    }
  }
  if (selRect.top - gap - ph >= minY) {
    for (const xv of xVariants) {
      candidates.push({ x: clampX(xv), y: Math.round(selRect.top - gap - ph), side: 'above', priority: 3 });
    }
  }

  // 4) 최후 폴백(화면 전체 범위 등 모든 자리가 범위와 겹칠 때)
  pushCand(baseAnchorX, barRect.top - gap - ph, 'above', 4);
  pushCand(baseAnchorX, barRect.bottom + gap, 'below', 4);
  pushCand(minX, minY, 'above', 4);
  pushCand(maxLeft, minY, 'above', 4);
  pushCand(minX, maxTop, 'below', 4);
  pushCand(maxLeft, maxTop, 'below', 4);

  const barCenterY = (barRect.top + barRect.bottom) / 2;
  let best = null;
  for (const cand of candidates) {
    const rect = {
      left: cand.x,
      top: cand.y,
      right: cand.x + pw,
      bottom: cand.y + ph,
      width: pw,
      height: ph,
    };
    const selOverlap = overlapArea(rect, selRect);
    const barOverlap = overlapArea(rect, barRect);
    const obsOverlap = obstacles.reduce((acc, obs) => acc + overlapArea(rect, obs), 0);
    const cx = cand.x + pw / 2;
    const cy = cand.y + ph / 2;
    const distToAnchor = Math.hypot(cx - targetCenter, cy - barCenterY);
    const score = {
      ...cand,
      selOverlap,
      barOverlap,
      obsOverlap,
      distToAnchor,
    };
    if (
      !best ||
      score.selOverlap < best.selOverlap - 0.5 ||
      (Math.abs(score.selOverlap - best.selOverlap) <= 0.5 &&
        (score.barOverlap < best.barOverlap - 0.5 ||
          (Math.abs(score.barOverlap - best.barOverlap) <= 0.5 &&
            (score.obsOverlap < best.obsOverlap - 0.5 ||
              (Math.abs(score.obsOverlap - best.obsOverlap) <= 0.5 &&
                (score.priority < best.priority ||
                  (score.priority === best.priority && score.distToAnchor < best.distToAnchor)))))))
    ) {
      best = score;
    }
  }

  const chosen = best ?? { x: minX, y: minY, side: /** @type {'above'} */ ('above') };
  const alignedHoriz = chosen.x + pw > barRect.left && chosen.x < barRect.right;
  const showArrow =
    alignedHoriz &&
    ((chosen.side === 'above' && Math.abs(chosen.y + ph + gap - barRect.top) <= 2) ||
      (chosen.side === 'below' && Math.abs(chosen.y - (barRect.bottom + gap)) <= 2));

  return {
    x: chosen.x,
    y: chosen.y,
    side: chosen.side,
    arrowX: computeArrowX(chosen.x),
    showArrow,
  };
}

/**
 * 툴팁 `"사각형 · R"` 또는 `"복사 · Enter"` / `"저장 · ⌘S"`. 키를 비웠으면 이름만.
 * @param {{ label: string, slot?: string, key?: string }} action
 * @param {Record<string, string>} combos
 * @param {'windows' | 'macos'} [platform='windows']
 * @param {string} [labelOverride]
 * @returns {string}
 */
export function toolbarTooltip(action, combos, platform = 'windows', labelOverride = '') {
  const label = labelOverride || action.label;
  let readable = '';
  if (action.slot) {
    const combo = Object.prototype.hasOwnProperty.call(combos ?? {}, action.slot)
      ? combos[action.slot]
      : defaultSlotCombo(action.slot, platform);
    readable = combo ? formatComboReadable(combo, platform) : '';
  } else if (action.key) {
    readable = action.key;
  }
  return readable ? `${label} · ${readable}` : label;
}
