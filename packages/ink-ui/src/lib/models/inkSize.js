// 주석 도구별 굵기·크기 로그 슬라이더, ±8% 자석 눈금, 실제 크기 미리보기, annotate.style 저장 모델(R2-ANN-2~4).
import { normalizeColorValue, pushRecentColor } from './palette.js';

export const SNAP_TOLERANCE = 0.08;
export const PREVIEW_BOX_MAX_PX = 38;
export const ANNOT_STYLE_MAX_BYTES = 2000;

/**
 * @typedef {'thin' | 'medium' | 'thick'} SizePresetKey
 * @typedef {{
 *   id: 'stroke' | 'marker' | 'eraser' | 'mosaic' | 'text',
 *   min: number,
 *   max: number,
 *   unit: 'px' | 'pt',
 *   defaultVal: number,
 *   presets: Record<SizePresetKey, number>,
 *   smallStep: number,
 *   largeStep: number,
 * }} ToolSizeSpec
 */

/** @type {Readonly<Record<'stroke' | 'marker' | 'eraser' | 'mosaic' | 'text', ToolSizeSpec>>} */
export const TOOL_SIZE_SPECS = Object.freeze({
  stroke: Object.freeze({
    id: 'stroke',
    min: 1,
    max: 32,
    unit: 'px',
    defaultVal: 4,
    presets: Object.freeze({ thin: 2, medium: 5, thick: 12 }),
    smallStep: 1,
    largeStep: 4,
  }),
  marker: Object.freeze({
    id: 'marker',
    min: 6,
    max: 48,
    unit: 'px',
    defaultVal: 16,
    presets: Object.freeze({ thin: 10, medium: 18, thick: 32 }),
    smallStep: 2,
    largeStep: 6,
  }),
  eraser: Object.freeze({
    id: 'eraser',
    min: 8,
    max: 64,
    unit: 'px',
    defaultVal: 20,
    presets: Object.freeze({ thin: 12, medium: 24, thick: 44 }),
    smallStep: 2,
    largeStep: 8,
  }),
  mosaic: Object.freeze({
    id: 'mosaic',
    min: 4,
    max: 48,
    unit: 'px',
    defaultVal: 10,
    presets: Object.freeze({ thin: 8, medium: 14, thick: 28 }),
    smallStep: 1,
    largeStep: 4,
  }),
  text: Object.freeze({
    id: 'text',
    min: 12,
    max: 120,
    unit: 'pt',
    defaultVal: 24,
    presets: Object.freeze({ thin: 18, medium: 32, thick: 64 }),
    smallStep: 2,
    largeStep: 8,
  }),
});

/** @type {readonly SizePresetKey[]} */
export const SIZE_PRESET_KEYS = Object.freeze(['thin', 'medium', 'thick']);

/**
 * 도구 id → 크기 규격.
 * @param {string | null | undefined} toolId
 * @returns {ToolSizeSpec}
 */
export function getToolSizeSpec(toolId) {
  if (toolId === 'marker') return TOOL_SIZE_SPECS.marker;
  if (toolId === 'eraser') return TOOL_SIZE_SPECS.eraser;
  if (toolId === 'mosaic') return TOOL_SIZE_SPECS.mosaic;
  if (toolId === 'text') return TOOL_SIZE_SPECS.text;
  return TOOL_SIZE_SPECS.stroke;
}

/**
 * 도구별 값을 범위 안 정수로 맞춘다.
 * @param {number} value
 * @param {ToolSizeSpec} spec
 * @returns {number}
 */
export function clampToolSize(value, spec) {
  if (!Number.isFinite(value)) return spec.defaultVal;
  return Math.max(spec.min, Math.min(spec.max, Math.round(value)));
}

/**
 * 값 → 로그 슬라이더 위치(0..1). 작은 값 구간일수록 촘촘하다.
 * @param {number} value
 * @param {ToolSizeSpec} spec
 * @returns {number}
 */
export function valueToSlider(value, spec) {
  const v = clampToolSize(value, spec);
  const ratio = spec.max / spec.min;
  if (ratio <= 1) return 0;
  return Math.max(0, Math.min(1, Math.log(v / spec.min) / Math.log(ratio)));
}

/**
 * 슬라이더 위치(0..1) → 값. `snap=true`면 가늘게·보통·굵게 눈금 ±8% 안에서 자석처럼 붙는다.
 * @param {number} t
 * @param {ToolSizeSpec} spec
 * @param {{ snap?: boolean, tolerance?: number }} [opts]
 * @returns {{ value: number, snapped: SizePresetKey | null, t: number }}
 */
export function sliderToValue(t, spec, { snap = true, tolerance = SNAP_TOLERANCE } = {}) {
  const ct = Number.isFinite(t) ? Math.max(0, Math.min(1, t)) : valueToSlider(spec.defaultVal, spec);
  if (snap) {
    /** @type {SizePresetKey | null} */
    let bestKey = null;
    let bestDist = Infinity;
    let bestPos = ct;
    for (const key of SIZE_PRESET_KEYS) {
      const pos = valueToSlider(spec.presets[key], spec);
      const dist = Math.abs(ct - pos);
      if (dist <= tolerance && dist < bestDist) {
        bestDist = dist;
        bestKey = key;
        bestPos = pos;
      }
    }
    if (bestKey) {
      return { value: spec.presets[bestKey], snapped: bestKey, t: bestPos };
    }
  }
  const raw = spec.min * Math.pow(spec.max / spec.min, ct);
  const value = clampToolSize(raw, spec);
  return { value, snapped: null, t: valueToSlider(value, spec) };
}

/**
 * `[` `]` / `⇧+[` `⇧+]` / 휠 한 단계 증감(로그 감각 + 최소 1단위 보장).
 * @param {number} current
 * @param {-1 | 1} dir
 * @param {ToolSizeSpec} spec
 * @param {{ large?: boolean }} [opts]
 * @returns {number}
 */
export function stepSize(current, dir, spec, { large = false } = {}) {
  const cur = clampToolSize(current, spec);
  const step = large ? spec.largeStep : Math.max(spec.smallStep, Math.round(cur * 0.12));
  return clampToolSize(cur + dir * step, spec);
}

/**
 * 값 숫자 표시(`6 px`, `24 pt`).
 * @param {number} value
 * @param {ToolSizeSpec} spec
 * @returns {string}
 */
export function formatSizeValue(value, spec) {
  return `${clampToolSize(value, spec)} ${spec.unit}`;
}

/**
 * 실제 화면 배율과 확대 배율을 반영한 미리보기 크기 계산(R2-ANN-3).
 * 칸(`maxPx`)을 넘으면 칸에 맞춰 줄이고 `실제의 n%` 비율을 돌려준다.
 * @param {{ value: number, displayScale?: number, zoomScale?: number, maxPx?: number }} opts
 * @returns {{ actualPx: number, drawPx: number, scalePercent: number, scaled: boolean }}
 */
export function computePreviewMetrics({ value, displayScale = 1, zoomScale = 1, maxPx = PREVIEW_BOX_MAX_PX }) {
  const ds = Number.isFinite(displayScale) && displayScale > 0 ? displayScale : 1;
  const zs = Number.isFinite(zoomScale) && zoomScale > 0 ? zoomScale : 1;
  const limit = Number.isFinite(maxPx) && maxPx > 0 ? maxPx : PREVIEW_BOX_MAX_PX;
  const raw = (Number.isFinite(value) && value > 0 ? value : 1) * ds * zs;
  const actualPx = Math.round(raw * 10) / 10;
  if (raw <= limit) {
    return { actualPx, drawPx: Math.max(1, raw), scalePercent: 100, scaled: false };
  }
  const scalePercent = Math.max(1, Math.round((limit / raw) * 100));
  return { actualPx, drawPx: limit, scalePercent, scaled: true };
}

// ─────────────── annotate.style JSON 상태(도구별 마지막 값 + 최근 색 5개) ───────────────

const STYLE_TOOLS = Object.freeze(['rect', 'ellipse', 'arrow', 'pen', 'marker', 'mosaic', 'text', 'eraser']);

/**
 * @typedef {{
 *   tools: Record<string, { color?: string, size: number }>,
 *   recentColors: string[],
 *   lastColor: string,
 * }} AnnotateStyleState
 */

/** @returns {AnnotateStyleState} */
export function createDefaultAnnotateStyle() {
  /** @type {Record<string, { color?: string, size: number }>} */
  const tools = {};
  for (const id of STYLE_TOOLS) {
    const spec = getToolSizeSpec(id);
    if (id === 'mosaic' || id === 'eraser') {
      tools[id] = { size: spec.defaultVal };
    } else {
      tools[id] = { color: 'R', size: spec.defaultVal };
    }
  }
  return { tools, recentColors: [], lastColor: 'R' };
}

/**
 * 설정 `annotate.style`(JSON 문자열 또는 객체)를 안전하게 파싱·정규화한다.
 * @param {unknown} raw
 * @returns {AnnotateStyleState}
 */
export function parseAnnotateStyle(raw) {
  const base = createDefaultAnnotateStyle();
  let obj = raw;
  if (typeof raw === 'string') {
    const trimmed = raw.trim();
    if (!trimmed) return base;
    try {
      obj = JSON.parse(trimmed);
    } catch {
      return base;
    }
  }
  if (!obj || typeof obj !== 'object') return base;
  const rec = /** @type {Record<string, any>} */ (obj);

  const lastColor = normalizeColorValue(rec.lastColor);
  if (lastColor) base.lastColor = lastColor;

  if (Array.isArray(rec.recentColors)) {
    let recent = /** @type {string[]} */ ([]);
    for (let i = rec.recentColors.length - 1; i >= 0; i--) {
      recent = pushRecentColor(recent, rec.recentColors[i]);
    }
    base.recentColors = recent;
  }

  if (rec.tools && typeof rec.tools === 'object') {
    for (const id of STYLE_TOOLS) {
      const entry = rec.tools[id];
      if (!entry || typeof entry !== 'object') continue;
      const spec = getToolSizeSpec(id);
      const size = typeof entry.size === 'number' ? clampToolSize(entry.size, spec) : spec.defaultVal;
      if (id === 'mosaic' || id === 'eraser') {
        base.tools[id] = { size };
      } else {
        const color = normalizeColorValue(entry.color) ?? base.lastColor;
        base.tools[id] = { color, size };
      }
    }
  }
  return base;
}

/**
 * `annotate.style` 설정 저장용 JSON 문자열(최대 2000바이트 안쪽 보장).
 * @param {AnnotateStyleState} style
 * @returns {string}
 */
export function serializeAnnotateStyle(style) {
  const clean = parseAnnotateStyle(style);
  const json = JSON.stringify(clean);
  if (json.length <= ANNOT_STYLE_MAX_BYTES) return json;
  const trimmed = { ...clean, recentColors: clean.recentColors.slice(0, 2) };
  return JSON.stringify(trimmed).slice(0, ANNOT_STYLE_MAX_BYTES);
}
