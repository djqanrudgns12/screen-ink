// 판서·칠판 화면 UI(4.8) 순수 모델 — 섬 6/8색 펼침 · 4초 40% 물러남 · 커서 회피(bbox+24px) ·
// 펜 색 고리 커서 · 확대 MiniMap 정규화 · 설정 칠판 미리 보기. DOM 의존 0.

import { INK_COLORS, MARKER_OPACITY } from '../ink/inkEngine.js';
import { BOARD_COLORS, resolveInkColor } from '../ink/canvasRender.js';
import { clamp, visibleDocFraction } from '../ink/coords.js';

/** 색 키를 누르거나 색 점에 마우스를 올렸을 때 6/8색 팔레트가 펼쳐지는 시간(ms, 04 S12). */
export const DRAW_PALETTE_EXPAND_MS = 800;

/** 판서 중 무동작 뒤 섬이 40%로 물러나는 시간(ms, 04 S12 · 10 §6). */
export const DRAW_IDLE_DIM_MS = 4000;

/** 물러난 판서 섬 투명도(10 §6). */
export const DRAW_IDLE_OPACITY = 0.4;

/** 커서가 섬 bbox 바깥 몇 px 안에 들어오면 비켜 가는지(A §5). */
export const ISLAND_AVOID_MARGIN_PX = 24;

/** 판서 확대 MiniMap 오른쪽·아래 여백(CSS px). */
export const DRAW_MINIMAP_MARGIN_PX = 20;

const EPS = 1e-6;

/**
 * @typedef {'R' | 'Y' | 'G' | 'O' | 'B' | 'L' | 'W' | 'K'} DrawColorCode
 * @typedef {{ code: DrawColorCode, label: string, hex: string, lightFg: boolean, needsBorder?: boolean }} PenSwatch
 */

/** 판서 기본 6색(R·Y·G·O·B·L, 10 §2 — B만 흰 글자, 나머지는 어두운 글자). @type {ReadonlyArray<PenSwatch>} */
export const DRAW_PEN_SWATCHES = Object.freeze([
  Object.freeze({ code: 'R', label: '빨강', hex: INK_COLORS.R, lightFg: false }),
  Object.freeze({ code: 'Y', label: '노랑', hex: INK_COLORS.Y, lightFg: false }),
  Object.freeze({ code: 'G', label: '초록', hex: INK_COLORS.G, lightFg: false }),
  Object.freeze({ code: 'O', label: '주황', hex: INK_COLORS.O, lightFg: false }),
  Object.freeze({ code: 'B', label: '파랑', hex: INK_COLORS.B, lightFg: true }),
  Object.freeze({ code: 'L', label: '연두', hex: INK_COLORS.L, lightFg: false }),
]);

/** 칠판 추가 2색(W·K). K는 검은 섬 위에서 보이도록 옅은 테두리를 두른다. @type {ReadonlyArray<PenSwatch>} */
export const BOARD_EXTRA_SWATCHES = Object.freeze([
  Object.freeze({ code: 'W', label: '흰색', hex: INK_COLORS.W, lightFg: false }),
  Object.freeze({ code: 'K', label: '검정', hex: INK_COLORS.K, lightFg: true, needsBorder: true }),
]);

/** 칠판 8색(R·Y·G·O·B·L·W·K). @type {ReadonlyArray<PenSwatch>} */
export const BOARD_PEN_SWATCHES = Object.freeze([...DRAW_PEN_SWATCHES, ...BOARD_EXTRA_SWATCHES]);

const VALID_COLORS = new Set(BOARD_PEN_SWATCHES.map((s) => s.code));

/**
 * 판서·칠판 모드 섬인지.
 * @param {string | null | undefined} variant
 */
export function isDrawIslandVariant(variant) {
  return variant === 'draw' || variant === 'board';
}

/**
 * 판서(6색) 또는 칠판(W·K 포함 8색) 팔레트 목록.
 * @param {boolean} [includeBoard=false]
 * @returns {ReadonlyArray<PenSwatch>}
 */
export function getDrawPaletteSwatches(includeBoard = false) {
  return includeBoard ? BOARD_PEN_SWATCHES : DRAW_PEN_SWATCHES;
}

/**
 * 색이 바뀌어 0.8초 팔레트 펼침을 시작해야 하는지.
 * 처음 진입(prevColor 없음)이나 같은 색이면 펼치지 않는다.
 * @param {string | null | undefined} prevColor
 * @param {string | null | undefined} nextColor
 * @param {string | null | undefined} variant
 */
export function shouldExpandDrawPalette(prevColor, nextColor, variant) {
  if (!isDrawIslandVariant(variant)) return false;
  if (!prevColor || !nextColor || prevColor === nextColor) return false;
  return VALID_COLORS.has(/** @type {DrawColorCode} */ (nextColor));
}

/**
 * 판서 섬의 40% 물러남 여부.
 * 마우스를 올렸거나 단추([바꾸기]·[되돌리기])가 있으면 물러나지 않는다.
 * @param {{ explicitIdleDim?: boolean, autoIdleDim?: boolean, hovering?: boolean, hasAction?: boolean }} opts
 */
export function resolveDrawIdleDim({
  explicitIdleDim = undefined,
  autoIdleDim = false,
  hovering = false,
  hasAction = false,
} = {}) {
  if (hovering || hasAction) return false;
  if (typeof explicitIdleDim === 'boolean') return explicitIdleDim;
  return Boolean(autoIdleDim);
}

/**
 * 커서가 섬 bbox + margin(기본 24px) 안에 들어왔는지(A §5).
 * bbox는 DOMRect({ left, top, right, bottom }) 또는 { x, y, w, h } 모두 받는다.
 * @param {{ x: number, y: number } | null | undefined} cursor
 * @param {{ left?: number, top?: number, right?: number, bottom?: number, x?: number, y?: number, w?: number, h?: number, width?: number, height?: number } | null | undefined} bbox
 * @param {number} [margin=ISLAND_AVOID_MARGIN_PX]
 */
export function shouldAvoidIsland(cursor, bbox, margin = ISLAND_AVOID_MARGIN_PX) {
  if (!cursor || !bbox) return false;
  if (!Number.isFinite(cursor.x) || !Number.isFinite(cursor.y)) return false;
  const left = typeof bbox.left === 'number' ? bbox.left : bbox.x;
  const top = typeof bbox.top === 'number' ? bbox.top : bbox.y;
  const right =
    typeof bbox.right === 'number'
      ? bbox.right
      : typeof left === 'number' && typeof (bbox.width ?? bbox.w) === 'number'
        ? left + /** @type {number} */ (bbox.width ?? bbox.w)
        : Number.NaN;
  const bottom =
    typeof bbox.bottom === 'number'
      ? bbox.bottom
      : typeof top === 'number' && typeof (bbox.height ?? bbox.h) === 'number'
        ? top + /** @type {number} */ (bbox.height ?? bbox.h)
        : Number.NaN;
  if (!Number.isFinite(left) || !Number.isFinite(top) || !Number.isFinite(right) || !Number.isFinite(bottom)) {
    return false;
  }
  if (right <= left || bottom <= top) return false;
  const m = Number.isFinite(margin) ? Math.max(0, margin) : ISLAND_AVOID_MARGIN_PX;
  return cursor.x >= left - m && cursor.x <= right + m && cursor.y >= top - m && cursor.y <= bottom + m;
}

/**
 * 별도 창(Tauri ink-island)에 떠 있는 위 가운데 판서 섬의 기준 bbox(오버레이 CSS px).
 * @param {number} viewportW
 * @param {{ width?: number, height?: number, top?: number }} [opts]
 */
export function estimateTopIslandRect(viewportW, { width = 320, height = 40, top = 14 } = {}) {
  if (!Number.isFinite(viewportW) || viewportW <= 0) return null;
  const w = Math.min(width, viewportW);
  const left = (viewportW - w) / 2;
  return { left, top, right: left + w, bottom: top + height };
}

/**
 * 칠판 섬 문구("칠판 · 흰색 · 1 / 1")용 구분점 배치 계산.
 * text가 "칠판 · 흰색"(Rust/fake)이든 text="칠판"+detail="흰색"(미리보기)이든 중복 없이 잇는다.
 * @param {{ text?: string, detail?: string, value?: string }} opts
 */
export function formatBoardIslandMeta({ text = '', detail = '', value = '' } = {}) {
  const cleanText = String(text ?? '').trim();
  const cleanDetail = String(detail ?? '').trim();
  const cleanValue = String(value ?? '').trim();
  const dotBeforeDetail = Boolean(cleanText && cleanDetail);
  const dotBeforeValue = Boolean((cleanText || cleanDetail) && cleanValue);
  const combined = [cleanText, cleanDetail, cleanValue].filter(Boolean).join(' · ');
  return { dotBeforeDetail, dotBeforeValue, combined };
}

/**
 * 칠판 배경·쪽 번호로 표준 섬 문구("칠판 · 흰색" + "1 / 1")를 만든다.
 * @param {{ background?: 'frame' | 'white' | 'black', pageIndex?: number, pageCount?: number }} opts
 */
export function formatBoardIslandText({ background = 'white', pageIndex = 0, pageCount = 1 } = {}) {
  const bgLabel = background === 'black' ? '검은색' : '흰색';
  const total = Math.max(1, Math.floor(Number(pageCount) || 1));
  const current = clamp(Math.floor(Number(pageIndex) || 0) + 1, 1, total);
  const text = `칠판 · ${bgLabel}`;
  const value = `${current} / ${total}`;
  return { text, value, combined: `${text} · ${value}` };
}

/**
 * DrawCanvas 펜 색 고리 커서 규격(화면 CSS px, 04 S12).
 * 펜 = 색 고리 + 굵기 점, 형광펜 = 색 고리 + 반투명 점(0.38), 지우개 = 점선 고리(점 없음), 그 밖 = null.
 * @param {{
 *   tool: string,
 *   color?: string,
 *   widthPx?: number,
 *   background?: 'frame' | 'white' | 'black',
 *   eraserCss?: number,
 * }} opts
 * @returns {{ kind: 'pen' | 'marker' | 'eraser', diameter: number, dot: number, color: string, dotOpacity: number, dashed: boolean } | null}
 */
export function computeCursorRing({
  tool,
  color = 'R',
  widthPx = 4,
  background = 'frame',
  eraserCss = 16,
} = /** @type {any} */ ({})) {
  const w = clamp(Number.isFinite(widthPx) ? widthPx : 4, 1, 30);
  if (tool === 'eraser') {
    const d = Math.max(16, Math.round(Number.isFinite(eraserCss) ? eraserCss : 16));
    return {
      kind: 'eraser',
      diameter: d,
      dot: 0,
      color: background === 'white' ? '#1A1A1A' : '#FFFFFF',
      dotOpacity: 0,
      dashed: true,
    };
  }
  if (tool === 'pen') {
    const dot = Math.max(2, Math.round(w));
    return {
      kind: 'pen',
      diameter: Math.max(18, dot + 12),
      dot,
      color: resolveInkColor(color, background),
      dotOpacity: 1,
      dashed: false,
    };
  }
  if (tool === 'marker') {
    const dot = Math.max(4, Math.round(w));
    return {
      kind: 'marker',
      diameter: Math.max(20, dot + 12),
      dot,
      color: resolveInkColor(color, background),
      dotOpacity: MARKER_OPACITY,
      dashed: false,
    };
  }
  return null;
}

/**
 * DrawSession의 진입 순간 커서(가상 화면 물리 px) → 오버레이 로컬 CSS 좌표.
 * @param {{ cursor?: { x: number, y: number }, monitor?: { bounds: { x: number, y: number, w: number, h: number }, scale: number } } | null | undefined} session
 */
export function sessionCursorToCss(session) {
  const c = session?.cursor;
  const m = session?.monitor;
  if (!c || !m?.bounds) return null;
  if (!Number.isFinite(c.x) || !Number.isFinite(c.y)) return null;
  const scale = Number.isFinite(m.scale) && m.scale > 0 ? m.scale : 1;
  const x = (c.x - m.bounds.x) / scale;
  const y = (c.y - m.bounds.y) / scale;
  const maxW = m.bounds.w / scale;
  const maxH = m.bounds.h / scale;
  if (x < 0 || y < 0 || x > maxW || y > maxH) return null;
  return { x, y };
}

/**
 * MiniMap 보이는 영역 사각형(0..1)을 테두리 밖으로 삐져나가지 않게 정규화한다.
 * @param {{ x?: number, y?: number, w?: number, h?: number } | null | undefined} rect
 * @returns {{ x: number, y: number, w: number, h: number }}
 */
export function normalizeMinimapViewport(rect) {
  const rawW = Number.isFinite(rect?.w) ? /** @type {number} */ (rect.w) : 1;
  const rawH = Number.isFinite(rect?.h) ? /** @type {number} */ (rect.h) : 1;
  const w = clamp(rawW, 0, 1);
  const h = clamp(rawH, 0, 1);
  const rawX = Number.isFinite(rect?.x) ? /** @type {number} */ (rect.x) : 0;
  const rawY = Number.isFinite(rect?.y) ? /** @type {number} */ (rect.y) : 0;
  const x = clamp(rawX, 0, Math.max(0, 1 - w));
  const y = clamp(rawY, 0, Math.max(0, 1 - h));
  return { x, y, w, h };
}

/**
 * 판서 확대(view.s > 1) 중 MiniMap 표시 상태와 정규화 영역(R-ZOOM-6).
 * @param {import('../ink/coords.js').ViewTransform} view
 * @param {import('../ink/coords.js').Size} viewport
 * @param {import('../ink/coords.js').Size} docSize
 */
export function drawMinimapState(view, viewport, docSize) {
  const s = Number.isFinite(view?.s) ? view.s : 1;
  const visible = s > 1 + EPS;
  if (!visible || !(docSize?.w > 0) || !(docSize?.h > 0) || !(viewport?.w > 0) || !(viewport?.h > 0)) {
    return { visible: false, viewport: { x: 0, y: 0, w: 1, h: 1 } };
  }
  return {
    visible: true,
    viewport: normalizeMinimapViewport(visibleDocFraction(view, viewport, docSize)),
  };
}

/**
 * 설정 화면(DrawZoomPage)의 흰/검은 칠판 작은 미리 보기 데이터.
 * 고른 펜 색·굵기를 그대로 비추고, 검은 칠판의 검정 펜(K)은 흰색 자동 치환을 반영한다.
 * @param {{ penColor?: string, penWidth?: number }} [opts]
 */
export function buildBoardPreviewSamples({ penColor = 'R', penWidth = 4 } = {}) {
  const w = clamp(Number.isFinite(penWidth) ? penWidth : 4, 1, 30);
  const strokeWidth = Math.round(clamp(w * 0.45 + 0.8, 1.5, 6.5) * 10) / 10;
  return [
    {
      id: /** @type {'white'} */ ('white'),
      key: 'W',
      label: '흰 칠판',
      bg: BOARD_COLORS.white,
      strokeColor: resolveInkColor(penColor, 'white'),
      strokeWidth,
      substituted: false,
    },
    {
      id: /** @type {'black'} */ ('black'),
      key: 'K',
      label: '검은 칠판',
      bg: BOARD_COLORS.black,
      strokeColor: resolveInkColor(penColor, 'black'),
      strokeWidth,
      substituted: penColor === 'K',
    },
  ];
}
