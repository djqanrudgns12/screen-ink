// 캡처 오버레이(S07) 순수 모델 — 물리↔CSS 좌표, 확대경 샘플링·위치 뒤집기, 색 표기

export const MAGNIFIER_LENS = Object.freeze({
  width: 108,
  height: 84,
  zoom: 9,
  cols: 13,
  rows: 11,
  halfCols: 6,
  halfRows: 5,
  crossX: 50,
  crossY: 37,
  drawOffsetX: 50 - 6 * 9,
  drawOffsetY: 37 - 5 * 9,
});

/**
 * 오버레이 세션에서 이 창이 담당하는 모니터 항목을 찾는다.
 * @param {import('@screen-ink/client').OverlaySession | null | undefined} overlay
 * @param {number} monitorId
 * @returns {import('@screen-ink/client').OverlayMonitor | null}
 */
export function findMonitorForOverlay(overlay, monitorId) {
  if (!overlay || !Array.isArray(overlay.monitors)) return null;
  return overlay.monitors.find((m) => m.id === monitorId) ?? null;
}

/**
 * 논리 CSS 좌표(모니터 오버레이 창 안)를 가상 화면 물리 px 좌표로 바꾼다.
 * @param {number} clientX
 * @param {number} clientY
 * @param {{ bounds: { x: number, y: number, w: number, h: number }, scale?: number }} monitor
 * @returns {{ x: number, y: number }}
 */
export function clientToPhysical(clientX, clientY, monitor) {
  const scale = monitor && typeof monitor.scale === 'number' && monitor.scale > 0 ? monitor.scale : 1;
  const bx = monitor?.bounds?.x ?? 0;
  const by = monitor?.bounds?.y ?? 0;
  return {
    x: Math.round(bx + clientX * scale),
    y: Math.round(by + clientY * scale),
  };
}

/**
 * 가상 화면 물리 px 좌표를 모니터 오버레이 창의 논리 CSS 좌표로 바꾼다.
 * @param {number} physX
 * @param {number} physY
 * @param {{ bounds: { x: number, y: number, w: number, h: number }, scale?: number }} monitor
 * @returns {{ x: number, y: number }}
 */
export function physicalToClient(physX, physY, monitor) {
  const scale = monitor && typeof monitor.scale === 'number' && monitor.scale > 0 ? monitor.scale : 1;
  const bx = monitor?.bounds?.x ?? 0;
  const by = monitor?.bounds?.y ?? 0;
  return {
    x: (physX - bx) / scale,
    y: (physY - by) / scale,
  };
}

/**
 * 물리 px 좌표가 해당 모니터 범위 안인지 판정한다.
 * @param {number} physX
 * @param {number} physY
 * @param {{ bounds: { x: number, y: number, w: number, h: number } } | null | undefined} monitor
 * @returns {boolean}
 */
export function isPhysicalPointInMonitor(physX, physY, monitor) {
  if (!monitor?.bounds) return false;
  const { x, y, w, h } = monitor.bounds;
  return physX >= x && physX < x + w && physY >= y && physY < y + h;
}

/**
 * 물리 px 사각형이 해당 모니터 범위와 겹치는지 판정한다(면적이 있는 겹침만 true).
 * @param {{ x: number, y: number, w: number, h: number } | null | undefined} rect
 * @param {{ bounds: { x: number, y: number, w: number, h: number } } | null | undefined} monitor
 * @returns {boolean}
 */
export function isPhysicalRectOverlappingMonitor(rect, monitor) {
  if (!rect || !monitor?.bounds) return false;
  if (rect.w <= 0 || rect.h <= 0) return false;
  const { x, y, w, h } = monitor.bounds;
  if (w <= 0 || h <= 0) return false;
  return rect.x < x + w && rect.x + rect.w > x && rect.y < y + h && rect.y + rect.h > y;
}

/**
 * 가상 화면 물리 px 사각형을 모니터 오버레이 창의 논리 CSS 사각형으로 바꾼다.
 * 좌표 = (rect - monitor.bounds) / scale.
 * @param {{ x: number, y: number, w: number, h: number }} rect
 * @param {{ bounds: { x: number, y: number, w: number, h: number }, scale?: number }} monitor
 * @returns {{ x: number, y: number, width: number, height: number }}
 */
export function physicalRectToClient(rect, monitor) {
  const scale = monitor && typeof monitor.scale === 'number' && monitor.scale > 0 ? monitor.scale : 1;
  const bx = monitor?.bounds?.x ?? 0;
  const by = monitor?.bounds?.y ?? 0;
  return {
    x: (rect.x - bx) / scale,
    y: (rect.y - by) / scale,
    width: rect.w / scale,
    height: rect.h / scale,
  };
}

/**
 * 세션의 첫 강조 사각형(물리 px)이 이 모니터와 겹치면 논리 CSS 사각형을 돌려주고, 아니면 null.
 * @param {{ x: number, y: number, w: number, h: number } | null | undefined} highlight
 * @param {{ bounds: { x: number, y: number, w: number, h: number }, scale?: number } | null | undefined} monitor
 * @returns {{ x: number, y: number, width: number, height: number } | null}
 */
export function resolveMonitorHighlight(highlight, monitor) {
  if (!highlight || !monitor) return null;
  if (!isPhysicalRectOverlappingMonitor(highlight, monitor)) return null;
  return physicalRectToClient(highlight, monitor);
}

/**
 * CaptureView(우선) 또는 진입 첫 강조를 이 모니터의 SelectionFrame 속성으로 바꾼다.
 * view가 한 번 도착한 뒤에는 view.rect가 null이어도 옛 highlight로 되돌아가지 않는다.
 * @param {import('@screen-ink/client').CaptureView | null | undefined} view
 * @param {{ x: number, y: number, w: number, h: number } | null | undefined} sessionHighlight
 * @param {{ bounds: { x: number, y: number, w: number, h: number }, scale?: number } | null | undefined} monitor
 * @returns {{ x: number, y: number, width: number, height: number, state: 'picking' | 'confirmed', instant: boolean } | null}
 */
export function resolveCaptureFrame(view, sessionHighlight, monitor) {
  if (!monitor) return null;
  if (view) {
    const css = resolveMonitorHighlight(view.rect, monitor);
    if (!css) return null;
    return {
      ...css,
      state: view.phase === 'confirmed' ? 'confirmed' : 'picking',
      instant: view.phase === 'drag',
    };
  }
  const hl = resolveMonitorHighlight(sessionHighlight, monitor);
  if (!hl) return null;
  return { ...hl, state: 'picking', instant: false };
}

/**
 * 물리 px 사각형 크기를 고정폭 배지 문자열(`718×40`)로 바꾼다.
 * @param {{ w: number, h: number } | null | undefined} rect
 * @returns {string}
 */
export function formatRectSize(rect) {
  if (!rect || !Number.isFinite(rect.w) || !Number.isFinite(rect.h)) return '';
  const w = Math.round(rect.w);
  const h = Math.round(rect.h);
  if (w <= 0 || h <= 0) return '';
  return `${w}×${h}`;
}

/**
 * 물리 px 안내선 선분을 이 모니터 영역([bx, bx+bw] × [by, by+bh], 경계 포함)으로 자르고 논리 CSS 좌표로 바꾼다.
 * 겹치지 않거나 길이가 0이면 null.
 * @param {{ x1: number, y1: number, x2: number, y2: number } | null | undefined} guide
 * @param {{ bounds: { x: number, y: number, w: number, h: number }, scale?: number } | null | undefined} monitor
 * @returns {{ x1: number, y1: number, x2: number, y2: number } | null}
 */
export function clipGuideToMonitor(guide, monitor) {
  if (!guide || !monitor?.bounds) return null;
  const { x: bx, y: by, w: bw, h: bh } = monitor.bounds;
  if (bw <= 0 || bh <= 0) return null;

  const { x1, y1, x2, y2 } = guide;
  if (![x1, y1, x2, y2].every(Number.isFinite)) return null;

  const minX = bx;
  const maxX = bx + bw;
  const minY = by;
  const maxY = by + bh;

  const dx = x2 - x1;
  const dy = y2 - y1;
  let t0 = 0;
  let t1 = 1;

  const clip = (p, q) => {
    if (p === 0) return q >= 0;
    const r = q / p;
    if (p < 0) {
      if (r > t1) return false;
      if (r > t0) t0 = r;
    } else {
      if (r < t0) return false;
      if (r < t1) t1 = r;
    }
    return true;
  };

  if (
    !clip(-dx, x1 - minX) ||
    !clip(dx, maxX - x1) ||
    !clip(-dy, y1 - minY) ||
    !clip(dy, maxY - y1)
  ) {
    return null;
  }

  const cx1 = x1 + t0 * dx;
  const cy1 = y1 + t0 * dy;
  const cx2 = x1 + t1 * dx;
  const cy2 = y1 + t1 * dy;

  if (Math.abs(cx2 - cx1) < 1e-6 && Math.abs(cy2 - cy1) < 1e-6) {
    return null;
  }

  const p1 = physicalToClient(cx1, cy1, monitor);
  const p2 = physicalToClient(cx2, cy2, monitor);
  return { x1: p1.x, y1: p1.y, x2: p2.x, y2: p2.y };
}

/**
 * 자석 안내선 배열에서 이 모니터와 겹치는 부분만 골라 논리 CSS 선분 배열로 바꾼다.
 * @param {{ x1: number, y1: number, x2: number, y2: number }[] | null | undefined} guides
 * @param {{ bounds: { x: number, y: number, w: number, h: number }, scale?: number } | null | undefined} monitor
 * @returns {{ x1: number, y1: number, x2: number, y2: number }[]}
 */
export function resolveMonitorGuides(guides, monitor) {
  if (!Array.isArray(guides) || !monitor) return [];
  const out = [];
  for (const g of guides) {
    const clipped = clipGuideToMonitor(g, monitor);
    if (clipped) out.push(clipped);
  }
  return out;
}

/**
 * 포인터·마우스 이벤트를 CaptureInput('move'|'down'|'up'|'dblclick')으로 바꾼다.
 * 좌클릭(0)·우클릭(2) 외 버튼은 무시한다.
 * @param {'move' | 'down' | 'up' | 'dblclick'} type
 * @param {{ clientX: number, clientY: number, button?: number } | null | undefined} event
 * @param {{ bounds: { x: number, y: number, w: number, h: number }, scale?: number } | null | undefined} monitor
 * @param {{ left: number, top: number } | null | undefined} [rootRect]
 * @returns {import('@screen-ink/client').CaptureInput | null}
 */
export function toCapturePointerInput(type, event, monitor, rootRect = null) {
  if (!event || !monitor) return null;
  if (!Number.isFinite(event.clientX) || !Number.isFinite(event.clientY)) return null;
  const localX = rootRect ? event.clientX - rootRect.left : event.clientX;
  const localY = rootRect ? event.clientY - rootRect.top : event.clientY;
  const { x, y } = clientToPhysical(localX, localY, monitor);

  if (type === 'move') {
    return { type: 'move', x, y };
  }
  if (event.button !== 0 && event.button !== 2) {
    return null;
  }
  return { type, x, y, button: event.button };
}

/**
 * 휠 이벤트를 CaptureInput('wheel')으로 바꾼다. 위(deltaY < 0) = -1(넓히기), 아래 = 1(좁히기).
 * @param {{ deltaY?: number } | null | undefined} event
 * @returns {{ type: 'wheel', dy: -1 | 1 } | null}
 */
export function toCaptureWheelInput(event) {
  if (!event || !Number.isFinite(event.deltaY) || event.deltaY === 0) return null;
  return { type: 'wheel', dy: event.deltaY < 0 ? -1 : 1 };
}

const CAPTURE_ALWAYS_PREVENT_CODES = new Set([
  'Escape',
  'Tab',
  'Enter',
  'NumpadEnter',
  'ArrowUp',
  'ArrowDown',
  'ArrowLeft',
  'ArrowRight',
  'AltLeft',
  'AltRight',
]);

const CAPTURE_PLAIN_PREVENT_CODES = new Set([
  'KeyW',
  'KeyA',
  'KeyS',
  'KeyD',
  'KeyR',
  'KeyC',
  'Comma',
  'Period',
  'Backquote',
  'Slash',
]);

const CAPTURE_CTRL_PREVENT_CODES = new Set(['KeyA', 'KeyC', 'KeyS']);

const CAPTURE_PLAIN_PREVENT_KEYS = new Set(['Escape', 'Tab', 'Enter', '?', ',', '.', '`']);

/**
 * 캡처 오버레이 keydown에서 브라우저 기본 동작(Tab 포커스·Ctrl+A/C/S·방향키 스크롤·Alt 메뉴 등)을 막을지 판정한다.
 * @param {{ key?: string, code?: string, ctrlKey?: boolean, shiftKey?: boolean, altKey?: boolean, metaKey?: boolean } | null | undefined} event
 * @returns {boolean}
 */
export function shouldPreventCaptureKey(event) {
  if (!event) return false;
  const code = event.code || '';
  const key = event.key || '';

  if (CAPTURE_ALWAYS_PREVENT_CODES.has(code) || key === 'Escape' || key === 'Tab' || key === 'Enter' || key === 'Alt') {
    return true;
  }
  if (event.ctrlKey && !event.altKey && !event.metaKey) {
    if (CAPTURE_CTRL_PREVENT_CODES.has(code)) return true;
    if (/^[acsACS]$/.test(key)) return true;
    return false;
  }
  if (!event.ctrlKey && !event.altKey && !event.metaKey) {
    if (CAPTURE_PLAIN_PREVENT_CODES.has(code) || CAPTURE_PLAIN_PREVENT_KEYS.has(key)) {
      return true;
    }
  }
  return false;
}

/**
 * 키보드 이벤트를 CaptureInput('key')으로 바꾼다.
 * 한글 IME 조합 중에도 물리 code를 그대로 넘겨 Rust가 판단할 수 있게 한다.
 * @param {{ key?: string, code?: string, ctrlKey?: boolean, shiftKey?: boolean, altKey?: boolean } | null | undefined} event
 * @returns {{ type: 'key', key: string, code: string, ctrl: boolean, shift: boolean, alt: boolean } | null}
 */
export function toCaptureKeyInput(event) {
  if (!event) return null;
  const key = typeof event.key === 'string' ? event.key : '';
  const code = typeof event.code === 'string' ? event.code : '';
  if (!key && !code) return null;
  return {
    type: 'key',
    key,
    code,
    ctrl: Boolean(event.ctrlKey),
    shift: Boolean(event.shiftKey),
    alt: Boolean(event.altKey),
  };
}

/**
 * 뷰포트 CSS 좌표를 오프스크린 정지 이미지의 정수 픽셀 좌표로 바꾼다.
 * 실제 오버레이 창(뷰포트 = 물리/scale, 이미지 = 물리)에서는 clientX*scale과 1:1이다.
 * @param {number} clientX
 * @param {number} clientY
 * @param {number} viewportW
 * @param {number} viewportH
 * @param {number} imageW
 * @param {number} imageH
 * @returns {{ x: number, y: number }}
 */
export function clientToImagePixel(clientX, clientY, viewportW, viewportH, imageW, imageH) {
  if (viewportW <= 0 || viewportH <= 0 || imageW <= 0 || imageH <= 0) {
    return { x: 0, y: 0 };
  }
  const rawX = Math.floor((clientX / viewportW) * imageW);
  const rawY = Math.floor((clientY / viewportH) * imageH);
  return {
    x: Math.max(0, Math.min(imageW - 1, rawX)),
    y: Math.max(0, Math.min(imageH - 1, rawY)),
  };
}

/**
 * 중심 픽셀 주변에서 getImageData로 읽을 안전한 이미지 사각형과 패치 안 붙여넣기 오프셋을 계산한다.
 * @param {number} centerX
 * @param {number} centerY
 * @param {number} imageW
 * @param {number} imageH
 * @param {number} [halfCols]
 * @param {number} [halfRows]
 * @returns {{ sx: number, sy: number, sw: number, sh: number, dstCol: number, dstRow: number }}
 */
export function computeSampleRect(
  centerX,
  centerY,
  imageW,
  imageH,
  halfCols = MAGNIFIER_LENS.halfCols,
  halfRows = MAGNIFIER_LENS.halfRows,
) {
  const x0 = centerX - halfCols;
  const y0 = centerY - halfRows;
  const x1 = centerX + halfCols + 1;
  const y1 = centerY + halfRows + 1;

  const sx = Math.max(0, Math.min(imageW, x0));
  const sy = Math.max(0, Math.min(imageH, y0));
  const ex = Math.max(0, Math.min(imageW, x1));
  const ey = Math.max(0, Math.min(imageH, y1));

  return {
    sx,
    sy,
    sw: Math.max(0, ex - sx),
    sh: Math.max(0, ey - sy),
    dstCol: sx - x0,
    dstRow: sy - y0,
  };
}

/**
 * getImageData 결과를 13×11 확대경 패치 버퍼와 가운데 픽셀 RGB로 정리한다.
 * 화면 가장자리 밖 칸은 투명(0,0,0,0)으로 비워 둔다.
 * @param {Uint8ClampedArray} srcData
 * @param {{ sw: number, sh: number, dstCol: number, dstRow: number }} rect
 * @param {number} [cols]
 * @param {number} [rows]
 * @param {number} [halfCols]
 * @param {number} [halfRows]
 * @returns {{ data: Uint8ClampedArray, width: number, height: number, centerRgb: [number, number, number] }}
 */
export function buildMagnifierPatch(
  srcData,
  rect,
  cols = MAGNIFIER_LENS.cols,
  rows = MAGNIFIER_LENS.rows,
  halfCols = MAGNIFIER_LENS.halfCols,
  halfRows = MAGNIFIER_LENS.halfRows,
) {
  const data = new Uint8ClampedArray(cols * rows * 4);
  const { sw, sh, dstCol, dstRow } = rect;

  for (let row = 0; row < sh; row += 1) {
    const targetRow = dstRow + row;
    if (targetRow < 0 || targetRow >= rows) continue;
    for (let col = 0; col < sw; col += 1) {
      const targetCol = dstCol + col;
      if (targetCol < 0 || targetCol >= cols) continue;
      const sIdx = (row * sw + col) * 4;
      const dIdx = (targetRow * cols + targetCol) * 4;
      data[dIdx] = srcData[sIdx];
      data[dIdx + 1] = srcData[sIdx + 1];
      data[dIdx + 2] = srcData[sIdx + 2];
      data[dIdx + 3] = srcData[sIdx + 3];
    }
  }

  const cIdx = (halfRows * cols + halfCols) * 4;
  /** @type {[number, number, number]} */
  const centerRgb =
    data[cIdx + 3] > 0 ? [data[cIdx], data[cIdx + 1], data[cIdx + 2]] : [255, 255, 255];

  return { data, width: cols, height: rows, centerRgb };
}

/**
 * 커서 위치와 뷰포트 크기에 따라 확대경 좌표와 반대쪽 뒤집기(flipX·flipY)를 계산한다.
 * 기본은 커서 오른쪽 아래, 화면 가장자리에 닿으면 반대쪽으로 넘긴다.
 * @param {number} cursorX
 * @param {number} cursorY
 * @param {number} viewportW
 * @param {number} viewportH
 * @param {number} [boxW=196]
 * @param {number} [boxH=111]
 * @param {{ offset?: number, margin?: number }} [opts]
 * @returns {{ x: number, y: number, flipX: boolean, flipY: boolean }}
 */
export function computeMagnifierPosition(
  cursorX,
  cursorY,
  viewportW,
  viewportH,
  boxW = 196,
  boxH = 111,
  { offset = 18, margin = 8 } = {},
) {
  const flipX = cursorX + offset + boxW > viewportW - margin;
  const flipY = cursorY + offset + boxH > viewportH - margin;

  const rawX = flipX ? cursorX - offset - boxW : cursorX + offset;
  const rawY = flipY ? cursorY - offset - boxH : cursorY + offset;

  const maxX = Math.max(margin, viewportW - boxW - margin);
  const maxY = Math.max(margin, viewportH - boxH - margin);

  return {
    x: Math.round(Math.max(margin, Math.min(maxX, rawX))),
    y: Math.round(Math.max(margin, Math.min(maxY, rawY))),
    flipX,
    flipY,
  };
}

/**
 * 채널 값을 0~255 정수로 맞춘다.
 * @param {number} n
 * @returns {number}
 */
export function clampByte(n) {
  if (!Number.isFinite(n)) return 0;
  return Math.max(0, Math.min(255, Math.round(n)));
}

/**
 * RGB 배열을 `#RRGGBB` 대문자 표기로 바꾼다.
 * @param {[number, number, number] | number[]} rgb
 * @returns {string}
 */
export function formatHex(rgb) {
  const r = clampByte(rgb?.[0] ?? 0).toString(16).padStart(2, '0');
  const g = clampByte(rgb?.[1] ?? 0).toString(16).padStart(2, '0');
  const b = clampByte(rgb?.[2] ?? 0).toString(16).padStart(2, '0');
  return `#${(r + g + b).toUpperCase()}`;
}

/**
 * RGB 배열을 `R, G, B` 표기로 바꾼다.
 * @param {[number, number, number] | number[]} rgb
 * @returns {string}
 */
export function formatRgb(rgb) {
  return `${clampByte(rgb?.[0] ?? 0)}, ${clampByte(rgb?.[1] ?? 0)}, ${clampByte(rgb?.[2] ?? 0)}`;
}

/**
 * 선택된 표기 방식('hex' | 'rgb')으로 색 문자열을 돌려준다.
 * @param {[number, number, number] | number[]} rgb
 * @param {'hex' | 'rgb'} [format='hex']
 * @returns {string}
 */
export function formatColor(rgb, format = 'hex') {
  return format === 'rgb' ? formatRgb(rgb) : formatHex(rgb);
}

/**
 * HEX ↔ RGB 색 표기를 전환한다(Shift).
 * @param {'hex' | 'rgb'} format
 * @returns {'hex' | 'rgb'}
 */
export function toggleColorFormat(format) {
  return format === 'rgb' ? 'hex' : 'rgb';
}
