// 캡처 주석(4.9, R-CAP-28 · B §2.1) 순수 모델 — 판서 엔진(ink/inkEngine.js)을 고치지 않고 캡처 범위 위에서 쓰는 얇은 층.
//
// 좌표(B §2.1): 문서 원점 (0, 0) = 주석을 시작한 순간 범위의 왼쪽 위(anchor, 가상 화면 물리 px),
// 문서 단위 = 그 모니터 논리 px(= 오버레이 창 CSS px), 확대 없음(s = 1). 범위를 나중에 옮기거나 늘려도
// 주석은 화면 같은 자리에 남고 지금 범위로만 잘라 보인다(B §5 위험 1).
// 결과(B §2.1 "합성 결과 전달 계약"): 펜·도형·글자 = 투명 RGBA 층(물리 px, 범위 크기), 모자이크 = 사각형 목록 → Rust가 원본에 합성.

import { drawItem, resolveInkColor } from '../ink/canvasRender.js';
import { TEXT_LINE_HEIGHT } from '../ink/inkEngine.js';
import { withBBox } from '../ink/hitTest.js';
import { matchDrawKey } from '../ink/drawKeys.js';
import { physicalToClient } from './captureOverlay.js';
import {
  DEFAULT_FONT_FAMILY,
  DEFAULT_FONT_WEIGHT,
  formatCanvasFont,
  normalizeFontWeight,
  sanitizeFontName,
} from './fontPicker.js';
import { colorForKey } from './palette.js';

/** 모자이크 칸(논리 px): 기본 10, 범위 4~48(R2-ANN-2). */
export const MOSAIC_BLOCK_DEFAULT = 10;
export const MOSAIC_BLOCK_MIN = 4;
export const MOSAIC_BLOCK_MAX = 48;

/** 도구 한 글자 단축키(R·O·A·P·H·M·T·E). */
export const TOOL_KEY_CODES = Object.freeze({
  KeyR: 'rect',
  KeyO: 'ellipse',
  KeyA: 'arrow',
  KeyP: 'pen',
  KeyH: 'marker',
  KeyM: 'mosaic',
  KeyT: 'text',
  KeyE: 'eraser',
});
/** 이보다 작게 끈 모자이크·도형은 만들지 않는다(CSS px). */
export const MIN_DRAG_PX = 2;
/** 휠 한 단계로 볼 누적 deltaY(px) — 엔진 WHEEL_STEP_DELTA와 같게(맥 휠 한 칸 = 40 px). */
export const ANNOT_WHEEL_STEP = 40;

/**
 * @typedef {{ x: number, y: number, w: number, h: number }} PhysRect
 * @typedef {{ id: number, bounds: PhysRect, scale?: number }} MonitorLike
 * @typedef {{ x: number, y: number }} Point
 * @typedef {{ x: number, y: number, w: number, h: number, block: number }} MosaicRect
 */

/**
 * 도구 → 엔진에 줄 것. 도형은 엔진 손그림(pen) + 수정 키 도형(Ctrl = 사각형 · Ctrl+Shift = 화살표 · Tab = 타원),
 * 모자이크는 엔진 입력을 쓰지 않고 이 층이 mosaic 항목을 더한다.
 * @param {string | null} toolId
 * @returns {{ engineTool: 'pen' | 'marker' | 'text' | 'eraser' | null, shape: 'rect' | 'ellipse' | 'arrow' | null, mosaic: boolean }}
 */
export function annotTarget(toolId) {
  switch (toolId) {
    case 'rect':
    case 'ellipse':
    case 'arrow':
      return { engineTool: 'pen', shape: toolId, mosaic: false };
    case 'pen':
    case 'marker':
    case 'text':
    case 'eraser':
      return { engineTool: toolId, shape: null, mosaic: false };
    case 'mosaic':
      return { engineTool: null, shape: null, mosaic: true };
    default:
      return { engineTool: null, shape: null, mosaic: false };
  }
}

/**
 * 도형 도구면 수정 키를 그 도형으로 고정한 포인터 표본(Alt = 점선은 그대로). 펜·형광펜은 사용자 수정 키 그대로
 * (Shift = 직선 등 판서와 같은 습관). 타원(Tab)은 표본에 없어 `tab` 값으로 따로 돌려준다(엔진 mods.tab).
 * @template {{ shiftKey?: boolean, ctrlKey?: boolean, metaKey?: boolean, altKey?: boolean }} T
 * @param {T} sample
 * @param {string | null} toolId
 * @param {'windows' | 'macos'} [platform]
 * @returns {{ sample: T, tab: boolean }}
 */
export function applyToolMods(sample, toolId, platform = 'windows') {
  const { shape } = annotTarget(toolId);
  if (!shape) return { sample, tab: false };
  const primary = shape === 'rect' || shape === 'arrow';
  const mods = { shiftKey: shape === 'arrow', ctrlKey: false, metaKey: false };
  if (primary) {
    if (platform === 'macos') mods.metaKey = true;
    else mods.ctrlKey = true;
  }
  return { sample: { ...sample, ...mods }, tab: shape === 'ellipse' };
}

/**
 * 엔진 문서 옵션: 범위 왼쪽 위 = 원점, 크기 = 범위 / 배율(논리 px).
 * @param {number} session
 * @param {PhysRect} rect
 * @param {MonitorLike} monitor
 */
export function annotDocOptions(session, rect, monitor) {
  const scale = monitor.scale && monitor.scale > 0 ? monitor.scale : 1;
  return {
    id: `capture-${session}`,
    mode: /** @type {'capture'} */ ('capture'),
    monitorId: monitor.id,
    docSize: { w: Math.max(1, rect.w / scale), h: Math.max(1, rect.h / scale) },
    physSize: { w: Math.max(1, rect.w), h: Math.max(1, rect.h) },
    scale,
    frameId: session,
  };
}

/**
 * 판서 설정(state.settings)에 캡처 주석만의 값을 덮는다: 0.5초 멈춤 도형 제안은 끈다
 * (제안 수락 Enter가 캡처 복사 키와 겹침 — 도형은 도구로 고른다).
 * @param {Record<string, any> | null | undefined} settings
 */
export function annotEngineSettings(settings) {
  return { ...(settings ?? {}), 'draw.shapeOnHold': false };
}

/**
 * 문서 원점(anchor, 물리 px)이 이 창 CSS 좌표 어디인지.
 * @param {Point} anchor
 * @param {MonitorLike} monitor
 */
export function anchorToCss(anchor, monitor) {
  return physicalToClient(anchor.x, anchor.y, monitor);
}

/**
 * 범위 캔버스(왼쪽 위 = frame)에 문서를 그릴 보기 — s = 1, 원점은 anchor 자리.
 * @param {Point} anchorCss
 * @param {{ x: number, y: number }} frame  지금 범위(이 창 CSS)
 */
export function layerView(anchorCss, frame) {
  return { s: 1, tx: anchorCss.x - frame.x, ty: anchorCss.y - frame.y };
}

/**
 * 이 창 CSS 좌표 → 문서 좌표.
 * @param {number} cssX
 * @param {number} cssY
 * @param {Point} anchorCss
 * @returns {Point}
 */
export function cssToDoc(cssX, cssY, anchorCss) {
  return { x: cssX - anchorCss.x, y: cssY - anchorCss.y };
}

/** @param {number} n */
export function clampMosaicBlock(n) {
  if (!Number.isFinite(n)) return MOSAIC_BLOCK_DEFAULT;
  return Math.min(MOSAIC_BLOCK_MAX, Math.max(MOSAIC_BLOCK_MIN, Math.round(n)));
}

/**
 * 휠 누적: 한 단계(50 px)가 차면 방향(위 = +1)을 돌려주고 비운다. 방향이 바뀌면 처음부터.
 * @param {number} acc
 * @param {number} deltaY  px로 맞춘 값
 * @returns {{ acc: number, dir: -1 | 0 | 1 }}
 */
export function wheelStep(acc, deltaY) {
  let a = acc !== 0 && Math.sign(acc) !== Math.sign(deltaY) ? 0 : acc;
  a += deltaY;
  if (Math.abs(a) < ANNOT_WHEEL_STEP) return { acc: a, dir: 0 };
  return { acc: 0, dir: a < 0 ? 1 : -1 };
}

/**
 * 두 점(문서)으로 모자이크 항목. 너무 작으면 null.
 * @param {Point} a
 * @param {Point} b
 * @param {number} blockSize  논리 px
 * @param {string} id
 * @returns {import('../ink/inkEngine.js').MosaicItem | null}
 */
export function mosaicItem(a, b, blockSize, id) {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  const w = Math.abs(b.x - a.x);
  const h = Math.abs(b.y - a.y);
  if (w < MIN_DRAG_PX || h < MIN_DRAG_PX) return null;
  return /** @type {import('../ink/inkEngine.js').MosaicItem} */ (withBBox({ id, kind: 'mosaic', x, y, w, h, blockSize: clampMosaicBlock(blockSize) }));
}

/**
 * 모자이크 항목(문서) → 결과 그림 기준 물리 px 사각형(왼쪽 위 = 범위 왼쪽 위). 그림 밖은 Rust가 버린다.
 * @param {{ x: number, y: number, w: number, h: number, blockSize: number }} item
 * @param {Point} anchor
 * @param {PhysRect} rect
 * @param {number} scale
 * @returns {MosaicRect}
 */
export function mosaicToPhys(item, anchor, rect, scale) {
  const ox = anchor.x - rect.x;
  const oy = anchor.y - rect.y;
  const x0 = Math.round(ox + item.x * scale);
  const y0 = Math.round(oy + item.y * scale);
  const x1 = Math.round(ox + (item.x + item.w) * scale);
  const y1 = Math.round(oy + (item.y + item.h) * scale);
  return { x: x0, y: y0, w: Math.max(0, x1 - x0), h: Math.max(0, y1 - y0), block: Math.max(1, Math.round(item.blockSize * scale)) };
}

/**
 * 결과 층 변환(문서 → 범위 물리 px): [a, b, c, d, e, f].
 * @param {Point} anchor
 * @param {PhysRect} rect
 * @param {number} scale
 */
export function exportTransform(anchor, rect, scale) {
  return [scale, 0, 0, scale, anchor.x - rect.x, anchor.y - rect.y];
}

/**
 * 글자 항목에서 굵기·폴백 스택까지 포함해 캔버스에 그린다.
 * @param {import('../ink/canvasRender.js').Ctx2D} ctx
 * @param {any} item
 * @param {string} fallbackFamily
 */
function drawBakedTextItem(ctx, item, fallbackFamily) {
  ctx.save();
  ctx.globalAlpha = 1;
  ctx.globalCompositeOperation = 'source-over';
  ctx.fillStyle = resolveInkColor(item.color, 'frame');
  const family = sanitizeFontName(item.fontFamily) || fallbackFamily;
  const weight = normalizeFontWeight(item.fontWeight ?? DEFAULT_FONT_WEIGHT);
  ctx.font = formatCanvasFont(item.fontSize, family, weight);
  ctx.textBaseline = 'top';
  const lh = item.fontSize * TEXT_LINE_HEIGHT;
  const lead = (lh - item.fontSize) / 2;
  const lines = String(item.text ?? '').split('\n');
  for (let i = 0; i < lines.length; i++) ctx.fillText(lines[i], item.x, item.y + i * lh + lead);
  ctx.restore();
}

/**
 * 주석 층에 들어 있는 글자 항목들의 고유 글꼴 목록을 뽑는다.
 * @param {ReadonlyArray<import('../ink/inkEngine.js').Item>} items
 * @param {string} [fallbackFamily]
 * @returns {Array<{ fontFamily: string, fontWeight: number, fontSize: number, sampleText: string }>}
 */
export function collectTextFonts(items, fallbackFamily = DEFAULT_FONT_FAMILY) {
  /** @type {Map<string, { fontFamily: string, fontWeight: number, fontSize: number, sampleText: string }>} */
  const map = new Map();
  for (const it of items) {
    if (!it || it.kind !== 'text') continue;
    const anyIt = /** @type {any} */ (it);
    const fontFamily = sanitizeFontName(anyIt.fontFamily) || fallbackFamily;
    const fontWeight = normalizeFontWeight(anyIt.fontWeight ?? DEFAULT_FONT_WEIGHT);
    const fontSize = Number.isFinite(anyIt.fontSize) && anyIt.fontSize > 0 ? anyIt.fontSize : 24;
    const key = `${fontFamily.toLowerCase()}|${fontWeight}`;
    if (!map.has(key)) {
      map.set(key, {
        fontFamily,
        fontWeight,
        fontSize,
        sampleText: String(anyIt.text || '가A1').slice(0, 16) || '가A1',
      });
    }
  }
  return [...map.values()];
}

/**
 * 저장·복사 이미지 굽기 전 `document.fonts.load()` 완료를 기다린다(최대 timeoutMs).
 * 불러오지 못한 글꼴은 기본 글꼴로 대체한 항목 목록과 실패 목록을 돌려준다.
 * @param {ReadonlyArray<import('../ink/inkEngine.js').Item>} items
 * @param {{
 *   fonts?: { load?: (font: string, text?: string) => Promise<unknown>, ready?: Promise<unknown> } | null,
 *   timeoutMs?: number,
 *   fallbackFamily?: string,
 * }} [opts]
 */
export async function ensureAnnotationFontsLoaded(items, opts = {}) {
  const fallbackFamily = opts.fallbackFamily ?? DEFAULT_FONT_FAMILY;
  const timeoutMs = typeof opts.timeoutMs === 'number' && opts.timeoutMs > 0 ? opts.timeoutMs : 500;
  const fonts = opts.fonts !== undefined ? opts.fonts : typeof document !== 'undefined' ? document.fonts : null;
  const specs = collectTextFonts(items, fallbackFamily);
  if (!specs.length || !fonts || typeof fonts.load !== 'function') {
    return { ok: true, loaded: /** @type {string[]} */ ([]), failed: /** @type {string[]} */ ([]), items: items.slice() };
  }

  /** @type {string[]} */
  const loaded = [];
  /** @type {string[]} */
  const failed = [];
  const failedKeys = new Set();

  const waitTimeout = (/** @type {Promise<unknown>} */ p) =>
    Promise.race([
      p.then(() => ({ timedOut: false })),
      new Promise((resolve) => setTimeout(() => resolve({ timedOut: true }), timeoutMs)),
    ]);

  await Promise.all(
    specs.map(async (spec) => {
      const desc = formatCanvasFont(spec.fontSize, spec.fontFamily, spec.fontWeight);
      const key = `${spec.fontFamily.toLowerCase()}|${spec.fontWeight}`;
      try {
        const res = /** @type {{ timedOut?: boolean }} */ (await waitTimeout(Promise.resolve(fonts.load(desc, spec.sampleText))));
        if (res?.timedOut) {
          failed.push(spec.fontFamily);
          failedKeys.add(key);
        } else {
          loaded.push(spec.fontFamily);
        }
      } catch {
        failed.push(spec.fontFamily);
        failedKeys.add(key);
      }
    }),
  );

  if (fonts.ready) {
    try {
      await waitTimeout(Promise.resolve(fonts.ready));
    } catch {
      // ignore
    }
  }

  if (failedKeys.size === 0) {
    return { ok: true, loaded, failed, items: items.slice() };
  }

  const normalizedItems = items.map((it) => {
    if (!it || it.kind !== 'text') return it;
    const anyIt = /** @type {any} */ (it);
    const fam = sanitizeFontName(anyIt.fontFamily) || fallbackFamily;
    const w = normalizeFontWeight(anyIt.fontWeight ?? DEFAULT_FONT_WEIGHT);
    if (!failedKeys.has(`${fam.toLowerCase()}|${w}`)) return it;
    return /** @type {import('../ink/inkEngine.js').Item} */ ({
      ...anyIt,
      fontFamily: fallbackFamily,
      fontWeight: DEFAULT_FONT_WEIGHT,
    });
  });

  return { ok: false, loaded, failed: [...new Set(failed)], items: normalizedItems };
}

/**
 * 주석 층 그리기(투명 캔버스 ctx, 범위 물리 크기). 항목 순서대로 그리되 모자이크는 **그 자리를 비운다** —
 * 먼저 그린 획 위에 모자이크를 덮으면 결과에서도 획이 모자이크 아래로 사라지게(Rust: 원본 모자이크 → 이 층 합성).
 * 화면 미리보기(canvasRender)와 같은 source-over·같은 알파(밝은 바탕 multiply 없음).
 * @param {import('../ink/canvasRender.js').Ctx2D} ctx
 * @param {ReadonlyArray<import('../ink/inkEngine.js').Item>} items
 * @param {{ anchor: Point, rect: PhysRect, scale: number, defaultFontFamily?: string }} o
 * @returns {{ mosaics: MosaicRect[], painted: number }}
 */
export function renderAnnotationLayer(ctx, items, o) {
  const [a, b, c, d, e, f] = exportTransform(o.anchor, o.rect, o.scale);
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, o.rect.w, o.rect.h);
  ctx.setTransform(a, b, c, d, e, f);
  /** @type {MosaicRect[]} */
  const mosaics = [];
  let painted = 0;
  const fallback = o.defaultFontFamily || DEFAULT_FONT_FAMILY;
  for (const it of items) {
    if (it.kind === 'mosaic') {
      mosaics.push(mosaicToPhys(it, o.anchor, o.rect, o.scale));
      ctx.clearRect(it.x, it.y, it.w, it.h);
      continue;
    }
    if (it.kind === 'text') {
      drawBakedTextItem(ctx, it, fallback);
    } else {
      drawItem(ctx, it, { background: 'frame' });
    }
    painted++;
  }
  return { mosaics, painted };
}

/**
 * 결과로 보낼 주석: 모자이크 목록 + (획·도형·글자가 있으면) 투명 RGBA 층.
 * @param {ReadonlyArray<import('../ink/inkEngine.js').Item>} items
 * @param {{ anchor: Point, rect: PhysRect, scale: number, defaultFontFamily?: string, createCanvas: (w: number, h: number) => any }} o
 * @returns {{ w: number, h: number, rgba: Uint8ClampedArray | null, mosaics: MosaicRect[] }}
 */
export function exportAnnotation(items, o) {
  const w = Math.max(0, Math.round(o.rect.w));
  const h = Math.max(0, Math.round(o.rect.h));
  const hasPaint = items.some((it) => it.kind !== 'mosaic');
  if (!hasPaint || w === 0 || h === 0) {
    return { w: 0, h: 0, rgba: null, mosaics: items.filter((it) => it.kind === 'mosaic').map((it) => mosaicToPhys(/** @type {any} */ (it), o.anchor, o.rect, o.scale)) };
  }
  const canvas = o.createCanvas(w, h);
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  const { mosaics } = renderAnnotationLayer(ctx, items, o);
  const rgba = ctx.getImageData(0, 0, w, h).data;
  // 캔버스 메모리는 바로 돌려준다
  canvas.width = 0;
  canvas.height = 0;
  return { w, h, rgba, mosaics };
}

/**
 * 글꼴 불러오기를 기다린 뒤 주석 층을 내보낸다(R2-TXT-3).
 * @param {ReadonlyArray<import('../ink/inkEngine.js').Item>} items
 * @param {{
 *   anchor: Point,
 *   rect: PhysRect,
 *   scale: number,
 *   defaultFontFamily?: string,
 *   createCanvas: (w: number, h: number) => any,
 *   fonts?: { load?: (font: string, text?: string) => Promise<unknown>, ready?: Promise<unknown> } | null,
 *   timeoutMs?: number,
 * }} o
 */
export async function exportAnnotationAsync(items, o) {
  const res = await ensureAnnotationFontsLoaded(items, {
    fonts: o.fonts,
    timeoutMs: o.timeoutMs,
    fallbackFamily: o.defaultFontFamily || DEFAULT_FONT_FAMILY,
  });
  const out = exportAnnotation(res.items, o);
  return {
    ...out,
    fontFallback: !res.ok,
    failedFonts: res.failed,
  };
}

/**
 * 모자이크 미리보기 원본: 이 모니터 정지 이미지를 문서 원점(anchor)에서 시작하게 옮긴 캔버스 계획(물리 px).
 * drawMosaic은 문서 좌표 × imageScale(= 배율)로 이 캔버스를 읽는다. anchor가 모니터 오른쪽·아래 밖이면 null.
 * @param {Point} anchor
 * @param {MonitorLike} monitor
 * @returns {{ w: number, h: number, dx: number, dy: number, dw: number, dh: number } | null}
 */
export function mosaicSourcePlan(anchor, monitor) {
  const b = monitor.bounds;
  const w = b.x + b.w - anchor.x;
  const h = b.y + b.h - anchor.y;
  if (w <= 0 || h <= 0) return null;
  return { w, h, dx: b.x - anchor.x, dy: b.y - anchor.y, dw: b.w, dh: b.h };
}

const ARROWS = new Set(['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown']);

/**
 * 주석이 있는 동안 키를 어디로: 글자 입력 중 = 입력 상자 · 도구를 고른 동안 Esc = 도구만 해제(주석 유지, 메인 결정) ·
 * 실행 취소/다시 실행(d-undo·d-redo 설정 키) = 엔진 · 도구를 고른 동안 방향키 = 삼킴(범위 조정은 도구를 해제한 뒤) ·
 * 나머지 = 캡처(Rust가 Esc 취소·Enter·Ctrl+S 판정 — 도구가 없을 때 Esc는 캡처 취소).
 * @param {import('../ink/drawKeys.js').KeyEventLike} event
 * @param {{ editing: boolean, toolActive: boolean, keymap: Map<string, string> | null, platform?: 'windows' | 'macos' }} o
 * @returns {'text' | 'release' | 'undo' | 'redo' | 'swallow' | 'capture'}
 */
export function annotKeyRoute(event, o) {
  if (o.editing) return 'text';
  const esc = event.code === 'Escape' || event.key === 'Escape';
  if (o.toolActive && esc && !event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey && !event.isComposing) return 'release';
  const id = o.keymap ? matchDrawKey(event, o.keymap, o.platform ?? 'windows') : null;
  if (id === 'd-undo') return 'undo';
  if (id === 'd-redo') return 'redo';
  if (o.toolActive && event.code && ARROWS.has(event.code)) return 'swallow';
  return 'capture';
}

/**
 * 캡처 확정 후 도구·색·굵기·스포이트 키보드 단축키 판정(R2-UI-TB-3 · R2-ANN-4).
 * - 패널이 열려 있으면 기본 8색(R·Y·G·O·B·L·K·W)을 먼저 받는다.
 * - 확정 상태에서는 R·O·A·P·H·M·T·E로 도구를 고르고, 도구가 켜진 중에는 `[` `]`(⇧ 큰 단계)·비충돌 색 키(Y·G·B·L·K·W)를 받는다.
 * @param {import('../ink/drawKeys.js').KeyEventLike} event
 * @param {{ editing?: boolean, eyedropper?: boolean, confirmed?: boolean, toolActive?: boolean, panelOpen?: boolean }} o
 * @returns {{ type: 'text' | 'commitText' | 'cancelEyedropper' } | { type: 'tool', id: string } | { type: 'color', code: string } | { type: 'size', dir: -1 | 1, large: boolean } | null}
 */
export function matchCaptureAnnotKey(event, o) {
  if (!event || event.isComposing) return null;
  const hasPrimaryMod = Boolean(event.ctrlKey || event.metaKey || event.altKey);
  const esc = event.code === 'Escape' || event.key === 'Escape';
  if (o.editing) {
    if (esc && !hasPrimaryMod && !event.shiftKey) return { type: 'commitText' };
    return { type: 'text' };
  }
  if (hasPrimaryMod) return null;

  if (o.eyedropper && esc && !event.shiftKey) return { type: 'cancelEyedropper' };

  const code = event.code || '';
  if (o.toolActive && (code === 'BracketLeft' || code === 'BracketRight')) {
    return { type: 'size', dir: code === 'BracketLeft' ? -1 : 1, large: Boolean(event.shiftKey) };
  }

  if (event.shiftKey) return null;

  if (o.panelOpen) {
    const c = colorForKey(code);
    if (c) return { type: 'color', code: c };
  }

  if (o.confirmed && code in TOOL_KEY_CODES) {
    return { type: 'tool', id: /** @type {Record<string, string>} */ (TOOL_KEY_CODES)[code] };
  }

  if (o.toolActive) {
    const c = colorForKey(code);
    if (c) return { type: 'color', code: c };
  }

  return null;
}
