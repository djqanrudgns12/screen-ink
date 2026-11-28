// 판서 엔진(A §2.2·§2.3·§2.6·§3, R-DRAW-0~10, R-BOARD-1·2) — 순수 JS 상태 기계. DOM·Canvas·Tauri 의존 0.
// 문서(InkDocument v1) 변경은 applyCommand / invertCommand 리듀서로만 일어난다(05 §6). 실행 취소 200단계.
// 입력은 DOM 사건을 흉내 낸 평범한 객체(화면 CSS 좌표 + 시각 ms)로 받고, 바깥에 알릴 일은 InkEffect 배열로 돌려준다.

import {
  IDENTITY_VIEW,
  centerOn,
  clamp,
  clampView,
  docSizeOf,
  edgePushDelta,
  nextZoomStep,
  panBy,
  sameView,
  screenWidthToDoc,
  viewToDoc,
  zoomAt,
  DEFAULT_START_ZOOM,
} from './coords.js';
import { createPointSmoother } from './oneEuro.js';
import { pathLength, simplifyStroke, SHORT_STROKE_LEN } from './rdp.js';
import { fitShape, pointsBounds, shapeFromDrag, HOLD_MIN_PATH, HOLD_MOVE_PX, HOLD_MS } from './shapeFit.js';
import {
  hitHandle,
  hitItemBySegment,
  itemsInRect,
  pointInRect,
  rectFromPoints,
  topItemAt,
  unionRect,
  withBBox,
} from './hitTest.js';
import {
  DRAW_COLOR_KEYS,
  DRAW_TOOL_KEYS,
  LATER_DRAW_KEYS,
  buildDrawKeymap,
  createImeGuard,
  defaultDrawCombo,
  matchBoardPageKey,
  matchDrawKey,
  normalizeCombo,
} from './drawKeys.js';

import { INK_COLORS, INK_COLOR_CODES } from '../models/palette.js';
import { translate } from '../i18n/core.js';

/**
 * @typedef {'R' | 'Y' | 'G' | 'O' | 'B' | 'L' | 'W' | 'K' | 'NV' | 'PU' | 'PK' | 'TL' | 'BR' | 'GR' | 'LY' | 'SB' | (string & {})} InkColor
 * @typedef {'frame' | 'white' | 'black'} InkBackground
 * @typedef {'pen' | 'marker' | 'text' | 'select' | 'eraser' | 'pan'} InkTool
 * @typedef {'draw' | 'board' | 'capture'} InkMode
 * @typedef {{ x: number, y: number, w: number, h: number }} Rect
 * @typedef {import('./coords.js').ViewTransform} ViewTransform
 * @typedef {import('./coords.js').Size} Size
 * @typedef {{ x: number, y: number, p: number, t: number }} StrokePoint  t = 획 시작부터 ms
 *
 * @typedef {Object} StrokeItem
 * @property {string} id
 * @property {'stroke'} kind
 * @property {'pen' | 'marker'} tool
 * @property {InkColor} color
 * @property {number} width   문서 px(획을 만든 순간 화면 굵기 / 배율)
 * @property {number} opacity pen = 1, marker = 0.38
 * @property {boolean} dashed
 * @property {StrokePoint[]} points
 * @property {Rect} [bbox]
 *
 * @typedef {Object} ShapeItem
 * @property {string} id
 * @property {'shape'} kind
 * @property {'line' | 'rect' | 'ellipse' | 'arrow'} shape
 * @property {InkColor} color
 * @property {number} width
 * @property {number} opacity
 * @property {boolean} dashed
 * @property {number} x1
 * @property {number} y1
 * @property {number} x2
 * @property {number} y2
 * @property {number} [headLen]
 * @property {Rect} [bbox]
 *
 * @typedef {Object} TextItem
 * @property {string} id
 * @property {'text'} kind
 * @property {InkColor} color
 * @property {string} fontFamily
 * @property {number} fontSize  문서 px(= 화면 굵기 × 6 / 배율)
 * @property {number} x
 * @property {number} y
 * @property {number} w
 * @property {number} h
 * @property {string} text
 * @property {Rect} [bbox]
 *
 * @typedef {Object} MosaicItem  묶음 B(캡처 주석) 전용 — 타입만 예약
 * @property {string} id
 * @property {'mosaic'} kind
 * @property {number} x
 * @property {number} y
 * @property {number} w
 * @property {number} h
 * @property {number} blockSize
 * @property {Rect} [bbox]
 *
 * @typedef {StrokeItem | ShapeItem | TextItem | MosaicItem} Item
 * @typedef {{ id: string, background: InkBackground, items: Item[] }} BoardPage
 *
 * @typedef {Object} InkDocument  스키마 v1(A §2.2) — Rust keptInk와 같은 JSON
 * @property {1} schemaVersion
 * @property {string} id
 * @property {InkMode} mode
 * @property {number} monitorId
 * @property {string} layoutFingerprint
 * @property {Size} docSize
 * @property {Size} physSize
 * @property {number} scale
 * @property {number | null} frameId
 * @property {InkBackground} background   기본 바탕(판서 = frame, 칠판 = white) — 새 장·W/K 되돌림 기준
 * @property {BoardPage[]} pages
 * @property {number} activePage
 * @property {ViewTransform} view
 *
 * @typedef {{ type: 'add', page: number, item: Item, index?: number }
 *   | { type: 'remove', page: number, entries: Array<{ index: number, item: Item }> }
 *   | { type: 'move', page: number, ids: string[], dx: number, dy: number }
 *   | { type: 'resize', page: number, id: string, before: Rect, after: Rect, fontBefore?: number, fontAfter?: number }
 *   | { type: 'setAttr', page: number, ids: string[], patch: Record<string, any>, prev: Record<string, Record<string, any>>, next?: Record<string, Record<string, any>> }
 *   | { type: 'correct', page: number, before: Item, after: Item }
 *   | { type: 'clear', page: number, items: Item[] }
 *   | { type: 'setBg', page: number, before: InkBackground, after: InkBackground }
 *   | { type: 'addPage', index: number, page: BoardPage }
 *   | { type: 'replaceDoc', before: { pages: BoardPage[], activePage: number }, after: { pages: BoardPage[], activePage: number } }} InkCommand
 *   replaceDoc = 보관 판서 복원(Ctrl+Shift+R) — 장 전체를 바꾸고, Ctrl+Z 1번이면 복원 전 장으로.
 *   setAttr.next = 항목마다 다른 값(고른 획들의 굵기 ±1 등). 없으면 patch를 모두에.
 *
 * @typedef {{ type: 'exit', reason: 'esc' | 'rightClick' }
 *   | { type: 'copy' }
 *   | { type: 'save' }
 *   | { type: 'restore' }
 *   | { type: 'help', open: boolean }
 *   | { type: 'centerCursor' }
 *   | { type: 'notice', text: string, action?: 'undo' }
 *   | { type: 'suggest', text: string, shape: 'line' | 'circle' | 'ellipse' | 'rect' | 'arrow' }
 *   | { type: 'suggestEnd', accepted: boolean }} InkEffect
 *   suggest = 0.5초 멈춤 도형 제안(draw.shapeApply 'suggest') — 섬에 "원으로 바꿀까요? · Enter", 섬 클릭 = acceptSuggestion()
 *   suggestEnd = 제안이 끝남(수락 또는 버림) — 섬 제안을 걷는다
 *
 * @typedef {Object} PointerSample  화면 CSS 좌표 + 사건 시각(event.timeStamp, ms)
 * @property {number} x
 * @property {number} y
 * @property {number} t
 * @property {number} [button]   0 왼쪽 · 1 가운데 · 2 오른쪽
 * @property {number} [pressure]
 * @property {string} [pointerType] 'mouse' | 'pen' | 'touch'
 * @property {boolean} [shiftKey]
 * @property {boolean} [ctrlKey]
 * @property {boolean} [altKey]
 * @property {boolean} [metaKey]
 *
 * @typedef {import('./drawKeys.js').KeyEventLike & { t?: number }} KeyInput
 *
 * @typedef {Object} EngineSettings  state.settings의 판서 키(05 §1)
 * @property {InkColor} [draw.color]
 * @property {number} [draw.width]
 * @property {'off' | 'light' | 'strong'} [draw.smoothing]
 * @property {boolean} [draw.straighten]
 * @property {boolean} [draw.shapeOnHold]
 * @property {'suggest' | 'auto'} [draw.shapeApply]  멈춤 도형: 제안(기본) / 바로 바꾸기
 * @property {number} [draw.startZoom]
 * @property {'step' | 'exit'} [draw.esc]
 * @property {boolean} [draw.rightClickPan]
 */

export const INK_SCHEMA_VERSION = 1;
export const MAX_UNDO = 200;

/** 판서 펜 색(10 §2 · R2-ANN-1) — palette.js에서 공용 색 표를 읽는다. */
export { INK_COLORS, INK_COLOR_CODES };
export const MARKER_OPACITY = 0.38;
export const DEFAULT_FONT_FAMILY = 'IBM Plex Sans KR';
/** 글자 크기 = 화면 굵기 × 6 (02 §3.7). */
export const TEXT_SIZE_MUL = 6;
export const TEXT_LINE_HEIGHT = 1.3;
export const MIN_WIDTH_PX = 1;
/** 굵기 상한 — 도구별 범위(R2-ANN-2: 지우개 8~64 px가 가장 넓다)를 덮는다. 판서 설정 `draw.width`(1~30)는 따로 가둔다. */
export const MAX_WIDTH_PX = 64;
/** 판서 설정 `draw.width` 범위(Rust 설정 1~30) — 판서 키 조절·설정 읽기는 이 안에서. */
export const DRAW_SETTING_MAX_PX = 30;
/** 지우개 반경(CSS px) = max(8, 굵기 × 1.5). */
export const ERASER_MIN_PX = 8;
export const ERASER_WIDTH_MUL = 1.5;
/** 고르기 판정 여유·핸들 반경(CSS px). */
export const SELECT_TOLERANCE_PX = 6;
export const HANDLE_RADIUS_PX = 8;
/**
 * 휠 한 단계로 볼 누적 deltaY(px). 맥 WebKit은 마우스 휠 한 칸을 40 px로 준다(Windows WebView2는 100 px) —
 * 50이면 맥에서 두 칸째에야 확대됐다(2026-11-26 맥 실측). 사건 하나는 한 단계만 넘기므로 Windows 한 칸도 한 단계.
 */
export const WHEEL_STEP_DELTA = 40;
/** 화살표 머리 길이 = 굵기 × 3.5(최소 6). */
export const ARROW_HEAD_MUL = 3.5;
/** 멈춤 도형 제안이 저절로 사라지는 시간(ms). */
export const SUGGEST_TIMEOUT_MS = 4000;
/** 모자이크(묶음 B) 기본 칸 크기(화면 px, Ctrl+휠로 조절 — 사용자 결정). */
export const DEFAULT_MOSAIC_BLOCK = 10;

/** 도형 이름 + 조사(섬 문구). */
const SHAPE_TO = Object.freeze({
  line: '직선으로',
  circle: '원으로',
  ellipse: '타원으로',
  rect: '사각형으로',
  arrow: '화살표로',
});

/** 섬 문구(04 S12·S14, A §2.6). */
export const INK_NOTICES = Object.freeze({
  horizontal: '수평으로 맞췄어요',
  vertical: '수직으로 맞췄어요',
  line: '직선으로 바꿨어요',
  circle: '원으로 바꿨어요',
  ellipse: '타원으로 바꿨어요',
  rect: '사각형으로 바꿨어요',
  arrow: '화살표로 바꿨어요',
  cleared: '그린 내용을 모두 지웠어요',
  restored: '이전 그리기를 불러왔어요',
  rescaled: '배율이 바뀌어 크기를 맞췄어요',
  differentMonitor: '다른 모니터라 불러올 수 없어요',
});

const EPS = 1e-9;
/** 같은 키를 누르고 있어도 한 번만 처리하는 칸. */
const NO_REPEAT = new Set(['d-clear', 'd-white', 'd-black', 'd-copy', 'd-save', 'd-restore', 'd-center', 'd-help', 'd-exit']);
const MOD_CODES = new Set(['ShiftLeft', 'ShiftRight', 'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'MetaLeft', 'MetaRight']);

// ─────────────── 문서 · 항목 순수 함수 ───────────────

/**
 * 새 판서 문서.
 * @param {Partial<Omit<InkDocument, 'pages' | 'schemaVersion'>> & { pages?: BoardPage[] }} [opts]
 * @returns {InkDocument}
 */
export function createInkDocument(opts = {}) {
  const mode = opts.mode ?? 'draw';
  const docSize = { ...(opts.docSize ?? { w: 1920, h: 1080 }) };
  const scale = opts.scale ?? 1;
  const background = opts.background ?? (mode === 'board' ? 'white' : 'frame');
  return {
    schemaVersion: INK_SCHEMA_VERSION,
    id: opts.id ?? `${mode}-0`,
    mode,
    monitorId: opts.monitorId ?? 0,
    layoutFingerprint: opts.layoutFingerprint ?? '',
    docSize,
    physSize: { ...(opts.physSize ?? { w: Math.round(docSize.w * scale), h: Math.round(docSize.h * scale) }) },
    scale,
    frameId: opts.frameId ?? null,
    background,
    pages: opts.pages ? opts.pages.map((p) => ({ ...p, items: p.items.slice() })) : [{ id: 'page-1', background, items: [] }],
    activePage: opts.activePage ?? 0,
    view: { ...(opts.view ?? IDENTITY_VIEW) },
  };
}

/**
 * DrawSession(Rust `screen-ink://draw`)에서 문서 만들기 옵션.
 * @param {{ session: number, board?: boolean, monitor: { id: number, bounds: Rect, scale: number }, layoutFingerprint?: string }} session
 */
export function documentOptionsFromSession(session) {
  const mode = session.board ? 'board' : 'draw';
  return {
    id: `${mode}-${session.session}`,
    mode: /** @type {InkMode} */ (mode),
    monitorId: session.monitor.id,
    layoutFingerprint: session.layoutFingerprint ?? '',
    docSize: docSizeOf(session.monitor),
    physSize: { w: session.monitor.bounds.w, h: session.monitor.bounds.h },
    scale: session.monitor.scale,
    frameId: session.board ? null : session.session,
  };
}

/**
 * 문서의 모든 항목 수.
 * @param {InkDocument} doc
 */
export function itemCount(doc) {
  return doc.pages.reduce((n, p) => n + p.items.length, 0);
}

/**
 * 항목의 모양 사각형(굵기 없이): 획 = 점 범위, 도형 = 두 끝점, 글자·모자이크 = 상자.
 * @param {Item} item
 * @returns {Rect}
 */
export function itemGeomRect(item) {
  if (item.kind === 'stroke') return pointsBounds(item.points);
  if (item.kind === 'shape') return rectFromPoints(item.x1, item.y1, item.x2, item.y2);
  return { x: item.x, y: item.y, w: item.w, h: item.h };
}

/**
 * 항목 평행 이동.
 * @template {Item} T
 * @param {T} item
 * @param {number} dx
 * @param {number} dy
 * @returns {T}
 */
export function translateItem(item, dx, dy) {
  /** @type {any} */
  const it = item;
  /** @type {any} */
  let out;
  if (it.kind === 'stroke') out = { ...it, points: it.points.map((/** @type {StrokePoint} */ p) => ({ ...p, x: p.x + dx, y: p.y + dy })) };
  else if (it.kind === 'shape') out = { ...it, x1: it.x1 + dx, y1: it.y1 + dy, x2: it.x2 + dx, y2: it.y2 + dy };
  else out = { ...it, x: it.x + dx, y: it.y + dy };
  if (it.bbox) out.bbox = { ...it.bbox, x: it.bbox.x + dx, y: it.bbox.y + dy };
  return out;
}

/**
 * 모양 사각형 from → to로 늘리기(글자는 fontSize 함께).
 * @template {Item} T
 * @param {T} item
 * @param {Rect} from
 * @param {Rect} to
 * @param {number} [fontSize]
 * @returns {T}
 */
export function resizeItem(item, from, to, fontSize) {
  const kx = from.w > EPS ? to.w / from.w : 1;
  const ky = from.h > EPS ? to.h / from.h : 1;
  const mx = (/** @type {number} */ x) => to.x + (x - from.x) * kx;
  const my = (/** @type {number} */ y) => to.y + (y - from.y) * ky;
  /** @type {any} */
  const it = item;
  /** @type {any} */
  let out;
  if (it.kind === 'stroke') out = { ...it, points: it.points.map((/** @type {StrokePoint} */ p) => ({ ...p, x: mx(p.x), y: my(p.y) })) };
  else if (it.kind === 'shape') out = { ...it, x1: mx(it.x1), y1: my(it.y1), x2: mx(it.x2), y2: my(it.y2) };
  else if (it.kind === 'text') out = { ...it, x: to.x, y: to.y, w: to.w, h: to.h, fontSize: fontSize ?? it.fontSize };
  else out = { ...it, x: to.x, y: to.y, w: to.w, h: to.h };
  return withBBox(out);
}

/**
 * 항목 크기 비율 바꾸기(배율이 바뀐 보관 판서 복원, 02 §3.6).
 * @template {Item} T
 * @param {T} item
 * @param {number} k
 * @returns {T}
 */
export function scaleItem(item, k) {
  /** @type {any} */
  const it = item;
  /** @type {any} */
  let out;
  if (it.kind === 'stroke') {
    out = { ...it, width: it.width * k, points: it.points.map((/** @type {StrokePoint} */ p) => ({ ...p, x: p.x * k, y: p.y * k })) };
  } else if (it.kind === 'shape') {
    out = { ...it, width: it.width * k, x1: it.x1 * k, y1: it.y1 * k, x2: it.x2 * k, y2: it.y2 * k };
    if (it.headLen != null) out.headLen = it.headLen * k;
  } else if (it.kind === 'text') {
    out = { ...it, x: it.x * k, y: it.y * k, w: it.w * k, h: it.h * k, fontSize: it.fontSize * k };
  } else {
    out = { ...it, x: it.x * k, y: it.y * k, w: it.w * k, h: it.h * k, blockSize: it.blockSize * k };
  }
  return withBBox(out);
}

/**
 * 문서 전체를 k배(새 문서 크기·배율 정보는 target으로).
 * @param {InkDocument} doc
 * @param {number} k
 * @param {Partial<InkDocument>} [target]
 * @returns {InkDocument}
 */
export function rescaleDocument(doc, k, target = {}) {
  return {
    ...doc,
    ...target,
    pages: doc.pages.map((p) => ({ ...p, items: p.items.map((it) => scaleItem(it, k)) })),
  };
}

/**
 * @param {InkDocument} doc
 * @param {number} index
 * @param {(page: BoardPage) => BoardPage} fn
 * @returns {InkDocument}
 */
function mapPage(doc, index, fn) {
  const pages = doc.pages.slice();
  pages[index] = fn(pages[index]);
  return { ...doc, pages };
}

/**
 * @param {InkDocument} doc
 * @param {number} index
 * @param {(items: Item[]) => Item[]} fn
 */
function mapItems(doc, index, fn) {
  return mapPage(doc, index, (p) => ({ ...p, items: fn(p.items) }));
}

/**
 * @param {Item} item
 * @param {Record<string, any>} patch
 * @returns {Item}
 */
function patchItem(item, patch) {
  return withBBox(/** @type {Item} */ ({ ...item, ...patch }));
}

/**
 * 명령 적용(순수 리듀서) — 새 문서를 돌려준다.
 * @param {InkDocument} doc
 * @param {InkCommand} cmd
 * @returns {InkDocument}
 */
export function applyCommand(doc, cmd) {
  switch (cmd.type) {
    case 'add':
      return mapItems(doc, cmd.page, (items) => {
        const out = items.slice();
        out.splice(cmd.index ?? out.length, 0, cmd.item);
        return out;
      });
    case 'remove': {
      const ids = new Set(cmd.entries.map((e) => e.item.id));
      return mapItems(doc, cmd.page, (items) => items.filter((it) => !ids.has(it.id)));
    }
    case 'move': {
      const ids = new Set(cmd.ids);
      return mapItems(doc, cmd.page, (items) => items.map((it) => (ids.has(it.id) ? translateItem(it, cmd.dx, cmd.dy) : it)));
    }
    case 'resize':
      return mapItems(doc, cmd.page, (items) =>
        items.map((it) => (it.id === cmd.id ? resizeItem(it, cmd.before, cmd.after, cmd.fontAfter) : it)),
      );
    case 'setAttr': {
      const ids = new Set(cmd.ids);
      return mapItems(doc, cmd.page, (items) =>
        items.map((it) => (ids.has(it.id) ? patchItem(it, cmd.next?.[it.id] ?? cmd.patch) : it)),
      );
    }
    case 'correct':
      return mapItems(doc, cmd.page, (items) => items.map((it) => (it.id === cmd.before.id ? cmd.after : it)));
    case 'clear':
      return mapItems(doc, cmd.page, () => []);
    case 'setBg':
      return mapPage(doc, cmd.page, (p) => ({ ...p, background: cmd.after }));
    case 'addPage': {
      const pages = doc.pages.slice();
      pages.splice(cmd.index, 0, { ...cmd.page, items: cmd.page.items.slice() });
      return { ...doc, pages, activePage: cmd.index };
    }
    case 'replaceDoc':
      return { ...doc, pages: cmd.after.pages, activePage: cmd.after.activePage };
    default:
      return doc;
  }
}

/**
 * 명령 되돌리기(순수 리듀서) — cmd를 적용하기 전 모양의 새 문서를 돌려준다.
 * @param {InkDocument} doc
 * @param {InkCommand} cmd
 * @returns {InkDocument}
 */
export function invertCommand(doc, cmd) {
  switch (cmd.type) {
    case 'add':
      return mapItems(doc, cmd.page, (items) => items.filter((it) => it.id !== cmd.item.id));
    case 'remove':
      return mapItems(doc, cmd.page, (items) => {
        const out = items.slice();
        for (const e of cmd.entries.slice().sort((a, b) => a.index - b.index)) out.splice(Math.min(e.index, out.length), 0, e.item);
        return out;
      });
    case 'move': {
      const ids = new Set(cmd.ids);
      return mapItems(doc, cmd.page, (items) => items.map((it) => (ids.has(it.id) ? translateItem(it, -cmd.dx, -cmd.dy) : it)));
    }
    case 'resize':
      return mapItems(doc, cmd.page, (items) =>
        items.map((it) => (it.id === cmd.id ? resizeItem(it, cmd.after, cmd.before, cmd.fontBefore) : it)),
      );
    case 'setAttr': {
      const ids = new Set(cmd.ids);
      return mapItems(doc, cmd.page, (items) => items.map((it) => (ids.has(it.id) ? patchItem(it, cmd.prev[it.id] ?? {}) : it)));
    }
    case 'correct':
      return mapItems(doc, cmd.page, (items) => items.map((it) => (it.id === cmd.after.id ? cmd.before : it)));
    case 'clear':
      return mapItems(doc, cmd.page, (items) => cmd.items.concat(items));
    case 'setBg':
      return mapPage(doc, cmd.page, (p) => ({ ...p, background: cmd.before }));
    case 'addPage': {
      const pages = doc.pages.slice();
      pages.splice(cmd.index, 1);
      const activePage = clamp(doc.activePage >= cmd.index ? doc.activePage - 1 : doc.activePage, 0, pages.length - 1);
      return { ...doc, pages, activePage };
    }
    case 'replaceDoc':
      return { ...doc, pages: cmd.before.pages, activePage: cmd.before.activePage };
    default:
      return doc;
  }
}

/**
 * 측정 도구가 없을 때 글자 상자 크기 어림(한글·전각 1em, 나머지 0.6em).
 * @param {string} text
 * @param {number} fontSize
 */
export function estimateTextSize(text, fontSize) {
  const lines = String(text).split('\n');
  let maxEm = 0;
  for (const line of lines) {
    let em = 0;
    for (const ch of line) em += (ch.codePointAt(0) ?? 0) >= 0x2e80 ? 1 : 0.6;
    if (em > maxEm) maxEm = em;
  }
  return { w: Math.max(fontSize * 0.5, maxEm * fontSize), h: lines.length * fontSize * TEXT_LINE_HEIGHT };
}

/**
 * 제안 도형 윤곽(점선 · 옅게) — 2층 미리보기 전용.
 * @param {ShapeItem} shaped
 * @returns {ShapeItem}
 */
function ghostOf(shaped) {
  return { ...shaped, id: 'ghost', dashed: true, opacity: 0.55 };
}

/**
 * id 'it-17' → 17.
 * @param {string} id
 */
function idNumber(id) {
  const m = /-(\d+)$/.exec(id);
  return m ? Number(m[1]) : 0;
}

// ─────────────── 엔진 ───────────────

/**
 * @typedef {Object} InkEngineOptions
 * @property {InkDocument} [doc]
 * @property {Parameters<typeof createInkDocument>[0]} [docOptions]
 * @property {Size} [viewport]   화면 CSS 크기(기본 = 문서 크기)
 * @property {ViewTransform} [view]  시작 보기(실시간 확대 → 판서 이어받기)
 * @property {'windows' | 'macos'} [platform]
 * @property {ReadonlyArray<{ id: string, combo: string }>} [shortcuts]  state.shortcuts
 * @property {EngineSettings | Record<string, any>} [settings]  state.settings
 * @property {(text: string, fontSize: number, fontFamily: string) => { w: number, h: number }} [measureText]
 */

export class InkEngine {
  /** @param {InkEngineOptions} [opts] */
  constructor(opts = {}) {
    /** @type {InkDocument} */
    this.doc = opts.doc ? createInkDocument(opts.doc) : createInkDocument(opts.docOptions);
    /** @type {Size} */
    this.viewport = { ...(opts.viewport ?? this.doc.docSize) };
    /** @type {ViewTransform} */
    this.view = clampView(opts.view ?? this.doc.view ?? IDENTITY_VIEW, this.viewport, this.doc.docSize);
    /** @type {'windows' | 'macos'} */
    this.platform = opts.platform ?? 'windows';
    this.measureText = opts.measureText ?? ((/** @type {string} */ t, /** @type {number} */ fs) => estimateTextSize(t, fs));
    /** @type {InkTool} */
    this.tool = 'pen';
    /** @type {InkTool} */
    this.prevTool = 'pen';
    /** 도구 막대에서 고른 도형 도구(null = 자유 손그림/수정키 도형). @type {'line' | 'rect' | 'ellipse' | 'arrow' | null} */
    this.shapeTool = null;
    /** @type {InkColor} */
    this.color = 'R';
    this.widthPx = 4;
    /** @type {'ko' | 'en'} */
    this.lang = 'ko';
    this.config = {
      smoothing: /** @type {'off' | 'light' | 'strong'} */ ('light'),
      straighten: true,
      shapeOnHold: true,
      shapeApply: /** @type {'suggest' | 'auto'} */ ('suggest'),
      startZoom: DEFAULT_START_ZOOM,
      esc: /** @type {'step' | 'exit'} */ ('step'),
      rightClickPan: true,
    };
    this.setSettings(opts.settings ?? {}, { initial: true });
    /** @type {Map<string, string>} */
    this.keymap = buildDrawKeymap(opts.shortcuts, this.platform);
    this.shortcuts = opts.shortcuts ?? null;
    this.ime = createImeGuard();
    /** @type {InkCommand[]} */
    this.undoStack = [];
    /** @type {InkCommand[]} */
    this.redoStack = [];
    /** @type {string[]} */
    this.selection = [];
    /** @type {{ id: string | null, x: number, y: number, text: string, color: InkColor, fontSize: number, fontFamily: string } | null} */
    this.editing = null;
    this.helpOpen = false;
    /** @type {{ x: number, y: number } | null} */
    this.cursor = null;
    this.mods = { shift: false, ctrl: false, alt: false, tab: false };
    /** @type {any} */
    this.gesture = null;
    /** @type {Set<string>} */
    this.hidden = new Set();
    /**
     * 멈춤 도형 제안(제안형) — 원래 손획 id·바꿀 도형·사라질 시각.
     * @type {{ strokeId: string, page: number, shaped: ShapeItem, name: 'line' | 'circle' | 'ellipse' | 'rect' | 'arrow', text: string, expiresAt: number } | null}
     */
    this.suggestion = null;
    /** 안쪽에서 생긴 알림(제안 버림 등) — 다음 입력 결과에 붙여 내보낸다. @type {InkEffect[]} */
    this.outbox = [];
    /** 화면이 다시 그릴 곳을 알 수 있게 바뀔 때마다 올린다. */
    this.revs = { doc: 0, view: 0, active: 0, hidden: 0 };
    /** @type {{ kind: 'append', page: number, item: Item } | { kind: 'full' }} */
    this.lastChange = { kind: 'full' };
    this.wheelAcc = 0;
    /** @type {number | null} */
    this.lastTick = null;
    this.seq = 0;
    this.pageSeq = this.doc.pages.length;
    this.syncIdCounters();
    /** @type {Set<(engine: InkEngine) => void>} */
    this.listeners = new Set();
  }

  // ── 설정 · 구독 ──

  /**
   * state.settings 반영. 처음 한 번은 펜 색·굵기도 설정값으로.
   * @param {Record<string, any>} settings
   * @param {{ initial?: boolean }} [opts]
   */
  setSettings(settings, opts = {}) {
    const s = settings ?? {};
    let langChanged = false;
    if ((s['general.lang'] === 'ko' || s['general.lang'] === 'en') && s['general.lang'] !== this.lang) {
      this.lang = s['general.lang'];
      langChanged = true;
    }
    if (s['draw.smoothing'] === 'off' || s['draw.smoothing'] === 'light' || s['draw.smoothing'] === 'strong') {
      this.config.smoothing = s['draw.smoothing'];
    }
    if (typeof s['draw.straighten'] === 'boolean') this.config.straighten = s['draw.straighten'];
    if (typeof s['draw.shapeOnHold'] === 'boolean') this.config.shapeOnHold = s['draw.shapeOnHold'];
    if (s['draw.shapeApply'] === 'suggest' || s['draw.shapeApply'] === 'auto') this.config.shapeApply = s['draw.shapeApply'];
    if (typeof s['draw.startZoom'] === 'number') this.config.startZoom = s['draw.startZoom'];
    if (s['draw.esc'] === 'step' || s['draw.esc'] === 'exit') this.config.esc = s['draw.esc'];
    if (typeof s['draw.rightClickPan'] === 'boolean') this.config.rightClickPan = s['draw.rightClickPan'];
    if (opts.initial) {
      if (INK_COLOR_CODES.includes(s['draw.color'])) this.color = s['draw.color'];
      if (typeof s['draw.width'] === 'number') this.widthPx = clamp(Math.round(s['draw.width']), MIN_WIDTH_PX, DRAW_SETTING_MAX_PX);
    }
    if (!opts.initial && langChanged && this.listeners?.size) this.notify();
  }

  /**
   * 설정 단축키 표가 바뀌면 판정표를 다시 만든다.
   * @param {ReadonlyArray<{ id: string, combo: string }> | null} shortcuts
   */
  setShortcuts(shortcuts) {
    this.shortcuts = shortcuts;
    this.keymap = buildDrawKeymap(shortcuts, this.platform);
    if (this.listeners?.size) this.notify();
  }

  /**
   * @param {(engine: InkEngine) => void} fn
   * @returns {() => void}
   */
  subscribe(fn) {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  notify() {
    for (const fn of this.listeners) fn(this);
  }

  // ── 읽기 ──

  get page() {
    return this.doc.pages[this.doc.activePage];
  }

  get items() {
    return this.page.items;
  }

  get background() {
    return this.page.background;
  }

  get canUndo() {
    return this.undoStack.length > 0;
  }

  get canRedo() {
    return this.redoStack.length > 0;
  }

  /** 지금 배율에서 새 획의 문서 굵기. */
  get docWidth() {
    return screenWidthToDoc(this.widthPx, this.view.s);
  }

  /** 지우개 반경(문서 px). */
  get eraserRadius() {
    return Math.max(ERASER_MIN_PX, this.widthPx * ERASER_WIDTH_MUL) / this.view.s;
  }

  /** 실행 취소 키 표시(섬 문구 "되돌리기 키로 원래 획"). */
  get undoLabel() {
    const row = this.shortcuts?.find((s) => s.id === 'd-undo');
    const fallback = defaultDrawCombo('d-undo', this.platform);
    const combo = row ? normalizeCombo(row.combo, this.platform) : fallback;
    return combo || fallback;
  }

  /** 섬(drawIsland)에 줄 한 줄 상태. */
  get islandPatch() {
    return {
      color: this.color,
      tool: this.tool,
      widthPx: this.widthPx,
      zoom: this.view.s,
      background: this.background,
      board: this.doc.mode === 'board',
      pageIndex: this.doc.activePage,
      pageCount: this.doc.pages.length,
      suggestion: this.suggestion ? this.suggestion.text : null,
    };
  }

  /** 화면이 쓰는 읽기 전용 모양(구독자에게). */
  getState() {
    return {
      doc: this.doc,
      items: this.items,
      background: this.background,
      view: this.view,
      viewport: this.viewport,
      tool: this.tool,
      shapeTool: this.shapeTool,
      color: this.color,
      widthPx: this.widthPx,
      lang: this.lang,
      docWidth: this.docWidth,
      eraserRadius: this.eraserRadius,
      helpOpen: this.helpOpen,
      selection: this.selection.slice(),
      selectionBox: this.selectionBox(),
      editing: this.editing ? { ...this.editing } : null,
      panning: this.tool === 'pan',
      canUndo: this.canUndo,
      canRedo: this.canRedo,
      undoDepth: this.undoStack.length,
      redoDepth: this.redoStack.length,
      pageIndex: this.doc.activePage,
      pageCount: this.doc.pages.length,
      suggestion: this.suggestion ? { text: this.suggestion.text, shape: this.suggestion.name } : null,
      revs: { ...this.revs },
    };
  }

  /**
   * 지금 그리는 중인 것(2층): 미리보기 항목들 + 끌어 고르기 사각형 + 제안 도형 윤곽(점선·옅게). 문서 좌표.
   * ghosts는 items 안에도 들어 있다(그리기 순서 = items).
   */
  getPreview() {
    /** @type {Item[]} */
    const items = [];
    /** @type {Item[]} */
    const ghosts = [];
    /** @type {Rect | null} */
    let marquee = null;
    const g = this.gesture;
    if (g?.kind === 'draw') {
      const it = this.previewDrawItem();
      if (it) items.push(it);
      if (g.held && this.config.shapeApply === 'suggest' && !this.activeShapeKind()) {
        ghosts.push(ghostOf(this.makeShape(g, g.held, 'ghost', this.mods.alt)));
      }
    } else if (g?.kind === 'move' && g.moved) {
      for (const it of this.items) if (g.ids.includes(it.id)) items.push(translateItem(it, g.dx, g.dy));
    } else if (g?.kind === 'resize' && g.after) {
      const it = this.items.find((i) => i.id === g.id);
      if (it) items.push(resizeItem(it, g.before, g.after, g.fontAfter));
    } else if (g?.kind === 'marquee') {
      marquee = rectFromPoints(g.start.x, g.start.y, g.end.x, g.end.y);
    }
    if (this.suggestion && this.suggestion.page === this.doc.activePage) ghosts.push(ghostOf(this.suggestion.shaped));
    items.push(...ghosts);
    return { items, ghosts, marquee };
  }

  /** 고른 항목을 감싸는 상자(문서 px, 끌어 옮기는 중이면 옮긴 자리). */
  selectionBox() {
    if (!this.selection.length) return null;
    const g = this.gesture;
    /** @type {Array<Rect | null>} */
    const boxes = [];
    for (const it of this.items) {
      if (!this.selection.includes(it.id)) continue;
      if (g?.kind === 'move' && g.moved) boxes.push(translateItem(it, g.dx, g.dy).bbox ?? null);
      else if (g?.kind === 'resize' && g.after && g.id === it.id) boxes.push(resizeItem(it, g.before, g.after, g.fontAfter).bbox ?? null);
      else boxes.push(it.bbox ?? null);
    }
    return unionRect(boxes);
  }

  /** 문서가 비었는지(보관 판단). */
  hasItems() {
    return itemCount(this.doc) > 0;
  }

  /** 보관·내보내기용 문서 JSON(보기 포함). */
  exportDocument() {
    return /** @type {InkDocument} */ (JSON.parse(JSON.stringify({ ...this.doc, view: this.view })));
  }

  /** rAF로 tick을 계속 불러야 하는지(멈춤 판정·가장자리 밀기). */
  needsTick() {
    const g = this.gesture;
    if (g?.kind === 'draw' && !g.held && this.freehandHoldEligible(g)) return true;
    if (this.suggestion) return true; // 4초 뒤 저절로 버림
    return this.edgePushActive();
  }

  /** 안쪽 알림을 꺼내 비운다(화면이 직접 부른 setTool 등 뒤에 써도 된다). @returns {InkEffect[]} */
  takeEffects() {
    if (!this.outbox.length) return [];
    const out = this.outbox;
    this.outbox = [];
    return out;
  }

  // ── 멈춤 도형 제안(draw.shapeApply 'suggest') ──

  /** 제안 버림(문서 변화 없음). */
  dropSuggestion() {
    if (!this.suggestion) return false;
    this.suggestion = null;
    this.revs.active++;
    this.outbox.push({ type: 'suggestEnd', accepted: false });
    return true;
  }

  /** 현재 언어의 안내 문구(한국어는 INK_NOTICES 그대로 유지). @param {keyof typeof INK_NOTICES} key */
  noticeOf(key) {
    return this.lang === 'en' ? translate('en', `draw.notice.${key}`) : INK_NOTICES[key];
  }

  /** @param {'line' | 'circle' | 'ellipse' | 'rect' | 'arrow'} shape */
  suggestOf(shape) {
    return this.lang === 'en' ? translate('en', `draw.suggest.${shape}`) : `${SHAPE_TO[shape]} 바꿀까요? · Enter`;
  }

  /** @param {'line' | 'circle' | 'ellipse' | 'rect' | 'arrow'} name */
  undoNoticeOf(name) {
    const notice = this.noticeOf(name);
    if (this.lang === 'en') return translate('en', 'draw.notice.undoOriginal', { notice, undo: this.undoLabel });
    return `${notice} · ${this.undoLabel}로 원래 획`;
  }

  /**
   * 제안 수락(Enter·섬 클릭) → correct(원래 손획, 도형) 명령. 그 뒤 Ctrl+Z 1번 = 원래 손획.
   * @returns {InkEffect[]}
   */
  acceptSuggestion() {
    const sg = this.suggestion;
    if (!sg) return this.takeEffects();
    this.suggestion = null;
    this.revs.active++;
    const before = this.doc.pages[sg.page]?.items.find((i) => i.id === sg.strokeId);
    if (!before) {
      this.outbox.push({ type: 'suggestEnd', accepted: false });
      this.notify();
      return this.takeEffects();
    }
    this.push({ type: 'correct', page: sg.page, before, after: sg.shaped });
    this.outbox.push(
      { type: 'suggestEnd', accepted: true },
      { type: 'notice', text: this.undoNoticeOf(sg.name) },
    );
    this.notify();
    return this.takeEffects();
  }

  // ── 문서 변경(명령) ──

  /**
   * @param {InkCommand} cmd
   */
  push(cmd) {
    this.dropSuggestion();
    const before = this.doc;
    this.doc = applyCommand(this.doc, cmd);
    this.undoStack.push(cmd);
    if (this.undoStack.length > MAX_UNDO) this.undoStack.shift();
    this.redoStack = [];
    const appended = cmd.type === 'add' && cmd.page === before.activePage && (cmd.index == null || cmd.index >= before.pages[cmd.page].items.length);
    this.markDoc(appended ? { kind: 'append', page: cmd.page, item: cmd.item } : { kind: 'full' });
  }

  /**
   * @param {{ kind: 'append', page: number, item: Item } | { kind: 'full' }} change
   */
  markDoc(change) {
    this.revs.doc++;
    this.lastChange = change;
  }

  undo() {
    this.dropSuggestion();
    const cmd = this.undoStack.pop();
    if (!cmd) return false;
    this.doc = invertCommand(this.doc, cmd);
    this.redoStack.push(cmd);
    this.afterHistory(cmd);
    return true;
  }

  redo() {
    this.dropSuggestion();
    const cmd = this.redoStack.pop();
    if (!cmd) return false;
    this.doc = applyCommand(this.doc, cmd);
    this.undoStack.push(cmd);
    if (this.undoStack.length > MAX_UNDO) this.undoStack.shift();
    this.afterHistory(cmd);
    return true;
  }

  /** @param {InkCommand} cmd */
  afterHistory(cmd) {
    if (cmd.type !== 'addPage' && cmd.type !== 'replaceDoc' && cmd.page !== this.doc.activePage && cmd.page < this.doc.pages.length) {
      this.doc = { ...this.doc, activePage: cmd.page };
    }
    const ids = new Set(this.items.map((i) => i.id));
    this.selection = this.selection.filter((id) => ids.has(id));
    this.markDoc({ kind: 'full' });
  }

  /**
   * 고른 항목 속성 바꾸기(색 키·Ctrl+휠). 바뀌는 항목이 없으면 명령을 쌓지 않는다.
   * @param {(item: Item) => Record<string, any> | null} fn  항목마다 바꿀 값(null = 건너뜀)
   */
  setAttrSelected(fn) {
    /** @type {Record<string, Record<string, any>>} */
    const prev = {};
    /** @type {Record<string, Record<string, any>>} */
    const next = {};
    /** @type {string[]} */
    const ids = [];
    for (const it of this.items) {
      if (!this.selection.includes(it.id)) continue;
      const patch = fn(it);
      if (!patch) continue;
      /** @type {Record<string, any>} */
      const before = {};
      for (const k of Object.keys(patch)) before[k] = /** @type {any} */ (it)[k];
      ids.push(it.id);
      prev[it.id] = before;
      next[it.id] = patch;
    }
    if (!ids.length) return false;
    const uniform = ids.every((id) => JSON.stringify(next[id]) === JSON.stringify(next[ids[0]]));
    this.push({ type: 'setAttr', page: this.doc.activePage, ids, patch: next[ids[0]], prev, ...(uniform ? {} : { next }) });
    return true;
  }

  nextId() {
    this.seq += 1;
    return `it-${this.seq}`;
  }

  syncIdCounters() {
    let max = 0;
    for (const p of this.doc.pages) for (const it of p.items) max = Math.max(max, idNumber(it.id));
    this.seq = Math.max(this.seq, max);
    this.pageSeq = Math.max(this.pageSeq, ...this.doc.pages.map((p) => idNumber(p.id)));
  }

  // ── 보기 ──

  /**
   * @param {ViewTransform} next
   */
  setView(next) {
    const v = clampView(next, this.viewport, this.doc.docSize);
    if (sameView(v, this.view)) return false;
    this.view = v;
    this.revs.view++;
    if (v.s <= 1 + EPS && this.tool === 'pan') this.tool = this.prevTool;
    return true;
  }

  /**
   * 화면 크기가 바뀌면 보기를 다시 가둔다.
   * @param {Size} size
   */
  setViewport(size) {
    if (size.w === this.viewport.w && size.h === this.viewport.h) return;
    this.viewport = { w: size.w, h: size.h };
    this.view = clampView(this.view, this.viewport, this.doc.docSize);
    this.revs.view++;
    this.notify();
  }

  /**
   * 확대 단계 한 칸(커서 고정점).
   * @param {1 | -1} dir
   * @param {{ x: number, y: number } | null} [at]
   */
  zoomStep(dir, at) {
    const next = nextZoomStep(this.view.s, dir, this.config.startZoom);
    if (Math.abs(next - this.view.s) < EPS) return false;
    const c = at ?? this.cursor ?? { x: this.viewport.w / 2, y: this.viewport.h / 2 };
    return this.setView(zoomAt(this.view, next, c, this.viewport, this.doc.docSize));
  }

  /** @param {number} dir */
  changeWidth(dir) {
    if (this.selection.length) {
      const k = 1 / this.view.s;
      return this.setAttrSelected((it) => {
        if (it.kind === 'stroke' || it.kind === 'shape') {
          const width = Math.max(0.25, it.width + dir * k);
          if (Math.abs(width - it.width) < EPS) return null;
          /** @type {Record<string, any>} */
          const patch = { width };
          if (it.kind === 'shape' && it.shape === 'arrow') patch.headLen = Math.max(6 * k, width * ARROW_HEAD_MUL);
          return patch;
        }
        if (it.kind === 'text') {
          const fontSize = Math.max(4 * k, it.fontSize + dir * TEXT_SIZE_MUL * k);
          if (Math.abs(fontSize - it.fontSize) < EPS) return null;
          const sz = this.measureText(it.text, fontSize, it.fontFamily);
          return { fontSize, w: sz.w, h: sz.h };
        }
        return null;
      });
    }
    const w = clamp(this.widthPx + dir, MIN_WIDTH_PX, DRAW_SETTING_MAX_PX);
    if (w === this.widthPx) return false;
    this.widthPx = w;
    this.revs.active++;
    return true;
  }

  edgePushActive() {
    if (this.view.s <= 1 + EPS || !this.cursor || this.editing) return false;
    const d = edgePushDelta(this.cursor, this.viewport, 16);
    return d.dx !== 0 || d.dy !== 0;
  }

  // ── 도구 · 색 · 바탕 ──

  /** @param {InkTool} tool */
  setTool(tool) {
    if (tool !== 'pan') {
      this.dropSuggestion();
      this.shapeTool = null;
    }
    if (this.editing) this.commitText();
    if (tool !== 'select' && tool !== 'pan' && this.selection.length) this.selection = [];
    if (tool === 'pan') {
      if (this.tool !== 'pan') this.prevTool = this.tool;
    } else {
      this.prevTool = tool;
    }
    this.tool = tool;
    this.revs.active++;
  }

  /**
   * 도구 막대 도형 도구('line' · 'rect' · 'ellipse' · 'arrow') 고르기(null이면 일반 펜).
   * @param {'line' | 'rect' | 'ellipse' | 'arrow' | null} shape
   */
  setShapeTool(shape) {
    this.setTool('pen');
    this.shapeTool = shape;
    this.revs.active++;
  }

  /** 지금 획에 적용할 도형 종류(수정 키가 도구 막대 선택보다 우선). */
  activeShapeKind() {
    return this.shapeKindFromMods() ?? (this.tool === 'pen' ? this.shapeTool : null);
  }

  /**
   * 굵기 직접 설정(1~MAX_WIDTH_PX 64 — 패널 도구별 범위 지원).
   * 판서 설정 `draw.width` 저장은 바깥(DrawOverlay)에서 DRAW_SETTING_MAX_PX(30)로 가둔다.
   * @param {number} px
   */
  setWidth(px) {
    if (!Number.isFinite(px)) return false;
    const w = clamp(px, MIN_WIDTH_PX, MAX_WIDTH_PX);
    if (Math.abs(w - this.widthPx) < EPS) return false;
    this.widthPx = w;
    if (this.editing) {
      this.editing = { ...this.editing, fontSize: (w * TEXT_SIZE_MUL) / this.view.s };
    }
    this.revs.active++;
    return true;
  }

  /** @param {InkColor} color */
  setColor(color) {
    this.color = color;
    if (this.editing) this.editing = { ...this.editing, color };
    if (this.selection.length) this.setAttrSelected((it) => (it.kind !== 'mosaic' && it.color !== color ? { color } : null));
    this.revs.active++;
  }

  /**
   * 바탕 직접 전환('frame' · 'white' · 'black', 도구 막대 칠판 ↔ 화면 전환).
   * @param {InkBackground} target
   */
  setBackground(target) {
    if (target !== 'frame' && target !== 'white' && target !== 'black') return false;
    if (this.doc.mode === 'board' && target === 'frame') return false;
    const cur = this.background;
    if (target === cur) return false;
    this.push({ type: 'setBg', page: this.doc.activePage, before: cur, after: target });
    this.notify();
    return true;
  }

  /**
   * W/K: 판서 중엔 흰·검은 칠판 ↔ 원래 화면 토글, 칠판(Alt+3)에선 흰 ↔ 검은.
   * @param {'white' | 'black'} target
   */
  toggleBackground(target) {
    const cur = this.background;
    /** @type {InkBackground} */
    let next;
    if (this.doc.mode === 'board') next = target;
    else next = cur === target ? this.doc.background : target;
    if (next === cur) return false;
    this.push({ type: 'setBg', page: this.doc.activePage, before: cur, after: next });
    return true;
  }

  /** 칠판 새 장(R-BOARD-2 · #18). */
  addPage() {
    if (this.editing) this.commitText();
    this.pageSeq += 1;
    const index = this.doc.activePage + 1;
    const page = { id: `page-${this.pageSeq}`, background: this.background, items: [] };
    this.selection = [];
    if (this.hidden.size) {
      this.hidden = new Set();
      this.revs.hidden++;
    }
    this.push({ type: 'addPage', index, page });
  }

  /** @param {number} index */
  gotoPage(index) {
    if (index < 0 || index >= this.doc.pages.length || index === this.doc.activePage) return false;
    if (this.editing) this.commitText();
    this.dropSuggestion();
    this.doc = { ...this.doc, activePage: index };
    this.selection = [];
    if (this.hidden.size) {
      this.hidden = new Set();
      this.revs.hidden++;
    }
    this.markDoc({ kind: 'full' });
    return true;
  }

  /** 칠판 다음 쪽(마지막 쪽이면 빈 칠판 새 쪽). */
  nextBoardPage() {
    if (this.doc.mode !== 'board') return false;
    if (this.doc.activePage < this.doc.pages.length - 1) {
      return this.gotoPage(this.doc.activePage + 1);
    }
    this.addPage();
    return true;
  }

  /** 칠판 이전 쪽. */
  prevBoardPage() {
    if (this.doc.mode !== 'board') return false;
    return this.gotoPage(this.doc.activePage - 1);
  }

  // ── 글자(T) ──

  /**
   * @param {number} dx
   * @param {number} dy
   */
  openText(dx, dy) {
    this.editing = {
      id: null,
      x: dx,
      y: dy,
      text: '',
      color: this.color,
      fontSize: (this.widthPx * TEXT_SIZE_MUL) / this.view.s,
      fontFamily: DEFAULT_FONT_FAMILY,
    };
    this.revs.active++;
  }

  /**
   * 고르기 뒤 더블클릭 — 글자 다시 고치기.
   * @param {string} id
   */
  editTextItem(id) {
    const it = this.items.find((i) => i.id === id);
    if (!it || it.kind !== 'text') return false;
    this.editing = { id, x: it.x, y: it.y, text: it.text, color: it.color, fontSize: it.fontSize, fontFamily: it.fontFamily };
    this.hidden = new Set([id]);
    this.revs.hidden++;
    this.revs.active++;
    return true;
  }

  /** @param {string} value */
  textInput(value) {
    if (!this.editing) return;
    this.editing = { ...this.editing, text: value };
    this.revs.active++;
    this.notify();
  }

  /** 글자 입력 끝(Esc·바깥 클릭). 빈 글자는 만들지 않는다. */
  commitText() {
    const e = this.editing;
    if (!e) return false;
    this.editing = null;
    if (this.hidden.size) {
      this.hidden = new Set();
      this.revs.hidden++;
    }
    this.revs.active++;
    const blank = e.text.trim() === '';
    const page = this.doc.activePage;
    if (e.id) {
      const index = this.items.findIndex((i) => i.id === e.id);
      const it = this.items[index];
      if (!it || it.kind !== 'text') return false;
      if (blank) {
        this.push({ type: 'remove', page, entries: [{ index, item: it }] });
        return true;
      }
      const prevFamily = it.fontFamily;
      const nextFamily = e.fontFamily || prevFamily;
      const prevWeight = /** @type {any} */ (it).fontWeight;
      const rawNextWeight = /** @type {any} */ (e).fontWeight;
      const nextWeight = rawNextWeight !== undefined ? rawNextWeight : prevWeight;
      if (it.text === e.text && it.color === e.color && prevFamily === nextFamily && prevWeight === nextWeight) {
        return false;
      }
      const sz = this.measureText(e.text, it.fontSize, nextFamily);
      /** @type {Record<string, any>} */
      const patch = { text: e.text, color: e.color, fontFamily: nextFamily, w: sz.w, h: sz.h };
      /** @type {Record<string, any>} */
      const prev = { text: it.text, color: it.color, fontFamily: prevFamily, w: it.w, h: it.h };
      if (nextWeight !== undefined || prevWeight !== undefined) {
        patch.fontWeight = nextWeight;
        prev.fontWeight = prevWeight;
      }
      this.push({
        type: 'setAttr',
        page,
        ids: [it.id],
        patch,
        prev: { [it.id]: prev },
      });
      return true;
    }
    if (blank) return false;
    const sz = this.measureText(e.text, e.fontSize, e.fontFamily);
    const fontWeight = /** @type {any} */ (e).fontWeight;
    /** @type {TextItem} */
    const item = withBBox({
      id: this.nextId(),
      kind: 'text',
      color: e.color,
      fontFamily: e.fontFamily,
      ...(fontWeight != null ? { fontWeight } : {}),
      fontSize: e.fontSize,
      x: e.x,
      y: e.y,
      w: sz.w,
      h: sz.h,
      text: e.text,
    });
    this.push({ type: 'add', page, item });
    return true;
  }

  compositionStart() {
    this.ime.start();
  }

  /** @param {number} t */
  compositionEnd(t) {
    this.ime.end(t);
  }

  // ── 포인터 ──

  /**
   * @param {PointerSample} e
   */
  updateMods(e) {
    const primary = this.platform === 'macos' ? Boolean(e.metaKey) : Boolean(e.ctrlKey);
    const changed = this.mods.shift !== Boolean(e.shiftKey) || this.mods.ctrl !== primary || this.mods.alt !== Boolean(e.altKey);
    this.mods.shift = Boolean(e.shiftKey);
    this.mods.ctrl = primary;
    this.mods.alt = Boolean(e.altKey);
    return changed;
  }

  /** 지금 수정 키로 정한 임시 도형(없으면 손그림). */
  shapeKindFromMods() {
    const { shift, ctrl, tab } = this.mods;
    if (tab) return 'ellipse';
    if (ctrl && shift) return 'arrow';
    if (ctrl) return 'rect';
    if (shift) return 'line';
    return null;
  }

  /**
   * @param {PointerSample} e
   */
  docPoint(e) {
    const d = viewToDoc(e.x, e.y, this.view);
    return { x: d.dx, y: d.dy };
  }

  /**
   * @param {PointerSample} e
   */
  pressureOf(e) {
    if (e.pointerType === 'pen' && typeof e.pressure === 'number' && e.pressure > 0) return clamp(e.pressure, 0, 1);
    return 0.5;
  }

  /**
   * @param {PointerSample} e
   * @returns {InkEffect[]}
   */
  pointerDown(e) {
    this.cursor = { x: e.x, y: e.y };
    this.updateMods(e);
    /** @type {InkEffect[]} */
    const fx = [];
    const button = e.button ?? 0;
    if (this.gesture) {
      if (button !== 0) return fx;
      fx.push(...this.finishGesture(e)); // 놓친 pointerup
    }
    if (button === 2) {
      fx.push(...this.rightClick());
      this.notify();
      return [...fx, ...this.takeEffects()];
    }
    if (button !== 0) return fx;
    this.dropSuggestion(); // 새 획 시작 = 제안 버림
    if (this.editing) {
      this.commitText();
      this.gesture = { kind: 'swallow' };
      this.notify();
      return [...fx, ...this.takeEffects()];
    }
    if (this.tool === 'pan') {
      this.tool = this.prevTool === 'pan' ? 'pen' : this.prevTool;
      this.gesture = { kind: 'swallow' };
      this.revs.active++;
      this.notify();
      return [...fx, ...this.takeEffects()];
    }
    const d = this.docPoint(e);
    if (this.tool === 'pen' || this.tool === 'marker') this.startDraw(e, d);
    else if (this.tool === 'eraser') this.startErase(d);
    else if (this.tool === 'select') this.startSelect(e, d);
    else if (this.tool === 'text') {
      this.openText(d.x, d.y);
      this.gesture = { kind: 'swallow' };
    }
    this.notify();
    return [...fx, ...this.takeEffects()];
  }

  /** @returns {InkEffect[]} */
  rightClick() {
    if (this.editing) this.commitText();
    if (this.view.s > 1 + EPS && this.config.rightClickPan) {
      if (this.tool !== 'pan') this.setTool('pan');
      return [];
    }
    return [{ type: 'exit', reason: 'rightClick' }];
  }

  /**
   * @param {PointerSample} e
   * @returns {InkEffect[]}
   */
  pointerMove(e) {
    const prev = this.cursor;
    this.cursor = { x: e.x, y: e.y };
    const modsChanged = this.updateMods(e);
    if (this.tool === 'pan' && prev) {
      if (this.setView(panBy(this.view, e.x - prev.x, e.y - prev.y, this.viewport, this.doc.docSize))) this.notify();
      return [];
    }
    const g = this.gesture;
    if (!g) return [];
    const d = this.docPoint(e);
    if (g.kind === 'draw') this.addDrawSample(e, d);
    else if (g.kind === 'erase') this.eraseTo(d);
    else if (g.kind === 'move') {
      g.dx = d.x - g.start.x;
      g.dy = d.y - g.start.y;
      if (!g.moved && Math.hypot(g.dx, g.dy) * this.view.s >= 1) {
        g.moved = true;
        this.hidden = new Set(g.ids);
        this.revs.hidden++;
      }
      this.revs.active++;
    } else if (g.kind === 'resize') this.updateResize(d);
    else if (g.kind === 'marquee') {
      g.end = d;
      this.revs.active++;
    } else if (modsChanged) this.revs.active++;
    this.notify();
    return [];
  }

  /**
   * @param {PointerSample} e
   * @returns {InkEffect[]}
   */
  pointerUp(e) {
    this.cursor = { x: e.x, y: e.y };
    this.updateMods(e);
    if (!this.gesture) return this.takeEffects();
    const fx = this.finishGesture(e);
    this.notify();
    return [...this.takeEffects(), ...fx];
  }

  /** 포인터가 창을 떠남 — 가장자리 밀기 멈춤. */
  pointerLeave() {
    this.cursor = null;
  }

  /** 포인터 취소 — 그리던 것을 버린다. */
  pointerCancel() {
    this.cancelGesture();
    this.notify();
    return this.takeEffects();
  }

  /**
   * 더블클릭: 고르기 도구에서 글자 위면 다시 고치기.
   * @param {PointerSample} e
   */
  doubleClick(e) {
    if (this.tool !== 'select' || this.editing) return [];
    const d = this.docPoint(e);
    const idx = topItemAt(this.items, d.x, d.y, SELECT_TOLERANCE_PX / this.view.s);
    const it = idx >= 0 ? this.items[idx] : null;
    if (it?.kind === 'text') {
      this.selection = [it.id];
      this.editTextItem(it.id);
      this.notify();
    }
    return [];
  }

  /**
   * @param {PointerSample} e
   * @returns {InkEffect[]}
   */
  finishGesture(e) {
    const g = this.gesture;
    /** @type {InkEffect[]} */
    let fx = [];
    if (g?.kind === 'draw') fx = this.finishDraw(e);
    else if (g?.kind === 'erase') this.finishErase();
    else if (g?.kind === 'move') this.finishMove();
    else if (g?.kind === 'resize') this.finishResize();
    else if (g?.kind === 'marquee') this.finishMarquee();
    this.gesture = null;
    this.revs.active++;
    return fx;
  }

  cancelGesture() {
    if (!this.gesture) return false;
    this.gesture = null;
    if (this.hidden.size && !this.editing) {
      this.hidden = new Set();
      this.revs.hidden++;
    }
    this.revs.active++;
    return true;
  }

  // ── 그리기 ──

  /**
   * @param {PointerSample} e
   * @param {{ x: number, y: number }} d
   */
  startDraw(e, d) {
    const smoother = createPointSmoother(this.config.smoothing);
    const p = this.pressureOf(e);
    const pt = { x: d.x, y: d.y, p, t: 0 };
    const sm = smoother.push({ ...pt, t: e.t });
    this.gesture = {
      kind: 'draw',
      tool: this.tool,
      color: this.color,
      width: this.docWidth,
      t0: e.t,
      start: { x: d.x, y: d.y },
      end: { x: d.x, y: d.y },
      raw: [pt],
      smooth: [{ ...sm, t: 0 }],
      smoother,
      hold: { x: e.x, y: e.y, t: e.t },
      /** @type {import('./shapeFit.js').FittedShape | null} */
      held: null,
    };
    this.revs.active++;
  }

  /**
   * @param {any} g
   */
  freehandHoldEligible(g) {
    return g.tool === 'pen' && this.config.shapeOnHold && !this.activeShapeKind();
  }

  /**
   * @param {PointerSample} e
   * @param {{ x: number, y: number }} d
   */
  addDrawSample(e, d) {
    const g = this.gesture;
    const last = g.raw[g.raw.length - 1];
    g.end = { x: d.x, y: d.y };
    if (Math.hypot(d.x - last.x, d.y - last.y) > 1e-6) {
      const pt = { x: d.x, y: d.y, p: this.pressureOf(e), t: Math.max(0, e.t - g.t0) };
      g.raw.push(pt);
      const sm = g.smoother.push({ ...pt, t: e.t });
      g.smooth.push({ ...sm, t: pt.t });
    }
    if (Math.hypot(e.x - g.hold.x, e.y - g.hold.y) > HOLD_MOVE_PX) {
      g.hold = { x: e.x, y: e.y, t: e.t };
      g.held = null;
    } else {
      this.checkHold(e.t);
    }
    this.revs.active++;
  }

  /** @param {number} t */
  checkHold(t) {
    const g = this.gesture;
    if (g?.kind !== 'draw' || g.held || !this.freehandHoldEligible(g)) return false;
    if (t - g.hold.t < HOLD_MS || pathLength(g.raw) < HOLD_MIN_PATH) return false;
    const fitted = fitShape(g.smooth, { straighten: this.config.straighten });
    if (!fitted) {
      g.hold = { ...g.hold, t }; // 실패하면 다음 0.5초 멈춤까지 다시 보지 않는다
      return false;
    }
    g.held = fitted;
    this.revs.active++;
    return true;
  }

  /**
   * @param {any} g
   * @param {{ shape: 'line' | 'rect' | 'ellipse' | 'arrow', x1: number, y1: number, x2: number, y2: number }} geom
   * @param {string} id
   * @param {boolean} dashed
   * @returns {ShapeItem}
   */
  makeShape(g, geom, id, dashed) {
    return withBBox({
      id,
      kind: 'shape',
      shape: geom.shape,
      color: g.color,
      width: g.width,
      opacity: g.tool === 'marker' ? MARKER_OPACITY : 1,
      dashed,
      x1: geom.x1,
      y1: geom.y1,
      x2: geom.x2,
      y2: geom.y2,
      ...(geom.shape === 'arrow' ? { headLen: Math.max(6 / this.view.s, g.width * ARROW_HEAD_MUL) } : {}),
    });
  }

  /**
   * @param {any} g
   * @param {StrokePoint[]} points
   * @param {string} id
   * @param {boolean} dashed
   * @returns {StrokeItem}
   */
  makeStroke(g, points, id, dashed) {
    return withBBox({
      id,
      kind: 'stroke',
      tool: g.tool,
      color: g.color,
      width: g.width,
      opacity: g.tool === 'marker' ? MARKER_OPACITY : 1,
      dashed,
      points,
    });
  }

  /** 지금 획의 미리보기 항목. */
  previewDrawItem() {
    const g = this.gesture;
    if (g?.kind !== 'draw') return null;
    const dashed = this.mods.alt;
    const kind = this.activeShapeKind();
    if (kind) {
      const { raw, snapped } = shapeFromDrag(kind, g.start, g.end, { straighten: this.config.straighten });
      return this.makeShape(g, snapped ?? raw, 'preview', dashed);
    }
    if (g.held && this.config.shapeApply === 'auto') return this.makeShape(g, g.held, 'preview', dashed);
    return /** @type {StrokeItem} */ ({
      id: 'preview',
      kind: 'stroke',
      tool: g.tool,
      color: g.color,
      width: g.width,
      opacity: g.tool === 'marker' ? MARKER_OPACITY : 1,
      dashed,
      points: g.smooth,
    });
  }

  /**
   * 획 끝: add(손획) → (보정이 있으면) correct(손획, 도형) 2명령(A §2.3).
   * @param {PointerSample} e
   * @returns {InkEffect[]}
   */
  finishDraw(e) {
    const g = this.gesture;
    const d = this.docPoint(e);
    if (Math.hypot(d.x - g.end.x, d.y - g.end.y) > 1e-6) this.addDrawSample(e, d);
    const dashed = this.mods.alt;
    const page = this.doc.activePage;
    /** @type {InkEffect[]} */
    const fx = [];
    const kind = this.activeShapeKind();
    if (kind) {
      const { raw, snapped } = shapeFromDrag(kind, g.start, g.end, { straighten: this.config.straighten });
      if (Math.abs(raw.x2 - raw.x1) * this.view.s < 1 && Math.abs(raw.y2 - raw.y1) * this.view.s < 1) return fx;
      const item = this.makeShape(g, raw, this.nextId(), dashed);
      this.push({ type: 'add', page, item });
      if (snapped) {
        this.push({ type: 'correct', page, before: item, after: this.makeShape(g, snapped, item.id, dashed) });
        fx.push({ type: 'notice', text: this.noticeOf(raw.y2 !== snapped.y2 ? 'horizontal' : 'vertical') });
      }
      return fx;
    }
    const short = pathLength(g.raw) < SHORT_STROKE_LEN;
    /** @type {StrokePoint[]} */
    const points = short ? g.raw.map((/** @type {StrokePoint} */ p) => ({ ...p })) : simplifyStroke(g.smooth);
    const stroke = this.makeStroke(g, points, this.nextId(), dashed);
    this.push({ type: 'add', page, item: stroke });
    if (g.held) {
      const shaped = this.makeShape(g, g.held, stroke.id, dashed);
      const name = /** @type {'line' | 'circle' | 'ellipse' | 'rect' | 'arrow'} */ (
        g.held.shape === 'ellipse' ? (g.held.circle ? 'circle' : 'ellipse') : g.held.shape
      );
      if (this.config.shapeApply === 'auto') {
        // 바로 바꾸기: add → correct 2명령(A §2.3)
        this.push({ type: 'correct', page, before: stroke, after: shaped });
        fx.push({ type: 'notice', text: this.undoNoticeOf(name) });
      } else {
        // 제안형(기본): 손획은 그대로, Enter·섬 클릭으로 수락할 때까지 제안만 띄운다
        const text = this.suggestOf(name);
        this.suggestion = { strokeId: stroke.id, page, shaped, name, text, expiresAt: e.t + SUGGEST_TIMEOUT_MS };
        this.revs.active++;
        fx.push({ type: 'suggest', text, shape: name });
      }
    }
    return fx;
  }

  // ── 지우개 ──

  /** @param {{ x: number, y: number }} d */
  startErase(d) {
    this.gesture = { kind: 'erase', last: d, radius: this.eraserRadius, hits: new Set() };
    this.eraseTo(d);
  }

  /** @param {{ x: number, y: number }} d */
  eraseTo(d) {
    const g = this.gesture;
    let hit = false;
    for (const it of this.items) {
      if (g.hits.has(it.id)) continue;
      if (hitItemBySegment(it, g.last.x, g.last.y, d.x, d.y, g.radius)) {
        g.hits.add(it.id);
        hit = true;
      }
    }
    g.last = d;
    if (hit) {
      this.hidden = new Set(g.hits);
      this.revs.hidden++;
    }
    this.revs.active++;
  }

  /** 한 번 끌기 = remove 명령 1개(A §2.2). */
  finishErase() {
    const g = this.gesture;
    if (this.hidden.size) {
      this.hidden = new Set();
      this.revs.hidden++;
    }
    if (!g.hits.size) return;
    const entries = this.items.map((item, index) => ({ index, item })).filter((e) => g.hits.has(e.item.id));
    this.selection = this.selection.filter((id) => !g.hits.has(id));
    this.push({ type: 'remove', page: this.doc.activePage, entries });
  }

  // ── 고르기 ──

  /**
   * @param {PointerSample} e
   * @param {{ x: number, y: number }} d
   */
  startSelect(e, d) {
    const s = this.view.s;
    const box = this.selectionBox();
    if (box && this.selection.length === 1) {
      const handle = hitHandle(box, d.x, d.y, HANDLE_RADIUS_PX / s);
      const it = this.items.find((i) => i.id === this.selection[0]);
      if (handle && it) {
        const before = itemGeomRect(it);
        const fixed = {
          x: handle === 'nw' || handle === 'sw' ? before.x + before.w : before.x,
          y: handle === 'nw' || handle === 'ne' ? before.y + before.h : before.y,
        };
        this.gesture = { kind: 'resize', id: it.id, handle, before, fixed, after: null, fontBefore: it.kind === 'text' ? it.fontSize : undefined };
        return;
      }
    }
    const idx = topItemAt(this.items, d.x, d.y, SELECT_TOLERANCE_PX / s);
    if (idx >= 0) {
      const id = this.items[idx].id;
      if (e.shiftKey) {
        if (!this.selection.includes(id)) this.selection = [...this.selection, id];
      } else if (!this.selection.includes(id)) this.selection = [id];
      this.gesture = { kind: 'move', start: d, dx: 0, dy: 0, ids: this.selection.slice(), moved: false };
    } else if (box && pointInRect(box, d.x, d.y)) {
      this.gesture = { kind: 'move', start: d, dx: 0, dy: 0, ids: this.selection.slice(), moved: false };
    } else {
      this.selection = [];
      this.gesture = { kind: 'marquee', start: d, end: d };
    }
    this.revs.active++;
  }

  /** @param {{ x: number, y: number }} d */
  updateResize(d) {
    const g = this.gesture;
    const min = 1 / this.view.s;
    const it = this.items.find((i) => i.id === g.id);
    if (!it) return;
    let after = rectFromPoints(g.fixed.x, g.fixed.y, d.x, d.y);
    if (it.kind === 'text' && g.before.h > EPS) {
      const k = Math.max(min / g.before.h, after.h / g.before.h);
      const w = g.before.w * k;
      const h = g.before.h * k;
      after = {
        x: d.x < g.fixed.x ? g.fixed.x - w : g.fixed.x,
        y: d.y < g.fixed.y ? g.fixed.y - h : g.fixed.y,
        w,
        h,
      };
      g.fontAfter = (g.fontBefore ?? it.fontSize) * k;
    } else {
      if (g.before.w > EPS) after.w = Math.max(min, after.w);
      else after = { ...after, x: g.before.x, w: 0 };
      if (g.before.h > EPS) after.h = Math.max(min, after.h);
      else after = { ...after, y: g.before.y, h: 0 };
    }
    if (!g.after) {
      this.hidden = new Set([g.id]);
      this.revs.hidden++;
    }
    g.after = after;
    this.revs.active++;
  }

  finishMove() {
    const g = this.gesture;
    if (g.moved) {
      this.hidden = new Set();
      this.revs.hidden++;
      if (Math.abs(g.dx) > EPS || Math.abs(g.dy) > EPS) {
        this.push({ type: 'move', page: this.doc.activePage, ids: g.ids, dx: g.dx, dy: g.dy });
      }
    }
  }

  finishResize() {
    const g = this.gesture;
    if (!g.after) return;
    this.hidden = new Set();
    this.revs.hidden++;
    /** @type {InkCommand} */
    const cmd = { type: 'resize', page: this.doc.activePage, id: g.id, before: g.before, after: g.after };
    if (g.fontBefore != null) {
      cmd.fontBefore = g.fontBefore;
      cmd.fontAfter = g.fontAfter;
    }
    this.push(cmd);
  }

  finishMarquee() {
    const g = this.gesture;
    const r = rectFromPoints(g.start.x, g.start.y, g.end.x, g.end.y);
    this.selection = r.w * this.view.s < 2 && r.h * this.view.s < 2 ? [] : itemsInRect(this.items, r);
  }

  // ── 휠 · tick ──

  /**
   * 휠: 그냥 = 확대 단계(커서 고정점), Ctrl(맥 Cmd)+휠 = 굵기 1~30(고른 항목이 있으면 그 굵기).
   * @param {{ x: number, y: number, deltaY: number, ctrlKey?: boolean, metaKey?: boolean }} e  deltaY는 px로 맞춰서
   * @returns {InkEffect[]}
   */
  wheel(e) {
    this.cursor = { x: e.x, y: e.y };
    if (this.wheelAcc !== 0 && Math.sign(this.wheelAcc) !== Math.sign(e.deltaY)) this.wheelAcc = 0;
    this.wheelAcc += e.deltaY;
    if (Math.abs(this.wheelAcc) < WHEEL_STEP_DELTA) return [];
    const dir = this.wheelAcc < 0 ? 1 : -1;
    this.wheelAcc = 0;
    const widthMode = this.platform === 'macos' ? Boolean(e.metaKey) : Boolean(e.ctrlKey);
    const changed = widthMode ? this.changeWidth(dir) : this.zoomStep(dir, { x: e.x, y: e.y });
    if (changed) this.notify();
    return this.takeEffects();
  }

  /**
   * 화면 프레임마다(rAF): 0.5초 멈춤 판정 + 가장자리 밀기(02 §3.4).
   * @param {number} t  performance.now()
   * @returns {InkEffect[]}
   */
  tick(t) {
    // 첫 프레임은 시각만 기록, 쉬었다 다시 돌면(> 100 ms) 한 프레임으로 본다(순간 이동 방지).
    const gap = this.lastTick == null ? 0 : t - this.lastTick;
    const dt = gap > 100 ? 1000 / 60 : gap;
    this.lastTick = t;
    let changed = false;
    const g = this.gesture;
    if (g?.kind === 'draw') changed = this.checkHold(t) || changed;
    if (this.suggestion && t >= this.suggestion.expiresAt) changed = this.dropSuggestion() || changed;
    if (this.edgePushActive() && this.cursor) {
      const push = edgePushDelta(this.cursor, this.viewport, dt);
      if (this.setView(panBy(this.view, push.dx, push.dy, this.viewport, this.doc.docSize))) {
        changed = true;
        if (g?.kind === 'draw') {
          const c = this.cursor;
          const last = g.raw[g.raw.length - 1];
          this.addDrawSample({ x: c.x, y: c.y, t, pointerType: 'pen', pressure: last?.p }, this.docPoint({ x: c.x, y: c.y, t }));
        } else if (g?.kind === 'erase') this.eraseTo(this.docPoint({ x: this.cursor.x, y: this.cursor.y, t }));
      }
    }
    if (changed) this.notify();
    return this.takeEffects();
  }

  // ── 키 ──

  /**
   * @param {KeyInput} e
   * @returns {boolean} 수정 키 상태가 바뀌었는지
   */
  updateModsFromKey(e, down = true) {
    let changed = false;
    if (e.code === 'Tab' && !this.editing) {
      changed = this.mods.tab !== down;
      this.mods.tab = down;
    }
    if (typeof e.shiftKey === 'boolean') changed = this.updateMods(/** @type {any} */ (e)) || changed;
    return changed;
  }

  /**
   * 키 누름. handled = true면 화면은 기본 동작을 막는다(preventDefault).
   * @param {KeyInput} e
   * @returns {{ handled: boolean, effects: InkEffect[] }}
   */
  keyDown(e) {
    const t = e.t ?? 0;
    /** @type {InkEffect[]} */
    const effects = [];
    const modsChanged = this.updateModsFromKey(e, true);
    if (modsChanged && this.gesture?.kind === 'draw') {
      this.revs.active++;
      this.notify();
    }
    if (this.ime.blocks(e, t)) return { handled: false, effects };
    if (this.editing) {
      if (e.code === 'Escape' && !e.ctrlKey && !e.altKey && !e.metaKey) {
        effects.push(...this.escape());
        this.notify();
        return { handled: true, effects: [...effects, ...this.takeEffects()] };
      }
      return { handled: false, effects };
    }
    const plain = !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey;
    if (this.suggestion && !this.gesture && plain && (e.code === 'Enter' || e.code === 'NumpadEnter')) {
      return { handled: true, effects: [...effects, ...this.acceptSuggestion()] };
    }
    if (e.code === 'Tab') return { handled: true, effects };
    if (e.code && MOD_CODES.has(e.code)) return { handled: e.code.startsWith('Alt'), effects };
    const id = matchDrawKey(e, this.keymap, this.platform);
    if (!id) {
      // 칠판(Alt+3 / ⌥3)에서만 PgDn/PgUp 쪽 넘기기를 처리한다(화면 위 그리기는 아래 앱 발표 넘기기와 겹치지 않게 그대로 통과).
      if (this.doc.mode === 'board') {
        const pageDir = matchBoardPageKey(e, this.keymap, this.platform);
        if (pageDir) {
          if (e.repeat || this.gesture) return { handled: true, effects };
          const changed = pageDir === 'next' ? this.nextBoardPage() : this.prevBoardPage();
          if (changed) this.notify();
          return { handled: true, effects: [...effects, ...this.takeEffects()] };
        }
      }
      return { handled: false, effects };
    }
    if (e.repeat && NO_REPEAT.has(id)) return { handled: true, effects };
    if (this.gesture && !['d-exit', 'd-undo', 'd-redo'].includes(id)) return { handled: true, effects };
    const out = this.runAction(id);
    this.notify();
    return { handled: true, effects: [...effects, ...this.takeEffects(), ...out] };
  }

  /**
   * @param {KeyInput} e
   * @returns {{ handled: boolean, effects: InkEffect[] }}
   */
  keyUp(e) {
    const changed = this.updateModsFromKey(e, false);
    if (changed && this.gesture?.kind === 'draw') {
      this.revs.active++;
      this.notify();
    }
    return { handled: e.code === 'Tab' || Boolean(e.code?.startsWith('Alt')), effects: [] };
  }

  /**
   * 판서 키 칸 하나 실행(R-DRAW-3, A §2.6).
   * @param {string} id
   * @returns {InkEffect[]}
   */
  runAction(id) {
    const tool = /** @type {Record<string, InkTool>} */ (DRAW_TOOL_KEYS)[id];
    if (tool) {
      this.setTool(tool);
      return [];
    }
    const color = /** @type {Record<string, InkColor>} */ (DRAW_COLOR_KEYS)[id];
    if (color) {
      this.setColor(color);
      return [];
    }
    if (LATER_DRAW_KEYS.includes(id)) return [];
    switch (id) {
      case 'd-clear':
        if (!this.items.length) return [];
        this.selection = [];
        this.push({ type: 'clear', page: this.doc.activePage, items: this.items.slice() });
        return [{ type: 'notice', text: this.noticeOf('cleared'), action: 'undo' }];
      case 'd-white':
        this.toggleBackground('white');
        return [];
      case 'd-black':
        this.toggleBackground('black');
        return [];
      case 'd-undo':
        if (this.gesture) this.cancelGesture();
        else this.undo();
        return [];
      case 'd-redo':
        if (!this.gesture) this.redo();
        return [];
      case 'd-copy':
        return [{ type: 'copy' }];
      case 'd-save':
        return [{ type: 'save' }];
      case 'd-restore':
        return [{ type: 'restore' }];
      case 'd-center':
        // 커서 가운데 + 1× 복귀(A §2.6). 실제 커서 옮기기는 바깥(centerCursor)이 맡는다.
        this.setView(IDENTITY_VIEW);
        return [{ type: 'centerCursor' }];
      case 'd-zin':
        this.zoomStep(1);
        return [];
      case 'd-zout':
        this.zoomStep(-1);
        return [];
      case 'd-wup':
        this.changeWidth(1);
        return [];
      case 'd-wdown':
        this.changeWidth(-1);
        return [];
      case 'd-help':
        this.helpOpen = !this.helpOpen;
        return [{ type: 'help', open: this.helpOpen }];
      case 'd-exit':
        return this.escape();
      default:
        return [];
    }
  }

  /**
   * Esc(R-DRAW-10): step = 도움말 → 지금 획 취소 → 글자 확정 → 고르기 해제 → 손 모드 해제 → 1× → 나가기 / exit = 바로 나가기.
   * @returns {InkEffect[]}
   */
  escape() {
    if (this.config.esc === 'exit') {
      if (this.editing) this.commitText();
      this.cancelGesture();
      this.dropSuggestion();
      return [{ type: 'exit', reason: 'esc' }];
    }
    if (this.helpOpen) {
      this.helpOpen = false;
      return [{ type: 'help', open: false }];
    }
    if (this.cancelGesture()) return [];
    if (this.dropSuggestion()) return []; // Esc = 제안 버림
    if (this.editing) {
      this.commitText();
      return [];
    }
    if (this.selection.length) {
      this.selection = [];
      this.revs.active++;
      return [];
    }
    if (this.tool === 'pan') {
      this.tool = this.prevTool === 'pan' ? 'pen' : this.prevTool;
      this.revs.active++;
      return [];
    }
    if (this.view.s > 1 + EPS) {
      this.setView(IDENTITY_VIEW);
      return [];
    }
    return [{ type: 'exit', reason: 'esc' }];
  }

  // ── 보관 · 복원 ──

  /**
   * 문서 통째로 바꾸기(실행 취소 기록은 비운다).
   * @param {InkDocument} doc
   */
  loadDocument(doc) {
    this.dropSuggestion();
    this.doc = createInkDocument(doc);
    this.undoStack = [];
    this.redoStack = [];
    this.selection = [];
    this.editing = null;
    this.gesture = null;
    this.hidden = new Set();
    this.syncIdCounters();
    this.view = clampView(this.view, this.viewport, this.doc.docSize);
    this.revs.view++;
    this.revs.hidden++;
    this.revs.active++;
    this.markDoc({ kind: 'full' });
    this.notify();
  }

  /**
   * 보관 판서 복원(02 §3.6, drawRestore 결과). 지금 문서의 모니터 정보는 그대로 두고 장·항목만 가져온다.
   * @param {{ document: InkDocument } | InkDocument | null | undefined} kept
   * @param {'none' | 'same' | 'rescaled' | 'differentMonitor'} status
   * @returns {InkEffect[]}
   */
  restoreKept(kept, status) {
    if (status === 'differentMonitor') return [{ type: 'notice', text: this.noticeOf('differentMonitor') }];
    const src = kept && 'document' in kept ? kept.document : /** @type {InkDocument | null | undefined} */ (kept);
    if (status === 'none' || !src || !Array.isArray(src.pages)) return [];
    let pages = createInkDocument(src).pages;
    if (status === 'rescaled' && src.docSize?.w > 0) {
      const k = this.doc.docSize.w / src.docSize.w;
      pages = rescaleDocument(createInkDocument(src), k).pages;
    }
    // 실행 취소할 수 있는 명령 하나로(그 전 기록은 그대로) — Ctrl+Z 1번 = 복원 전 그리기
    if (this.editing) this.commitText();
    this.cancelGesture();
    this.selection = [];
    if (this.hidden.size) {
      this.hidden = new Set();
      this.revs.hidden++;
    }
    this.push({
      type: 'replaceDoc',
      before: { pages: this.doc.pages, activePage: this.doc.activePage },
      after: { pages, activePage: clamp(src.activePage ?? 0, 0, pages.length - 1) },
    });
    this.syncIdCounters();
    this.revs.active++;
    this.notify();
    return [...this.takeEffects(), { type: 'notice', text: this.noticeOf(status === 'rescaled' ? 'rescaled' : 'restored') }];
  }
}

/**
 * @param {InkEngineOptions} [opts]
 */
export function createInkEngine(opts) {
  return new InkEngine(opts);
}
