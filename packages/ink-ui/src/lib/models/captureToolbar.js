// 선택 툴바(R-CAP-27, 04 S08) 순수 모델 — 어느 모니터에 띄울지, 어디에 붙일지, 단추·툴팁.
// 좌표: 사각형·모니터 = 가상 화면 물리 px, 배치 = 그 모니터 오버레이 창의 CSS px.

import { formatComboReadable } from './shortcutDisplay.js';

/** 범위와 툴바 사이(CSS px) */
export const TOOLBAR_GAP = 8;
/** 모니터 가장자리에서 띄우는 거리(CSS px) */
export const TOOLBAR_EDGE = 8;

/**
 * 단추(왼→오). 주석 도구(R-CAP-28)는 판서 엔진(단계 4.1)과 함께 앞에 붙는다.
 * `command` = 앱에 보내는 CaptureInput {type:'command', name}, `slot` = 툴팁에 쓸 지금 키(확정본 칸).
 * @type {ReadonlyArray<{ id: string, icon: string, label: string, slot: string, command: string, primary?: boolean }>}
 */
export const TOOLBAR_ACTIONS = Object.freeze([
  { id: 'cancel', icon: 'close', label: '취소', slot: 'c-cancel', command: 'cancel' },
  // 캡처 핀(R-CAP-29) — 키 칸 없음(툴팁 = 이름 + 한 줄 설명)
  { id: 'pin', icon: 'pin', label: '화면에 고정', slot: '', command: 'pin' },
  { id: 'save', icon: 'save', label: '저장', slot: 'c-save', command: 'save' },
  { id: 'copy', icon: 'copy', label: '복사', slot: 'c-copy', command: 'copy', primary: true },
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
 * 범위가 이 모니터 밖으로 나가 있어도(여러 모니터) 보이는 부분 기준으로 자른 뒤 계산한다.
 * @param {{ x: number, y: number, width: number, height: number }} sel  이 모니터 CSS 좌표의 범위(잘리지 않은 값이어도 됨)
 * @param {{ w: number, h: number }} viewport  오버레이 창 CSS 크기
 * @param {{ w: number, h: number }} size  툴바 CSS 크기
 * @returns {{ x: number, y: number, side: 'below' | 'above' | 'inside' }}
 */
export function placeToolbar(sel, viewport, size) {
  const vw = Math.max(0, viewport.w);
  const vh = Math.max(0, viewport.h);
  // 보이는 부분
  const left = Math.max(0, sel.x);
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

/** 설정을 아직 못 받았을 때의 기본값(확정본 v1). 실행 취소·다시 실행은 판서 키 칸(d-undo·d-redo)을 같이 쓴다. */
const FALLBACK = Object.freeze({
  windows: { 'c-cancel': 'Esc', 'c-save': 'Ctrl+S', 'c-copy': 'Enter', 'd-undo': 'Ctrl+Z', 'd-redo': 'Ctrl+Y' },
  macos: { 'c-cancel': 'Esc', 'c-save': 'Cmd+S', 'c-copy': 'Enter', 'd-undo': 'Cmd+Z', 'd-redo': 'Shift+Cmd+Z' },
});

// ─────────────── 캡처 주석 도구(R-CAP-28, B §2.1) ───────────────

/**
 * 주석 도구 8종(왼→오). 판서 엔진(inkEngine.js)을 그대로 쓴다: 도형은 엔진 손그림 도구 + 수정 키 도형,
 * 모자이크는 엔진 문서에 mosaic 항목을 더한다(captureAnnotate.js).
 * @type {ReadonlyArray<{ id: string, icon: string, label: string, tool: 'shape' | 'pen' | 'marker' | 'mosaic' | 'text' | 'eraser', shape?: 'rect' | 'ellipse' | 'arrow' }>}
 */
export const TOOLBAR_ANNOTATE_TOOLS = Object.freeze([
  { id: 'rect', icon: 'rect', label: '사각형', tool: 'shape', shape: 'rect' },
  { id: 'ellipse', icon: 'ellipse', label: '타원', tool: 'shape', shape: 'ellipse' },
  { id: 'arrow', icon: 'arrow', label: '화살표', tool: 'shape', shape: 'arrow' },
  { id: 'pen', icon: 'pen', label: '펜', tool: 'pen' },
  { id: 'marker', icon: 'marker', label: '형광펜', tool: 'marker' },
  { id: 'mosaic', icon: 'mosaic', label: '모자이크', tool: 'mosaic' },
  { id: 'text', icon: 'text', label: '글자', tool: 'text' },
  { id: 'eraser', icon: 'eraser', label: '지우개', tool: 'eraser' },
]);

/**
 * 실행 취소·다시 실행. `slot` = 툴팁 키(판서 키 칸을 같이 씀), `command` = 오버레이 안에서 엔진에.
 * @type {ReadonlyArray<{ id: string, icon: string, label: string, slot: string, command: string }>}
 */
export const TOOLBAR_HISTORY = Object.freeze([
  { id: 'undo', icon: 'undo', label: '실행 취소', slot: 'd-undo', command: 'annot.undo' },
  { id: 'redo', icon: 'redo', label: '다시 실행', slot: 'd-redo', command: 'annot.redo' },
]);

/** 보조 옵션: 색 6(판서 S12와 같음) · 굵기 3단(화면 px) · 모자이크 칸 3단(논리 px, Ctrl+휠로 6~24). */
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
/** 보조 옵션 알약과 툴바 사이(CSS px). */
export const TOOLBAR_OPTIONS_GAP = 4;

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
 * 도구가 고른 상태에서 보일 보조 옵션.
 * @param {string | null} toolId
 * @returns {'stroke' | 'mosaic' | 'none'}
 */
export function annotOptionsKind(toolId) {
  if (!toolId || toolId === 'eraser') return 'none';
  return toolId === 'mosaic' ? 'mosaic' : 'stroke';
}

/**
 * 보조 옵션 알약 자리(CSS px, 왼쪽 위): 툴바 바깥쪽(툴바가 범위 아래면 아래, 위면 위)으로 4px.
 * 그쪽에 자리가 없으면 반대쪽. 가로는 툴바 오른쪽 끝에 맞추고 화면 안으로.
 * @param {{ x: number, y: number, side: 'below' | 'above' | 'inside' }} bar  placeToolbar 결과
 * @param {{ w: number, h: number }} barSize
 * @param {{ w: number, h: number }} optsSize
 * @param {{ w: number, h: number }} viewport
 * @returns {{ x: number, y: number, side: 'below' | 'above' }}
 */
export function placeToolOptions(bar, barSize, optsSize, viewport) {
  const maxX = Math.max(TOOLBAR_EDGE, viewport.w - TOOLBAR_EDGE - optsSize.w);
  const x = Math.min(maxX, Math.max(TOOLBAR_EDGE, bar.x + barSize.w - optsSize.w));
  const below = bar.y + barSize.h + TOOLBAR_OPTIONS_GAP;
  const above = bar.y - TOOLBAR_OPTIONS_GAP - optsSize.h;
  const fitsBelow = below + optsSize.h <= viewport.h - TOOLBAR_EDGE;
  const fitsAbove = above >= TOOLBAR_EDGE;
  const preferBelow = bar.side === 'below';
  if (preferBelow ? fitsBelow || !fitsAbove : !fitsAbove && fitsBelow) return { x, y: below, side: 'below' };
  return { x, y: above, side: 'above' };
}

/**
 * 툴팁 "복사 · Enter"(지금 설정된 키, 읽기 표기). 키를 비워 두었으면 이름만.
 * @param {{ label: string, slot: string }} action
 * @param {Record<string, string>} combos
 * @param {'windows' | 'macos'} [platform='windows']
 * @returns {string}
 */
export function toolbarTooltip(action, combos, platform = 'windows') {
  const fallback = FALLBACK[platform] ?? FALLBACK.windows;
  const combo = Object.prototype.hasOwnProperty.call(combos ?? {}, action.slot) ? combos[action.slot] : fallback[action.slot];
  const readable = combo ? formatComboReadable(combo, platform) : '';
  return readable ? `${action.label} · ${readable}` : action.label;
}
