// 판서 엔진 좌표 — 물리(Phys) ↔ 문서(Doc) ↔ 화면 CSS(View) 변환은 이 파일 한 곳에서만 한다(A §2.1, R-DPI-1).
// Phys: 가상 화면 물리 px(주 모니터 왼쪽 위 원점) · Doc: 대상 모니터 왼쪽 위 원점의 논리 px · View: 오버레이 웹뷰 CSS px.
// DOM·Canvas 의존 0.

/**
 * @typedef {{ s: number, tx: number, ty: number }} ViewTransform  s = 배율, tx·ty = 문서 원점이 놓이는 화면 CSS px
 * @typedef {{ w: number, h: number }} Size
 * @typedef {{ x: number, y: number, w: number, h: number }} Rect
 * @typedef {{ bounds: Rect, scale: number }} MonitorGeometry  bounds = 물리 px, scale = 배율(1.5 = 150%)
 */

/** 휠 확대 단계(R-DRAW-9, 02 §3.5). */
export const ZOOM_STEPS = Object.freeze([1, 1.25, 1.5, 1.75, 2, 3, 4, 6, 8]);
/** 설정 `draw.startZoom`이 고를 수 있는 값. */
export const START_ZOOM_CHOICES = Object.freeze([1.25, 1.5, 1.75, 2, 3, 4]);
export const MIN_ZOOM = 1;
export const MAX_ZOOM = 8;
export const DEFAULT_START_ZOOM = 1.75;
/** 화면 기준 굵기의 문서 굵기 하한. */
export const MIN_DOC_WIDTH = 0.25;
/** 가장자리 밀기: 가장자리 안쪽 폭(CSS px)과 속도(CSS px/초, 60fps에서 12px/프레임). */
export const EDGE_PUSH_MARGIN = 30;
export const EDGE_PUSH_SPEED = 720;

export const IDENTITY_VIEW = Object.freeze({ s: 1, tx: 0, ty: 0 });

const EPS = 1e-9;

/**
 * @param {number} v
 * @param {number} lo
 * @param {number} hi
 */
export function clamp(v, lo, hi) {
  return v < lo ? lo : v > hi ? hi : v;
}

/**
 * 문서 → 화면 CSS.
 * @param {number} dx
 * @param {number} dy
 * @param {ViewTransform} view
 */
export function docToView(dx, dy, view) {
  return { vx: dx * view.s + view.tx, vy: dy * view.s + view.ty };
}

/**
 * 화면 CSS → 문서.
 * @param {number} vx
 * @param {number} vy
 * @param {ViewTransform} view
 */
export function viewToDoc(vx, vy, view) {
  return { dx: (vx - view.tx) / view.s, dy: (vy - view.ty) / view.s };
}

/**
 * 문서 → 캔버스 백버퍼 px(CSS × dpr).
 * @param {number} dx
 * @param {number} dy
 * @param {ViewTransform} view
 * @param {number} dpr
 */
export function docToCanvasPx(dx, dy, view, dpr) {
  return { cx: (dx * view.s + view.tx) * dpr, cy: (dy * view.s + view.ty) * dpr };
}

/**
 * 문서 사각형 → 화면 CSS 사각형.
 * @param {Rect} r
 * @param {ViewTransform} view
 * @returns {Rect}
 */
export function docRectToView(r, view) {
  return { x: r.x * view.s + view.tx, y: r.y * view.s + view.ty, w: r.w * view.s, h: r.h * view.s };
}

/**
 * 모니터의 문서 크기(논리 px) = 물리 크기 / 배율.
 * @param {MonitorGeometry} monitor
 * @returns {Size}
 */
export function docSizeOf(monitor) {
  const scale = monitor.scale > 0 ? monitor.scale : 1;
  return { w: monitor.bounds.w / scale, h: monitor.bounds.h / scale };
}

/**
 * 가상 화면 물리 px → 그 모니터의 문서 좌표.
 * @param {number} px
 * @param {number} py
 * @param {MonitorGeometry} monitor
 */
export function physToDoc(px, py, monitor) {
  const scale = monitor.scale > 0 ? monitor.scale : 1;
  return { dx: (px - monitor.bounds.x) / scale, dy: (py - monitor.bounds.y) / scale };
}

/**
 * 문서 좌표 → 가상 화면 물리 px(실수 — 크롭은 docRectToPhys로 정수화).
 * @param {number} dx
 * @param {number} dy
 * @param {MonitorGeometry} monitor
 */
export function docToPhys(dx, dy, monitor) {
  const scale = monitor.scale > 0 ? monitor.scale : 1;
  return { px: monitor.bounds.x + dx * scale, py: monitor.bounds.y + dy * scale };
}

/**
 * 문서 사각형 → 그것을 덮는 정수 물리 사각형(바깥으로 내림·올림).
 * @param {Rect} r
 * @param {MonitorGeometry} monitor
 * @returns {Rect}
 */
export function docRectToPhys(r, monitor) {
  const a = docToPhys(r.x, r.y, monitor);
  const b = docToPhys(r.x + r.w, r.y + r.h, monitor);
  const x = Math.floor(a.px + EPS);
  const y = Math.floor(a.py + EPS);
  return { x, y, w: Math.ceil(b.px - EPS) - x, h: Math.ceil(b.py - EPS) - y };
}

/**
 * 한 축 가두기: 내용이 화면보다 크면 빈 곳이 안 보이게(t ∈ [view − content, 0]),
 * 작으면 내용 전체가 화면 안에 있게(t ∈ [0, view − content]).
 * @param {number} t
 * @param {number} viewLen
 * @param {number} contentLen
 */
function clampAxis(t, viewLen, contentLen) {
  const free = viewLen - contentLen;
  return free <= 0 ? clamp(t, free, 0) : clamp(t, 0, free);
}

/**
 * 배율을 1~8로, 이동을 문서 밖이 보이지 않게 가둔다.
 * @param {ViewTransform} view
 * @param {Size} viewport  화면 CSS 크기
 * @param {Size} docSize
 * @returns {ViewTransform}
 */
export function clampView(view, viewport, docSize) {
  const s = clamp(view.s, MIN_ZOOM, MAX_ZOOM);
  return {
    s,
    tx: clampAxis(view.tx, viewport.w, docSize.w * s),
    ty: clampAxis(view.ty, viewport.h, docSize.h * s),
  };
}

/**
 * 커서 고정점 확대: 커서 아래 문서 점이 화면에서 제자리에 있도록 이동량을 정하고 가둔다(02 §3.5).
 * @param {ViewTransform} view
 * @param {number} nextS
 * @param {{ x: number, y: number }} cursor  화면 CSS
 * @param {Size} viewport
 * @param {Size} docSize
 * @returns {ViewTransform}
 */
export function zoomAt(view, nextS, cursor, viewport, docSize) {
  const s2 = clamp(nextS, MIN_ZOOM, MAX_ZOOM);
  const k = s2 / view.s;
  return clampView(
    { s: s2, tx: cursor.x - k * (cursor.x - view.tx), ty: cursor.y - k * (cursor.y - view.ty) },
    viewport,
    docSize,
  );
}

/**
 * 다음 확대 단계. 1×에서 첫 확대는 시작 배율로 바로 간다.
 * @param {number} s  지금 배율
 * @param {1 | -1} dir  1 = 확대, -1 = 축소
 * @param {number} [startZoom]
 */
export function nextZoomStep(s, dir, startZoom = DEFAULT_START_ZOOM) {
  if (dir > 0) {
    if (s <= MIN_ZOOM + EPS) {
      return START_ZOOM_CHOICES.includes(startZoom) ? startZoom : DEFAULT_START_ZOOM;
    }
    for (const step of ZOOM_STEPS) if (step > s + EPS) return step;
    return MAX_ZOOM;
  }
  for (let i = ZOOM_STEPS.length - 1; i >= 0; i--) if (ZOOM_STEPS[i] < s - EPS) return ZOOM_STEPS[i];
  return MIN_ZOOM;
}

/**
 * 화면 CSS만큼 이동(손 모드·가장자리 밀기) 후 가두기.
 * @param {ViewTransform} view
 * @param {number} dvx
 * @param {number} dvy
 * @param {Size} viewport
 * @param {Size} docSize
 */
export function panBy(view, dvx, dvy, viewport, docSize) {
  return clampView({ s: view.s, tx: view.tx + dvx, ty: view.ty + dvy }, viewport, docSize);
}

/**
 * 화면 점이 가리키는 문서 점을 화면 가운데로 옮긴 보기.
 * @param {ViewTransform} view
 * @param {{ x: number, y: number }} cursor
 * @param {Size} viewport
 * @param {Size} docSize
 */
export function centerOn(view, cursor, viewport, docSize) {
  const d = viewToDoc(cursor.x, cursor.y, view);
  return clampView(
    { s: view.s, tx: viewport.w / 2 - d.dx * view.s, ty: viewport.h / 2 - d.dy * view.s },
    viewport,
    docSize,
  );
}

/**
 * 가장자리 밀기 이동량(CSS px). 커서가 가장자리 margin 안이면 그 반대쪽으로 문서를 민다.
 * @param {{ x: number, y: number } | null} cursor
 * @param {Size} viewport
 * @param {number} dtMs  지난 프레임과의 간격
 * @param {{ margin?: number, speed?: number }} [opts]
 */
export function edgePushDelta(cursor, viewport, dtMs, opts = {}) {
  const margin = opts.margin ?? EDGE_PUSH_MARGIN;
  const speed = opts.speed ?? EDGE_PUSH_SPEED;
  if (!cursor || !(dtMs > 0)) return { dx: 0, dy: 0 };
  const step = (speed * Math.min(dtMs, 100)) / 1000;
  const axis = (/** @type {number} */ p, /** @type {number} */ len) => {
    if (p < 0 || p > len) return 0;
    if (p < margin) return step;
    if (p > len - margin) return -step;
    return 0;
  };
  return { dx: axis(cursor.x, viewport.w), dy: axis(cursor.y, viewport.h) };
}

/**
 * 화면 기준 굵기 → 획을 만드는 순간의 문서 굵기(02 §3.2).
 * @param {number} screenWidthPx  1~30
 * @param {number} s
 */
export function screenWidthToDoc(screenWidthPx, s) {
  return Math.max(MIN_DOC_WIDTH, screenWidthPx / s);
}

/**
 * MiniMap(R-ZOOM-6)용: 지금 보이는 문서 영역을 0~1 비율로.
 * @param {ViewTransform} view
 * @param {Size} viewport
 * @param {Size} docSize
 * @returns {Rect}
 */
export function visibleDocFraction(view, viewport, docSize) {
  const a = viewToDoc(0, 0, view);
  const w = viewport.w / view.s;
  const h = viewport.h / view.s;
  const fx = (/** @type {number} */ v) => clamp(v, 0, 1);
  const x = fx(a.dx / docSize.w);
  const y = fx(a.dy / docSize.h);
  return { x, y, w: fx(w / docSize.w), h: fx(h / docSize.h) };
}

/**
 * 두 보기가 같은지(부동소수 허용).
 * @param {ViewTransform} a
 * @param {ViewTransform} b
 */
export function sameView(a, b) {
  return Math.abs(a.s - b.s) < EPS && Math.abs(a.tx - b.tx) < EPS && Math.abs(a.ty - b.ty) < EPS;
}
