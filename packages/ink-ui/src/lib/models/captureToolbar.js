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

/** 설정을 아직 못 받았을 때의 기본값(확정본 v1) */
const FALLBACK = Object.freeze({
  windows: { 'c-cancel': 'Esc', 'c-save': 'Ctrl+S', 'c-copy': 'Enter' },
  macos: { 'c-cancel': 'Esc', 'c-save': 'Cmd+S', 'c-copy': 'Enter' },
});

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
