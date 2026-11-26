// 그리기·칠판 도구 막대(R2-DRAW-TB) 및 판서 색·굵기 패널(R2-ANN-5) 순수 모델. DOM 의존 0.

import { DRAW_SETTING_MAX_PX, TEXT_SIZE_MUL } from '../ink/inkEngine.js';
import { defaultDrawCombo, normalizeCombo } from '../ink/drawKeys.js';
import { clamp } from '../ink/coords.js';
import { BASIC_COLOR_CODES } from './palette.js';
import { clampToolSize, getToolSizeSpec } from './inkSize.js';
import { formatComboReadable } from './shortcutDisplay.js';

/** 도구 막대가 손대지 않은 뒤 흐려지는 시간(ms). */
export const DRAW_TOOLBAR_IDLE_MS = 3000;
/** 흐려졌을 때 도구 막대 투명도. */
export const DRAW_TOOLBAR_DIM_OPACITY = 0.28;
/** 커서가 도구 막대에 가까워졌다고 보는 여유 거리(CSS px). */
export const DRAW_TOOLBAR_PROXIMITY_PX = 72;
/** 화면 아래에서 도구 막대까지 여백(CSS px). */
export const DRAW_TOOLBAR_BOTTOM_PX = 18;
/** 도구 막대와 색·굵기 패널 사이 간격(CSS px). */
export const DRAW_TOOLBAR_GAP_PX = 10;
/** 화면 가장자리 최소 여백(CSS px). */
export const DRAW_TOOLBAR_EDGE_PX = 8;

/**
 * 그리기·칠판 도구 막대 왼쪽 묶음 7종(R2-DRAW-TB):
 * 펜 · 형광펜 · 지우개 · 도형(직선 · 사각형 · 원) · 글자.
 * @type {ReadonlyArray<{
 *   id: string,
 *   icon: string,
 *   labelKey: string,
 *   kind: 'tool' | 'shape',
 *   tool?: 'pen' | 'marker' | 'eraser' | 'text',
 *   shape?: 'line' | 'rect' | 'ellipse',
 *   slot?: string,
 *   modKey?: 'Shift' | 'primary' | 'Tab',
 * }>}
 */
export const DRAW_TOOLBAR_TOOLS = Object.freeze([
  Object.freeze({ id: 'pen', icon: 'pen', labelKey: 'draw.tool.pen', kind: 'tool', tool: 'pen', slot: 'd-pen' }),
  Object.freeze({ id: 'marker', icon: 'marker', labelKey: 'draw.tool.marker', kind: 'tool', tool: 'marker', slot: 'd-marker' }),
  Object.freeze({ id: 'eraser', icon: 'eraser', labelKey: 'draw.tool.eraser', kind: 'tool', tool: 'eraser', slot: 'd-eraser' }),
  Object.freeze({ id: 'line', icon: 'line', labelKey: 'draw.tool.line', kind: 'shape', shape: 'line', modKey: 'Shift' }),
  Object.freeze({ id: 'rect', icon: 'rect', labelKey: 'draw.tool.rect', kind: 'shape', shape: 'rect', modKey: 'primary' }),
  Object.freeze({ id: 'ellipse', icon: 'ellipse', labelKey: 'draw.tool.ellipse', kind: 'shape', shape: 'ellipse', modKey: 'Tab' }),
  Object.freeze({ id: 'text', icon: 'text', labelKey: 'draw.tool.text', kind: 'tool', tool: 'text', slot: 'd-text' }),
]);

/**
 * 되돌리기 · 다시 하기 묶음.
 * @type {ReadonlyArray<{ id: 'undo' | 'redo', icon: string, labelKey: string, slot: string }>}
 */
export const DRAW_TOOLBAR_HISTORY = Object.freeze([
  Object.freeze({ id: 'undo', icon: 'undo', labelKey: 'draw.action.undo', slot: 'd-undo' }),
  Object.freeze({ id: 'redo', icon: 'redo', labelKey: 'draw.action.redo', slot: 'd-redo' }),
]);

/**
 * 엔진의 현재 도구·도형 도구 상태로 도구 막대 활성 id를 계산한다.
 * @param {string} tool
 * @param {string | null | undefined} [shapeTool]
 * @returns {string}
 */
export function activeDrawToolId(tool, shapeTool = null) {
  if (tool === 'pen' && (shapeTool === 'line' || shapeTool === 'rect' || shapeTool === 'ellipse')) {
    return shapeTool;
  }
  return tool || 'pen';
}

/**
 * 도구 막대 id → `ColorWidthPanel` / `inkSize.getToolSizeSpec`가 쓰는 toolId.
 * @param {string | null | undefined} toolId
 * @returns {'pen' | 'marker' | 'eraser' | 'text' | 'rect' | 'ellipse'}
 */
export function panelToolIdForDraw(toolId) {
  if (toolId === 'marker' || toolId === 'eraser' || toolId === 'text' || toolId === 'rect' || toolId === 'ellipse') {
    return toolId;
  }
  return 'pen';
}

/**
 * `ColorWidthPanel`의 도구별 크기 값을 엔진 `widthPx` 단위로 바꾼다.
 * @param {string | null | undefined} toolId
 * @param {number} size
 * @returns {number}
 */
export function drawToolSizeToEngineWidth(toolId, size) {
  const id = panelToolIdForDraw(toolId);
  const spec = getToolSizeSpec(id);
  const clamped = clampToolSize(size, spec);
  if (id === 'eraser') return Math.max(1, Math.round((clamped / 3) * 10) / 10);
  if (id === 'text') return Math.max(1, Math.round((clamped / TEXT_SIZE_MUL) * 10) / 10);
  return clamped;
}

/**
 * 엔진 `widthPx` → `ColorWidthPanel`의 도구별 크기 값(정수, 도구 범위 안).
 * @param {string | null | undefined} toolId
 * @param {number} engineWidth
 * @returns {number}
 */
export function engineWidthToDrawToolSize(toolId, engineWidth) {
  const id = panelToolIdForDraw(toolId);
  const spec = getToolSizeSpec(id);
  const raw = Number.isFinite(engineWidth) ? engineWidth : spec.defaultVal;
  if (id === 'eraser') return clampToolSize(raw * 3, spec);
  if (id === 'text') return clampToolSize(raw * TEXT_SIZE_MUL, spec);
  return clampToolSize(raw, spec);
}

/**
 * 판서 설정 `draw.width` 저장용 값(Rust 설정 범위 1~30으로 가둔다).
 * @param {number} val
 * @returns {number}
 */
export function clampDrawSettingWidth(val) {
  if (!Number.isFinite(val)) return 4;
  return clamp(Math.round(val), 1, DRAW_SETTING_MAX_PX);
}

/**
 * `drawIsland` / `draw.color`에 전달할 기본 8색 코드(넓힌 색·HEX면 `undefined`).
 * @param {string | null | undefined} color
 * @returns {string | undefined}
 */
export function drawIslandColor(color) {
  return color && BASIC_COLOR_CODES.includes(color) ? color : undefined;
}

/**
 * 칠판 ↔ 화면 전환 단추의 다음 배경·라벨 키·아이콘을 결정한다.
 * - 그리기 모드(`mode === 'draw'`): 화면(`frame`) ↔ 칠판(`white`), 검은 칠판이면 화면으로 복귀.
 * - 칠판 모드(`mode === 'board'`): 흰 칠판(`white`) ↔ 검은 칠판(`black`).
 * @param {{ mode?: 'draw' | 'board' | 'capture', background?: 'frame' | 'white' | 'black' }} opts
 * @returns {{ nextBackground: 'frame' | 'white' | 'black', labelKey: string, icon: 'board' | 'monitor' | 'dark', slot: string }}
 */
export function resolveBoardToggleAction({ mode = 'draw', background = 'frame' } = {}) {
  if (mode === 'board') {
    const toBlack = background !== 'black';
    return {
      nextBackground: toBlack ? 'black' : 'white',
      labelKey: toBlack ? 'draw.action.toBlackboard' : 'draw.action.toWhiteboard',
      icon: toBlack ? 'dark' : 'board',
      slot: toBlack ? 'd-black' : 'd-white',
    };
  }
  const onBoard = background !== 'frame';
  return {
    nextBackground: onBoard ? 'frame' : 'white',
    labelKey: onBoard ? 'draw.action.toScreen' : 'draw.action.toBoard',
    icon: onBoard ? 'monitor' : 'board',
    slot: background === 'black' ? 'd-black' : 'd-white',
  };
}

/**
 * 도구·동작 말풍선 문구(`"펜 · P"`, `"되돌리기 · ⌘Z"`, `"사각형 · ⌘"`).
 * @param {{ label: string, slot?: string, modKey?: string, key?: string }} item
 * @param {Record<string, string> | null | undefined} combos
 * @param {'windows' | 'macos'} [platform='windows']
 * @returns {string}
 */
export function formatDrawTooltip(item, combos, platform = 'windows') {
  const label = item?.label ?? '';
  if (!label) return '';
  let readable = '';
  if (item.slot) {
    const map = combos ?? {};
    const raw = Object.prototype.hasOwnProperty.call(map, item.slot) ? map[item.slot] : defaultDrawCombo(item.slot, platform);
    const norm = normalizeCombo(raw, platform);
    readable = norm ? formatComboReadable(norm, platform) : '';
  } else if (item.modKey) {
    if (item.modKey === 'primary') readable = platform === 'macos' ? '⌘' : 'Ctrl';
    else if (item.modKey === 'Shift') readable = platform === 'macos' ? '⇧' : 'Shift';
    else readable = item.modKey;
  } else if (item.key) {
    readable = item.key;
  }
  return readable ? `${label} · ${readable}` : label;
}

/**
 * 도구 막대 표시·흐림·포인터 활성 판정(R2-DRAW-TB):
 * - 숨기기 키로 숨겼으면 `visible: false`.
 * - 그리는 중(`drawing`)이면 즉시 흐리고 포인터 통과(`interactive: false`).
 * - 패널 열림·호버·커서 근접 시 또렷하고 클릭 가능.
 * - 손대지 않고 3초 지나면(`idleDim`) 흐려진다.
 * @param {{
 *   hidden?: boolean,
 *   drawing?: boolean,
 *   idleDim?: boolean,
 *   hovering?: boolean,
 *   panelOpen?: boolean,
 *   nearToolbar?: boolean,
 * }} [opts]
 * @returns {{ visible: boolean, dimmed: boolean, interactive: boolean }}
 */
export function resolveDrawToolbarVisibility({
  hidden = false,
  drawing = false,
  idleDim = false,
  hovering = false,
  panelOpen = false,
  nearToolbar = false,
} = {}) {
  if (hidden) return { visible: false, dimmed: false, interactive: false };
  if (drawing) return { visible: true, dimmed: true, interactive: false };
  if (panelOpen || hovering || nearToolbar) return { visible: true, dimmed: false, interactive: true };
  return { visible: true, dimmed: Boolean(idleDim), interactive: true };
}

/**
 * 커서가 사각형 주변 여유 거리(`margin`) 안에 들어왔는지 판정한다.
 * @param {{ x: number, y: number } | null | undefined} cursor
 * @param {{ left?: number, top?: number, right?: number, bottom?: number } | null | undefined} rect
 * @param {number} [margin=DRAW_TOOLBAR_PROXIMITY_PX]
 * @returns {boolean}
 */
export function isNearRect(cursor, rect, margin = DRAW_TOOLBAR_PROXIMITY_PX) {
  if (!cursor || !rect) return false;
  if (!Number.isFinite(cursor.x) || !Number.isFinite(cursor.y)) return false;
  const { left, top, right, bottom } = rect;
  if (
    typeof left !== 'number' ||
    typeof top !== 'number' ||
    typeof right !== 'number' ||
    typeof bottom !== 'number' ||
    !Number.isFinite(left) ||
    !Number.isFinite(top) ||
    !Number.isFinite(right) ||
    !Number.isFinite(bottom) ||
    right <= left ||
    bottom <= top
  ) {
    return false;
  }
  const m = Number.isFinite(margin) ? Math.max(0, margin) : DRAW_TOOLBAR_PROXIMITY_PX;
  return cursor.x >= left - m && cursor.x <= right + m && cursor.y >= top - m && cursor.y <= bottom + m;
}

/**
 * 그리기 도구 막대 위 색·굵기 패널 자리 계산(CSS px).
 * 아래 가운데 도구 막대 바로 위(10px) 우선 배치, 화면 가장자리 8px 클램프.
 * @param {{ left: number, top: number, width: number, height: number }} barRect
 * @param {{ w: number, h: number }} panelSize
 * @param {{ w: number, h: number }} viewport
 * @param {number | null} [anchorCenterX]  뷰포트 기준 단추 가운데 X(없으면 막대 가운데)
 * @returns {{ x: number, y: number, side: 'above' | 'below', arrowX: number }}
 */
export function placeDrawColorPanel(barRect, panelSize, viewport, anchorCenterX = null) {
  const vw = Math.max(0, viewport.w);
  const vh = Math.max(0, viewport.h);
  const pw = Math.max(0, panelSize.w);
  const ph = Math.max(0, panelSize.h);
  const targetCx =
    typeof anchorCenterX === 'number' && Number.isFinite(anchorCenterX)
      ? anchorCenterX
      : barRect.left + barRect.width / 2;

  const maxX = Math.max(DRAW_TOOLBAR_EDGE_PX, vw - DRAW_TOOLBAR_EDGE_PX - pw);
  const x = Math.round(Math.min(maxX, Math.max(DRAW_TOOLBAR_EDGE_PX, targetCx - pw / 2)));

  const aboveY = barRect.top - DRAW_TOOLBAR_GAP_PX - ph;
  const belowY = barRect.top + barRect.height + DRAW_TOOLBAR_GAP_PX;
  const fitsAbove = aboveY >= DRAW_TOOLBAR_EDGE_PX;
  const fitsBelow = belowY + ph <= vh - DRAW_TOOLBAR_EDGE_PX;
  const side = fitsAbove || !fitsBelow ? 'above' : 'below';
  const maxY = Math.max(DRAW_TOOLBAR_EDGE_PX, vh - DRAW_TOOLBAR_EDGE_PX - ph);
  const rawY = side === 'above' ? aboveY : belowY;
  const y = Math.round(Math.min(maxY, Math.max(DRAW_TOOLBAR_EDGE_PX, rawY)));

  const arrowPad = Math.min(18, Math.max(8, pw / 4));
  const arrowX = Math.round(Math.min(Math.max(arrowPad, targetCx - x), Math.max(arrowPad, pw - arrowPad)));
  return { x, y, side, arrowX };
}
