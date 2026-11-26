// 주석·판서 팔레트 순수 모델(R2-ANN-1): 기본 8색(키 유지) + 넓힌 8색 + 내 색(HEX)·스포이트·최근 5색.

export const MAX_RECENT_COLORS = 5;
export const MARKER_ALPHA = 0.38;

/**
 * @typedef {{
 *   code: string,
 *   hex: string,
 *   labelKey: string,
 *   key?: string,
 *   lightFg?: boolean,
 *   needsBorder?: boolean,
 *   group: 'basic' | 'extended',
 * }} PaletteSwatch
 */

/** 기본 8색(기존 판서 키 R·Y·G·O·B·L + K·W 그대로 유지). @type {readonly PaletteSwatch[]} */
export const BASIC_COLORS = Object.freeze([
  Object.freeze({ code: 'R', hex: '#E5484D', labelKey: 'capture.color.R', key: 'R', lightFg: true, group: 'basic' }),
  Object.freeze({ code: 'Y', hex: '#F2B705', labelKey: 'capture.color.Y', key: 'Y', lightFg: false, group: 'basic' }),
  Object.freeze({ code: 'G', hex: '#2BA36B', labelKey: 'capture.color.G', key: 'G', lightFg: true, group: 'basic' }),
  Object.freeze({ code: 'O', hex: '#F97316', labelKey: 'capture.color.O', key: 'O', lightFg: false, group: 'basic' }),
  Object.freeze({ code: 'B', hex: '#2F6FED', labelKey: 'capture.color.B', key: 'B', lightFg: true, group: 'basic' }),
  Object.freeze({ code: 'L', hex: '#84CC16', labelKey: 'capture.color.L', key: 'L', lightFg: false, group: 'basic' }),
  Object.freeze({ code: 'K', hex: '#1A1A1A', labelKey: 'capture.color.K', key: 'K', lightFg: true, needsBorder: true, group: 'basic' }),
  Object.freeze({ code: 'W', hex: '#FFFFFF', labelKey: 'capture.color.W', key: 'W', lightFg: false, needsBorder: true, group: 'basic' }),
]);

/** 넓힌 8색(남색·보라·분홍·청록·갈색·회색·연노랑·연하늘). @type {readonly PaletteSwatch[]} */
export const EXTENDED_COLORS = Object.freeze([
  Object.freeze({ code: 'NV', hex: '#1E3A8A', labelKey: 'capture.color.NV', lightFg: true, group: 'extended' }),
  Object.freeze({ code: 'PU', hex: '#8B5CF6', labelKey: 'capture.color.PU', lightFg: true, group: 'extended' }),
  Object.freeze({ code: 'PK', hex: '#EC4899', labelKey: 'capture.color.PK', lightFg: true, group: 'extended' }),
  Object.freeze({ code: 'TL', hex: '#06B6D4', labelKey: 'capture.color.TL', lightFg: false, group: 'extended' }),
  Object.freeze({ code: 'BR', hex: '#92400E', labelKey: 'capture.color.BR', lightFg: true, group: 'extended' }),
  Object.freeze({ code: 'GR', hex: '#6B7280', labelKey: 'capture.color.GR', lightFg: true, group: 'extended' }),
  Object.freeze({ code: 'LY', hex: '#FEF08A', labelKey: 'capture.color.LY', lightFg: false, needsBorder: true, group: 'extended' }),
  Object.freeze({ code: 'SB', hex: '#7DD3FC', labelKey: 'capture.color.SB', lightFg: false, group: 'extended' }),
]);

/** 전체 16색 목록. @type {readonly PaletteSwatch[]} */
export const PALETTE_COLORS = Object.freeze([...BASIC_COLORS, ...EXTENDED_COLORS]);

/**
 * 판서 엔진·렌더러 공용 색 표(`inkEngine.js`가 그대로 재수출).
 * 기존 8키(`R,Y,G,O,B,L,W,K`)의 HEX를 절대 바꾸지 않는다.
 */
export const INK_COLORS = Object.freeze({
  R: '#E5484D',
  Y: '#F2B705',
  G: '#2BA36B',
  O: '#F97316',
  B: '#2F6FED',
  L: '#84CC16',
  W: '#FFFFFF',
  K: '#1A1A1A',
  NV: '#1E3A8A',
  PU: '#8B5CF6',
  PK: '#EC4899',
  TL: '#06B6D4',
  BR: '#92400E',
  GR: '#6B7280',
  LY: '#FEF08A',
  SB: '#7DD3FC',
});

export const BASIC_COLOR_CODES = Object.freeze(BASIC_COLORS.map((c) => c.code));
export const INK_COLOR_CODES = Object.freeze(Object.keys(INK_COLORS));

const HEX_6_RE = /^#([0-9a-fA-F]{6})$/;
const HEX_3_RE = /^#([0-9a-fA-F]{3})$/;

/**
 * `#RRGGBB` 또는 `#RGB` 형식인지 검사한다.
 * @param {unknown} v
 * @returns {boolean}
 */
export function isHexColor(v) {
  if (typeof v !== 'string') return false;
  const s = v.trim();
  return HEX_6_RE.test(s) || HEX_3_RE.test(s);
}

/**
 * HEX 문자열을 대문자 `#RRGGBB`로 정규화한다. 유효하지 않으면 null.
 * @param {unknown} v
 * @returns {string | null}
 */
export function normalizeHex(v) {
  if (typeof v !== 'string') return null;
  const s = v.trim();
  const m6 = s.match(HEX_6_RE);
  if (m6) return `#${m6[1].toUpperCase()}`;
  const m3 = s.match(HEX_3_RE);
  if (m3) {
    const [r, g, b] = m3[1];
    return `#${(r + r + g + g + b + b).toUpperCase()}`;
  }
  return null;
}

/**
 * RGB 바이트 3개 → `#RRGGBB`.
 * @param {[number, number, number] | number[]} rgb
 * @returns {string}
 */
export function rgbToHex(rgb) {
  const clamp = (n) => Math.max(0, Math.min(255, Math.round(Number.isFinite(n) ? n : 0)));
  const r = clamp(rgb?.[0] ?? 0).toString(16).padStart(2, '0');
  const g = clamp(rgb?.[1] ?? 0).toString(16).padStart(2, '0');
  const b = clamp(rgb?.[2] ?? 0).toString(16).padStart(2, '0');
  return `#${(r + g + b).toUpperCase()}`;
}

/**
 * 색 코드(`R`, `NV` 등) 또는 `#RRGGBB` → 실제 HEX 색. 모르면 빨강(`#E5484D`).
 * @param {string | null | undefined} codeOrHex
 * @returns {string}
 */
export function resolvePaletteHex(codeOrHex) {
  if (!codeOrHex) return INK_COLORS.R;
  const norm = normalizeHex(codeOrHex);
  if (norm) return norm;
  return /** @type {Record<string, string>} */ (INK_COLORS)[codeOrHex] ?? INK_COLORS.R;
}

/**
 * 유효한 색 값(팔레트 코드 또는 HEX)인지 확인하고 정규화된 값을 돌려준다.
 * @param {unknown} v
 * @returns {string | null}
 */
export function normalizeColorValue(v) {
  if (typeof v !== 'string') return null;
  const hex = normalizeHex(v);
  if (hex) return hex;
  return Object.prototype.hasOwnProperty.call(INK_COLORS, v) ? v : null;
}

/**
 * 형광펜 미리보기용 반투명 CSS 색(`rgba(r, g, b, alpha)`).
 * @param {string | null | undefined} codeOrHex
 * @param {number} [alpha=MARKER_ALPHA]
 * @returns {string}
 */
export function markerPreviewColor(codeOrHex, alpha = MARKER_ALPHA) {
  const hex = resolvePaletteHex(codeOrHex);
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/**
 * 밝은 색인지 판정(체크/테두리 대비 보정용).
 * @param {string | null | undefined} codeOrHex
 * @returns {boolean}
 */
export function isLightColor(codeOrHex) {
  const hex = resolvePaletteHex(codeOrHex);
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  return lum > 0.65;
}

const KEY_TO_CODE = new Map([
  ['KeyR', 'R'],
  ['KeyY', 'Y'],
  ['KeyG', 'G'],
  ['KeyO', 'O'],
  ['KeyB', 'B'],
  ['KeyL', 'L'],
  ['KeyK', 'K'],
  ['KeyW', 'W'],
]);

/**
 * 물리 키 코드 → 기본 8색 코드.
 * @param {string | null | undefined} code
 * @returns {string | null}
 */
export function colorForKey(code) {
  return code ? KEY_TO_CODE.get(code) ?? null : null;
}

/**
 * 최근 내 색 목록에 새 HEX를 맨 앞에 넣고 중복 제거·최대 5개 유지.
 * @param {ReadonlyArray<string> | null | undefined} recent
 * @param {string} hex
 * @returns {string[]}
 */
export function pushRecentColor(recent, hex) {
  const norm = normalizeHex(hex);
  if (!norm) return Array.isArray(recent) ? recent.slice(0, MAX_RECENT_COLORS) : [];
  const next = [norm];
  if (Array.isArray(recent)) {
    for (const item of recent) {
      const c = normalizeHex(item);
      if (c && c !== norm) next.push(c);
      if (next.length >= MAX_RECENT_COLORS) break;
    }
  }
  return next;
}
