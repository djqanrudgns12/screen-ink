// 판서 화면 안 키 29칸(`d-*`, R-DRAW-3·R-KEY-6) — 설정 단축키 표(state.shortcuts)에서 읽어
// KeyboardEvent.code 기준 조합(eventToCombo)으로 판정한다. 한글 IME 조합 중(isComposing·keyCode 229)과
// 조합이 끝난 직후 40 ms는 막는다(A §5). 맥은 Cmd = metaKey. DOM 의존 0.

import defaults from '../../../../../docs/shortcuts/defaults-v1.json' with { type: 'json' };
import { eventToCombo } from '../models/keyCombo.js';

/** @typedef {'windows' | 'macos'} Platform */
/** @typedef {{ id: string, scope?: string, combo: string }} ShortcutLike */
/**
 * @typedef {Object} KeyEventLike
 * @property {string} [code]
 * @property {string} [key]
 * @property {boolean} [ctrlKey]
 * @property {boolean} [altKey]
 * @property {boolean} [shiftKey]
 * @property {boolean} [metaKey]
 * @property {boolean} [isComposing]
 * @property {number} [keyCode]
 * @property {boolean} [repeat]
 */

/** 확정본 v1 판서 화면 안 29칸(catalog-v1 · ink-domain::shortcuts scope "draw"). */
export const DRAW_KEY_IDS = Object.freeze([
  'd-pen',
  'd-marker',
  'd-text',
  'd-select',
  'd-eraser',
  'd-clear',
  'd-red',
  'd-yellow',
  'd-green',
  'd-orange',
  'd-blue',
  'd-lime',
  'd-white',
  'd-black',
  'd-undo',
  'd-redo',
  'd-copy',
  'd-save',
  'd-restore',
  'd-center',
  'd-zin',
  'd-zout',
  'd-wup',
  'd-wdown',
  'd-help',
  'd-exit',
  'd-spot',
  'd-check',
  'd-number',
]);

/** 칸은 있지만 1차에서 동작하지 않는 후속 키(R-DRAW-7). */
export const LATER_DRAW_KEYS = Object.freeze(['d-spot', 'd-check', 'd-number']);

/** 조합이 끝난 직후 새어 나오는 키를 막는 시간(ms). */
export const IME_GUARD_MS = 40;

/** 색 키 → 펜 색 코드. */
export const DRAW_COLOR_KEYS = Object.freeze({
  'd-red': 'R',
  'd-yellow': 'Y',
  'd-green': 'G',
  'd-orange': 'O',
  'd-blue': 'B',
  'd-lime': 'L',
});

/** 도구 키 → 도구. */
export const DRAW_TOOL_KEYS = Object.freeze({
  'd-pen': 'pen',
  'd-marker': 'marker',
  'd-text': 'text',
  'd-select': 'select',
  'd-eraser': 'eraser',
});

/** @type {Record<string, { win?: string | null, mac?: string | null }>} */
const DEFAULT_SLOTS = /** @type {any} */ (defaults).slots;

/**
 * 확정본 기본값(설정 표에 칸이 없을 때만 쓴다).
 * @param {string} id
 * @param {Platform} [platform]
 */
export function defaultDrawCombo(id, platform = 'windows') {
  const slot = DEFAULT_SLOTS[id];
  if (!slot) return '';
  return (platform === 'macos' ? slot.mac : slot.win) ?? '';
}

const MOD_ALIASES = /** @type {Record<string, 'ctrl' | 'alt' | 'shift' | 'meta'>} */ ({
  ctrl: 'ctrl',
  control: 'ctrl',
  alt: 'alt',
  option: 'alt',
  opt: 'alt',
  shift: 'shift',
  win: 'meta',
  meta: 'meta',
  cmd: 'meta',
  command: 'meta',
  super: 'meta',
});

/**
 * 조합 문자열을 eventToCombo와 같은 정규 순서로(Windows: Ctrl+Alt+Shift+Win, 맥: Ctrl+Option+Shift+Cmd).
 * @param {string | null | undefined} combo
 * @param {Platform} [platform]
 * @returns {string}  빈 문자열 = 비움
 */
export function normalizeCombo(combo, platform = 'windows') {
  if (!combo) return '';
  const parts = String(combo)
    .split('+')
    .map((s) => s.trim())
    .filter(Boolean);
  if (!parts.length) return '';
  const mods = { ctrl: false, alt: false, shift: false, meta: false };
  /** @type {string | null} */
  let key = null;
  for (const part of parts) {
    const m = MOD_ALIASES[part.toLowerCase()];
    if (m) mods[m] = true;
    else key = part.length === 1 ? part.toUpperCase() : part;
  }
  if (!key) return '';
  const mac = platform === 'macos';
  const out = [];
  if (mods.ctrl) out.push('Ctrl');
  if (mods.alt) out.push(mac ? 'Option' : 'Alt');
  if (mods.shift) out.push('Shift');
  if (mods.meta) out.push(mac ? 'Cmd' : 'Win');
  out.push(key);
  return out.join('+');
}

/**
 * 설정 단축키 표 → 판서 키 판정표(정규 조합 → 칸 id). 비운 칸('')은 동작하지 않는다.
 * 표에 없는 칸만 확정본 기본값을 쓴다.
 * @param {ReadonlyArray<ShortcutLike> | null | undefined} shortcuts
 * @param {Platform} [platform]
 * @returns {Map<string, string>}
 */
export function buildDrawKeymap(shortcuts, platform = 'windows') {
  const byId = new Map((shortcuts ?? []).map((s) => [s.id, s]));
  /** @type {Map<string, string>} */
  const map = new Map();
  for (const id of DRAW_KEY_IDS) {
    const row = byId.get(id);
    const combo = normalizeCombo(row ? row.combo : defaultDrawCombo(id, platform), platform);
    if (combo && !map.has(combo)) map.set(combo, id);
  }
  return map;
}

/**
 * IME 조합 중인 키 사건인가(isComposing 또는 keyCode 229).
 * @param {KeyEventLike | null | undefined} event
 */
export function isComposingEvent(event) {
  return Boolean(event && (event.isComposing || event.keyCode === 229));
}

/**
 * 키 사건 → 판서 키 칸 id(없으면 null). 조합 중이면 null.
 * @param {KeyEventLike} event
 * @param {Map<string, string>} keymap
 * @param {Platform} [platform]
 * @returns {string | null}
 */
export function matchDrawKey(event, keymap, platform = 'windows') {
  if (!event || isComposingEvent(event)) return null;
  const combo = eventToCombo(event, platform);
  if (!combo) return null;
  return keymap.get(combo) ?? null;
}

/**
 * 조합 상태 추적 — compositionstart~compositionend와 끝난 뒤 40 ms 동안 도구·색 키를 막는다.
 */
export function createImeGuard() {
  let composing = false;
  let endedAt = -Infinity;
  return {
    start() {
      composing = true;
    },
    /** @param {number} t  ms */
    end(t) {
      composing = false;
      endedAt = t;
    },
    get composing() {
      return composing;
    },
    /**
     * @param {KeyEventLike} event
     * @param {number} t  ms(사건 시각)
     */
    blocks(event, t) {
      return composing || isComposingEvent(event) || t - endedAt < IME_GUARD_MS;
    },
  };
}
