// 판서 2D Canvas 그리기(A §3.5) — 2D 컨텍스트를 인자로 받는 순수 함수. DOM 조회·전역 상태 없음.
// 0층 배경 · 1층 굳힌 판서(전체/1획 증분) · 2층 지금 획(dirty rect만 지우고 다시) · 내보내기 합성(물리 해상도).
// 검은 칠판에서 검정(K) 펜은 흰색으로 치환해 보인다(04 S14). 형광펜은 alpha 0.38(검은 칠판 0.45),
// 배경과 같은 캔버스에 합성할 때만(내보내기) multiply.

import { INK_COLORS, TEXT_LINE_HEIGHT } from './inkEngine.js';
import { arrowHeadPoints, itemBBox, unionRect, ARROW_HEAD_DEG } from './hitTest.js';
import { docRectToView } from './coords.js';

/**
 * @typedef {import('./inkEngine.js').Item} Item
 * @typedef {import('./inkEngine.js').StrokeItem} StrokeItem
 * @typedef {import('./inkEngine.js').ShapeItem} ShapeItem
 * @typedef {import('./inkEngine.js').TextItem} TextItem
 * @typedef {import('./inkEngine.js').MosaicItem} MosaicItem
 * @typedef {import('./inkEngine.js').InkBackground} InkBackground
 * @typedef {import('./inkEngine.js').InkDocument} InkDocument
 * @typedef {import('./coords.js').ViewTransform} ViewTransform
 * @typedef {{ x: number, y: number, w: number, h: number }} Rect
 *
 * 쓰는 2D 컨텍스트 부분(CanvasRenderingContext2D·OffscreenCanvasRenderingContext2D·시험용 기록기 모두).
 * @typedef {Object} Ctx2D
 * @property {(a: number, b: number, c: number, d: number, e: number, f: number) => void} setTransform
 * @property {(x: number, y: number, w: number, h: number) => void} clearRect
 * @property {(x: number, y: number, w: number, h: number) => void} fillRect
 * @property {(x: number, y: number, w: number, h: number) => void} strokeRect
 * @property {() => void} beginPath
 * @property {() => void} closePath
 * @property {(x: number, y: number) => void} moveTo
 * @property {(x: number, y: number) => void} lineTo
 * @property {(cx: number, cy: number, x: number, y: number) => void} quadraticCurveTo
 * @property {(x: number, y: number, rx: number, ry: number, rot: number, a0: number, a1: number) => void} ellipse
 * @property {() => void} stroke
 * @property {() => void} fill
 * @property {(text: string, x: number, y: number) => void} fillText
 * @property {(text: string) => { width: number }} measureText
 * @property {(segments: number[]) => void} setLineDash
 * @property {(...args: any[]) => void} drawImage
 * @property {() => void} save
 * @property {() => void} restore
 * @property {any} strokeStyle
 * @property {any} fillStyle
 * @property {number} lineWidth
 * @property {string} lineCap
 * @property {string} lineJoin
 * @property {number} globalAlpha
 * @property {string} globalCompositeOperation
 * @property {string} font
 * @property {string} textBaseline
 * @property {boolean} imageSmoothingEnabled
 *
 * @typedef {Object} DrawOptions
 * @property {InkBackground} background
 * @property {boolean} [composite]   배경과 같은 캔버스(내보내기) — 밝은 바탕 형광펜 multiply
 * @property {CanvasImageSource | null} [image]  모자이크 원본(배경 정지 이미지)
 * @property {number} [imageScale]   배경 이미지 px / 문서 px
 * @property {(w: number, h: number) => any} [createCanvas]  모자이크 축소 버퍼
 */

/** 칠판 바탕 색. */
export const BOARD_COLORS = Object.freeze({ white: '#FFFFFF', black: '#121212' });
export const MARKER_OPACITY_ON_BLACK = 0.45;
/** 고르기 테두리·핸들(--ink-accent). */
export const SELECTION_COLOR = '#FF9F0A';
export const HANDLE_SIZE_PX = 8;
/** 빈 정지 이미지 자리(이미지가 아직 없을 때). */
export const FRAME_PLACEHOLDER = '#2A2B2E';

/**
 * 표시 색: 검은 칠판에서 K(검정) → 흰색.
 * @param {string} code
 * @param {InkBackground} background
 */
export function resolveInkColor(code, background) {
  if (background === 'black' && code === 'K') return INK_COLORS.W;
  return /** @type {Record<string, string>} */ (INK_COLORS)[code] ?? INK_COLORS.R;
}

/**
 * 문서 좌표로 그리게 변환(dpr × 보기).
 * @param {Ctx2D} ctx
 * @param {ViewTransform} view
 * @param {number} dpr
 */
export function setViewTransform(ctx, view, dpr) {
  ctx.setTransform(dpr * view.s, 0, 0, dpr * view.s, dpr * view.tx, dpr * view.ty);
}

/**
 * 캔버스 전체 지우기.
 * @param {Ctx2D} ctx
 * @param {number} w  백버퍼 px
 * @param {number} h
 */
export function clearCanvas(ctx, w, h) {
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, w, h);
}

/**
 * 0층 배경: 정지 화면(frame) 또는 흰·검은 칠판.
 * @param {Ctx2D} ctx
 * @param {{ background: InkBackground, image?: CanvasImageSource | null, view: ViewTransform, dpr: number, docSize: { w: number, h: number }, canvasW: number, canvasH: number }} o
 */
export function drawBackground(ctx, o) {
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  if (o.background !== 'frame') {
    ctx.fillStyle = BOARD_COLORS[o.background];
    ctx.fillRect(0, 0, o.canvasW, o.canvasH);
    return;
  }
  ctx.fillStyle = FRAME_PLACEHOLDER;
  ctx.fillRect(0, 0, o.canvasW, o.canvasH);
  if (!o.image) return;
  setViewTransform(ctx, o.view, o.dpr);
  ctx.imageSmoothingEnabled = true;
  ctx.drawImage(o.image, 0, 0, o.docSize.w, o.docSize.h);
}

/**
 * 필압 → 굵기 배수(0.6 ~ 1.4, 마우스 0.5 → 1).
 * @param {number} p
 */
export function pressureMul(p) {
  return 0.6 + 0.8 * (p < 0 ? 0 : p > 1 ? 1 : p);
}

/**
 * 2차 베지어 중점 연결 경로.
 * @param {Ctx2D} ctx
 * @param {ReadonlyArray<{ x: number, y: number }>} pts
 */
function tracePath(ctx, pts) {
  const n = pts.length;
  ctx.moveTo(pts[0].x, pts[0].y);
  if (n === 1) {
    ctx.lineTo(pts[0].x + 0.01, pts[0].y); // 둥근 끝으로 점
    return;
  }
  if (n === 2) {
    ctx.lineTo(pts[1].x, pts[1].y);
    return;
  }
  for (let i = 1; i < n - 1; i++) {
    const mx = (pts[i].x + pts[i + 1].x) / 2;
    const my = (pts[i].y + pts[i + 1].y) / 2;
    ctx.quadraticCurveTo(pts[i].x, pts[i].y, mx, my);
  }
  ctx.lineTo(pts[n - 1].x, pts[n - 1].y);
}

/**
 * 투명도·합성 방식.
 * @param {Ctx2D} ctx
 * @param {{ opacity: number, tool?: string }} item
 * @param {DrawOptions} o
 */
function applyAlpha(ctx, item, o) {
  const marker = item.tool === 'marker' || item.opacity < 1;
  if (!marker) {
    ctx.globalAlpha = 1;
    ctx.globalCompositeOperation = 'source-over';
    return;
  }
  if (o.background === 'black') {
    ctx.globalAlpha = MARKER_OPACITY_ON_BLACK;
    ctx.globalCompositeOperation = 'source-over';
  } else {
    ctx.globalAlpha = item.opacity;
    ctx.globalCompositeOperation = o.composite ? 'multiply' : 'source-over';
  }
}

/**
 * @param {Ctx2D} ctx
 * @param {number} width
 * @param {boolean} dashed
 */
function applyDash(ctx, width, dashed) {
  ctx.setLineDash(dashed ? [width * 2.5, width * 2] : []);
}

/**
 * 손획.
 * @param {Ctx2D} ctx
 * @param {StrokeItem} item
 * @param {DrawOptions} o
 */
export function drawStroke(ctx, item, o) {
  const pts = item.points;
  if (!pts.length) return;
  const color = resolveInkColor(item.color, o.background);
  const marker = item.tool === 'marker';
  applyAlpha(ctx, item, o);
  ctx.strokeStyle = color;
  ctx.lineJoin = 'round';
  ctx.lineCap = marker ? (pts.length === 1 ? 'square' : 'butt') : 'round';
  applyDash(ctx, item.width, item.dashed);
  const p0 = pts[0].p ?? 0.5;
  const varying = !marker && !item.dashed && pts.some((p) => Math.abs((p.p ?? 0.5) - p0) > 0.02);
  if (!varying) {
    ctx.lineWidth = item.width * (marker ? 1 : pressureMul(p0));
    ctx.beginPath();
    tracePath(ctx, pts);
    ctx.stroke();
    return;
  }
  // 필압이 바뀌는 펜: 구간마다 굵기(둥근 끝이 이음새를 메움)
  for (let i = 1; i < pts.length; i++) {
    const a = pts[i - 1];
    const b = pts[i];
    ctx.lineWidth = item.width * pressureMul(((a.p ?? 0.5) + (b.p ?? 0.5)) / 2);
    ctx.beginPath();
    ctx.moveTo(a.x, a.y);
    ctx.lineTo(b.x, b.y);
    ctx.stroke();
  }
}

/**
 * 도형: 직선 · 화살표(채운 머리) · 사각형 · 타원.
 * @param {Ctx2D} ctx
 * @param {ShapeItem} item
 * @param {DrawOptions} o
 */
export function drawShape(ctx, item, o) {
  const color = resolveInkColor(item.color, o.background);
  applyAlpha(ctx, item, o);
  ctx.strokeStyle = color;
  ctx.fillStyle = color;
  ctx.lineWidth = item.width;
  ctx.lineCap = 'round';
  ctx.lineJoin = item.shape === 'rect' ? 'miter' : 'round';
  applyDash(ctx, item.width, item.dashed);
  const { x1, y1, x2, y2 } = item;
  ctx.beginPath();
  if (item.shape === 'line') {
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
  } else if (item.shape === 'arrow') {
    const head = item.headLen ?? item.width * 3.5;
    const len = Math.hypot(x2 - x1, y2 - y1);
    const back = Math.min(len, head * Math.cos((ARROW_HEAD_DEG * Math.PI) / 180));
    const k = len > 0 ? (len - back) / len : 0;
    ctx.moveTo(x1, y1);
    ctx.lineTo(x1 + (x2 - x1) * k, y1 + (y2 - y1) * k);
    ctx.stroke();
    const [l, r] = arrowHeadPoints(x1, y1, x2, y2, head);
    ctx.setLineDash([]);
    ctx.beginPath();
    ctx.moveTo(x2, y2);
    ctx.lineTo(l.x, l.y);
    ctx.lineTo(r.x, r.y);
    ctx.closePath();
    ctx.fill();
  } else if (item.shape === 'rect') {
    ctx.strokeRect(Math.min(x1, x2), Math.min(y1, y2), Math.abs(x2 - x1), Math.abs(y2 - y1));
  } else {
    ctx.ellipse((x1 + x2) / 2, (y1 + y2) / 2, Math.abs(x2 - x1) / 2, Math.abs(y2 - y1) / 2, 0, 0, Math.PI * 2);
    ctx.stroke();
  }
}

/**
 * 글꼴 문자열.
 * @param {number} fontSize
 * @param {string} fontFamily
 */
export function fontOf(fontSize, fontFamily) {
  return `${fontSize}px "${fontFamily}", sans-serif`;
}

/**
 * 글자 상자. 줄 높이 1.3, 위쪽 반 행간은 DOM textarea와 맞춘다.
 * @param {Ctx2D} ctx
 * @param {TextItem} item
 * @param {DrawOptions} o
 */
export function drawText(ctx, item, o) {
  ctx.globalAlpha = 1;
  ctx.globalCompositeOperation = 'source-over';
  ctx.fillStyle = resolveInkColor(item.color, o.background);
  ctx.font = fontOf(item.fontSize, item.fontFamily);
  ctx.textBaseline = 'top';
  const lh = item.fontSize * TEXT_LINE_HEIGHT;
  const lead = (lh - item.fontSize) / 2;
  const lines = item.text.split('\n');
  for (let i = 0; i < lines.length; i++) ctx.fillText(lines[i], item.x, item.y + i * lh + lead);
}

/**
 * 모자이크(묶음 B): 배경 원본을 blockSize로 줄였다가 계단 그대로 키운다. 원본이 없으면 회색 칸.
 * @param {Ctx2D} ctx
 * @param {MosaicItem} item
 * @param {DrawOptions} o
 */
export function drawMosaic(ctx, item, o) {
  ctx.globalAlpha = 1;
  ctx.globalCompositeOperation = 'source-over';
  const block = Math.max(1, item.blockSize);
  if (o.image && o.createCanvas) {
    const k = o.imageScale ?? 1;
    const cw = Math.max(1, Math.ceil(item.w / block));
    const ch = Math.max(1, Math.ceil(item.h / block));
    const small = o.createCanvas(cw, ch);
    const sctx = small.getContext('2d');
    sctx.imageSmoothingEnabled = true;
    sctx.drawImage(o.image, item.x * k, item.y * k, item.w * k, item.h * k, 0, 0, cw, ch);
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(small, 0, 0, cw, ch, item.x, item.y, item.w, item.h);
    ctx.imageSmoothingEnabled = true;
    return;
  }
  ctx.fillStyle = '#8A8F98';
  ctx.fillRect(item.x, item.y, item.w, item.h);
}

/**
 * 항목 하나(상태 저장·복원으로 감쌈).
 * @param {Ctx2D} ctx
 * @param {Item} item
 * @param {DrawOptions} o
 */
export function drawItem(ctx, item, o) {
  ctx.save();
  if (item.kind === 'stroke') drawStroke(ctx, item, o);
  else if (item.kind === 'shape') drawShape(ctx, item, o);
  else if (item.kind === 'text') drawText(ctx, item, o);
  else if (item.kind === 'mosaic') drawMosaic(ctx, item, o);
  ctx.restore();
}

/**
 * 1층 전체 다시 그리기(Undo·이동·확대·바탕 바뀜). 숨긴 항목(지우는 중·옮기는 중·고치는 글자)은 뺀다.
 * @param {Ctx2D} ctx
 * @param {ReadonlyArray<Item>} items
 * @param {DrawOptions & { view: ViewTransform, dpr: number, canvasW: number, canvasH: number, hidden?: ReadonlySet<string> }} o
 */
export function renderItems(ctx, items, o) {
  clearCanvas(ctx, o.canvasW, o.canvasH);
  setViewTransform(ctx, o.view, o.dpr);
  let drawn = 0;
  for (const it of items) {
    if (o.hidden?.has(it.id)) continue;
    drawItem(ctx, it, o);
    drawn++;
  }
  return drawn;
}

/**
 * 1층 증분: 새로 굳힌 1획만 더 그린다.
 * @param {Ctx2D} ctx
 * @param {Item} item
 * @param {DrawOptions & { view: ViewTransform, dpr: number }} o
 */
export function bakeItem(ctx, item, o) {
  setViewTransform(ctx, o.view, o.dpr);
  drawItem(ctx, item, o);
}

/**
 * 화면 CSS 사각형 → 덮는 백버퍼 정수 사각형.
 * @param {Rect} r
 * @param {number} dpr
 * @param {number} canvasW
 * @param {number} canvasH
 * @returns {Rect | null}
 */
export function toCanvasRect(r, dpr, canvasW, canvasH) {
  const x = Math.max(0, Math.floor(r.x * dpr));
  const y = Math.max(0, Math.floor(r.y * dpr));
  const x2 = Math.min(canvasW, Math.ceil((r.x + r.w) * dpr));
  const y2 = Math.min(canvasH, Math.ceil((r.y + r.h) * dpr));
  if (x2 <= x || y2 <= y) return null;
  return { x, y, w: x2 - x, h: y2 - y };
}

/**
 * 2층(지금 획·고르기)이 차지하는 화면 CSS 영역.
 * @param {{ items: ReadonlyArray<Item>, marquee?: Rect | null, selectionBox?: Rect | null }} layer
 * @param {ViewTransform} view
 * @param {number} [pad]  CSS px 여유(핸들·안티에일리어싱)
 * @returns {Rect | null}
 */
export function activeViewBounds(layer, view, pad = HANDLE_SIZE_PX) {
  /** @type {Array<Rect>} */
  const rects = [];
  for (const it of layer.items) rects.push(docRectToView(it.bbox ?? itemBBox(it), view));
  if (layer.marquee) rects.push(docRectToView(layer.marquee, view));
  if (layer.selectionBox) rects.push(docRectToView(layer.selectionBox, view));
  const u = unionRect(rects);
  return u ? { x: u.x - pad, y: u.y - pad, w: u.w + pad * 2, h: u.h + pad * 2 } : null;
}

/**
 * 고르기 상자·핸들(화면 CSS 단위로 그린다).
 * @param {Ctx2D} ctx
 * @param {Rect} box  문서
 * @param {ViewTransform} view
 * @param {number} dpr
 * @param {boolean} handles
 */
export function drawSelection(ctx, box, view, dpr, handles) {
  const r = docRectToView(box, view);
  ctx.save();
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.globalAlpha = 1;
  ctx.setLineDash([]);
  ctx.strokeStyle = SELECTION_COLOR;
  ctx.lineWidth = 1.5;
  ctx.strokeRect(r.x - 3, r.y - 3, r.w + 6, r.h + 6);
  if (handles) {
    const hs = HANDLE_SIZE_PX;
    ctx.fillStyle = '#FFFFFF';
    for (const [cx, cy] of [
      [r.x, r.y],
      [r.x + r.w, r.y],
      [r.x, r.y + r.h],
      [r.x + r.w, r.y + r.h],
    ]) {
      ctx.fillRect(cx - hs / 2, cy - hs / 2, hs, hs);
      ctx.strokeRect(cx - hs / 2, cy - hs / 2, hs, hs);
    }
  }
  ctx.restore();
}

/**
 * 끌어 고르기 사각형.
 * @param {Ctx2D} ctx
 * @param {Rect} rect  문서
 * @param {ViewTransform} view
 * @param {number} dpr
 */
export function drawMarquee(ctx, rect, view, dpr) {
  const r = docRectToView(rect, view);
  ctx.save();
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.globalAlpha = 1;
  ctx.fillStyle = 'rgba(255, 159, 10, 0.08)';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.setLineDash([4, 3]);
  ctx.strokeStyle = SELECTION_COLOR;
  ctx.lineWidth = 1;
  ctx.strokeRect(r.x, r.y, r.w, r.h);
  ctx.restore();
}

/**
 * 2층 다시 그리기: 이전 dirty rect만 지우고 지금 획·미리보기·고르기를 그린 뒤 새 dirty rect(백버퍼 px)를 돌려준다.
 * @param {Ctx2D} ctx
 * @param {{ items: ReadonlyArray<Item>, marquee?: Rect | null, selectionBox?: Rect | null, handles?: boolean }} layer
 * @param {DrawOptions & { view: ViewTransform, dpr: number, canvasW: number, canvasH: number }} o
 * @param {Rect | null} prevDirty
 * @returns {Rect | null}
 */
export function renderActive(ctx, layer, o, prevDirty) {
  if (prevDirty) {
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(prevDirty.x, prevDirty.y, prevDirty.w, prevDirty.h);
  }
  const bounds = activeViewBounds(layer, o.view);
  const dirty = bounds ? toCanvasRect(bounds, o.dpr, o.canvasW, o.canvasH) : null;
  if (!dirty) return null;
  if (layer.items.length) {
    setViewTransform(ctx, o.view, o.dpr);
    for (const it of layer.items) drawItem(ctx, it, o);
  }
  if (layer.marquee) drawMarquee(ctx, layer.marquee, o.view, o.dpr);
  if (layer.selectionBox) drawSelection(ctx, layer.selectionBox, o.view, o.dpr, layer.handles ?? false);
  return dirty;
}

/**
 * 내보내기 합성(4.7): 배경 + 한 장의 모든 항목을 물리 해상도로. ctx는 physSize 크기 캔버스.
 * 확대 중 Ctrl+C·Ctrl+S도 늘 모니터 전체(사용자 결정) — doc.view(확대·이동)는 읽지 않는다.
 * @param {Ctx2D} ctx
 * @param {InkDocument} doc
 * @param {{ image?: CanvasImageSource | null, pageIndex?: number, createCanvas?: (w: number, h: number) => any }} [o]
 */
export function renderDocument(ctx, doc, o = {}) {
  const page = doc.pages[o.pageIndex ?? doc.activePage];
  const k = doc.physSize.w / doc.docSize.w;
  const background = page.background;
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.globalAlpha = 1;
  ctx.globalCompositeOperation = 'source-over';
  if (background === 'frame') {
    ctx.fillStyle = FRAME_PLACEHOLDER;
    ctx.fillRect(0, 0, doc.physSize.w, doc.physSize.h);
    if (o.image) ctx.drawImage(o.image, 0, 0, doc.physSize.w, doc.physSize.h);
  } else {
    ctx.fillStyle = BOARD_COLORS[background];
    ctx.fillRect(0, 0, doc.physSize.w, doc.physSize.h);
  }
  ctx.setTransform(k, 0, 0, k, 0, 0);
  /** @type {DrawOptions} */
  const opts = { background, composite: true, image: o.image ?? null, imageScale: k, createCanvas: o.createCanvas };
  for (const it of page.items) drawItem(ctx, it, opts);
}

/**
 * 캔버스로 글자 상자 크기를 재는 함수(InkEngine.measureText에 넣는다).
 * @param {Ctx2D} ctx
 * @returns {(text: string, fontSize: number, fontFamily: string) => { w: number, h: number }}
 */
export function createCanvasTextMeasurer(ctx) {
  return (text, fontSize, fontFamily) => {
    ctx.font = fontOf(fontSize, fontFamily);
    const lines = String(text).split('\n');
    let w = 0;
    for (const line of lines) w = Math.max(w, ctx.measureText(line).width);
    return { w: Math.max(fontSize * 0.5, w), h: lines.length * fontSize * TEXT_LINE_HEIGHT };
  };
}
