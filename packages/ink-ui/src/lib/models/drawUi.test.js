import test from 'node:test';
import assert from 'node:assert/strict';
import {
  BOARD_PEN_SWATCHES,
  DRAW_IDLE_DIM_MS,
  DRAW_IDLE_OPACITY,
  DRAW_PALETTE_EXPAND_MS,
  DRAW_PEN_SWATCHES,
  ISLAND_AVOID_MARGIN_PX,
  buildBoardPreviewSamples,
  computeCursorRing,
  drawMinimapState,
  estimateTopIslandRect,
  formatBoardIslandMeta,
  formatBoardIslandText,
  getDrawPaletteSwatches,
  isDrawIslandVariant,
  normalizeMinimapViewport,
  resolveDrawIdleDim,
  sessionCursorToCss,
  shouldAvoidIsland,
  shouldExpandDrawPalette,
} from './drawUi.js';

test('판서 6색(R·Y·G·O·B·L)과 칠판 8색(W·K 포함) 팔레트 및 0.8초 펼침 조건', () => {
  assert.equal(DRAW_PALETTE_EXPAND_MS, 800);
  assert.deepEqual(
    getDrawPaletteSwatches(false).map((s) => s.code),
    ['R', 'Y', 'G', 'O', 'B', 'L'],
  );
  assert.deepEqual(
    getDrawPaletteSwatches(true).map((s) => s.code),
    ['R', 'Y', 'G', 'O', 'B', 'L', 'W', 'K'],
  );
  assert.equal(DRAW_PEN_SWATCHES.find((s) => s.code === 'B')?.lightFg, true);
  assert.equal(DRAW_PEN_SWATCHES.find((s) => s.code === 'R')?.lightFg, false);
  assert.equal(BOARD_PEN_SWATCHES.find((s) => s.code === 'K')?.needsBorder, true);

  assert.equal(isDrawIslandVariant('draw'), true);
  assert.equal(isDrawIslandVariant('board'), true);
  assert.equal(isDrawIslandVariant('result'), false);

  // 색이 실제로 바뀔 때만 펼침 — 처음 진입·같은 색·캡처 섬은 펼치지 않음
  assert.equal(shouldExpandDrawPalette('R', 'B', 'draw'), true);
  assert.equal(shouldExpandDrawPalette('W', 'K', 'board'), true);
  assert.equal(shouldExpandDrawPalette(null, 'R', 'draw'), false);
  assert.equal(shouldExpandDrawPalette('R', 'R', 'draw'), false);
  assert.equal(shouldExpandDrawPalette('R', 'B', 'result'), false);
  assert.equal(shouldExpandDrawPalette('R', 'PINK', 'draw'), false);
});

test('4초 무동작 40% 물러남은 마우스 올림·단추 활성 시 풀리고 명시값을 우선한다', () => {
  assert.equal(DRAW_IDLE_DIM_MS, 4000);
  assert.equal(DRAW_IDLE_OPACITY, 0.4);

  assert.equal(resolveDrawIdleDim({ autoIdleDim: true }), true);
  assert.equal(resolveDrawIdleDim({ autoIdleDim: false }), false);
  assert.equal(resolveDrawIdleDim({ explicitIdleDim: true, autoIdleDim: false }), true);
  assert.equal(resolveDrawIdleDim({ explicitIdleDim: false, autoIdleDim: true }), false);
  assert.equal(resolveDrawIdleDim({ explicitIdleDim: true, hovering: true }), false);
  assert.equal(resolveDrawIdleDim({ explicitIdleDim: true, hasAction: true }), false);
});

test('커서가 섬 bbox+24px 안에 오면 비켜 가고 바깥이면 그대로 둔다', () => {
  assert.equal(ISLAND_AVOID_MARGIN_PX, 24);
  const box = { left: 460, top: 14, right: 780, bottom: 54 };

  // 안쪽 및 정확히 24px 경계
  assert.equal(shouldAvoidIsland({ x: 600, y: 30 }, box), true);
  assert.equal(shouldAvoidIsland({ x: 436, y: 14 }, box), true);
  assert.equal(shouldAvoidIsland({ x: 804, y: 78 }, box), true);
  // 25px 바깥은 회피하지 않음
  assert.equal(shouldAvoidIsland({ x: 435, y: 30 }, box), false);
  assert.equal(shouldAvoidIsland({ x: 600, y: 79 }, box), false);
  // { x, y, w, h } 형식과 잘못된 입력
  assert.equal(shouldAvoidIsland({ x: 100, y: 20 }, { x: 110, y: 10, w: 200, h: 40 }), true);
  assert.equal(shouldAvoidIsland(null, box), false);
  assert.equal(shouldAvoidIsland({ x: 500, y: 20 }, { left: 0, top: 0, right: 0, bottom: 0 }), false);

  const est = estimateTopIslandRect(1280);
  assert.deepEqual(est, { left: 480, top: 14, right: 800, bottom: 54 });
  assert.equal(estimateTopIslandRect(0), null);
});

test('칠판 섬 문구는 "칠판 · 흰색 · 1 / 1" 모양을 중복 구분점 없이 만든다', () => {
  assert.deepEqual(formatBoardIslandText({ background: 'white', pageIndex: 0, pageCount: 1 }), {
    text: '칠판 · 흰색',
    value: '1 / 1',
    combined: '칠판 · 흰색 · 1 / 1',
  });
  assert.deepEqual(formatBoardIslandText({ background: 'black', pageIndex: 1, pageCount: 3 }), {
    text: '칠판 · 검은색',
    value: '2 / 3',
    combined: '칠판 · 검은색 · 2 / 3',
  });
  assert.equal(formatBoardIslandMeta({ text: '칠판 · 흰색', value: '1 / 1' }).combined, '칠판 · 흰색 · 1 / 1');
  assert.equal(formatBoardIslandMeta({ text: '칠판', detail: '흰색', value: '1 / 3' }).combined, '칠판 · 흰색 · 1 / 3');
});

test('펜 색 고리 커서는 도구·색·화면 굵기·검은 칠판 치환을 반영한다', () => {
  const pen4 = computeCursorRing({ tool: 'pen', color: 'R', widthPx: 4, background: 'frame' });
  assert.deepEqual(pen4, {
    kind: 'pen',
    diameter: 18,
    dot: 4,
    color: '#E5484D',
    dotOpacity: 1,
    dashed: false,
  });

  const pen20 = computeCursorRing({ tool: 'pen', color: 'K', widthPx: 20, background: 'black' });
  assert.equal(pen20?.color, '#FFFFFF', '검은 칠판에서 검정 펜은 흰색 고리로 치환');
  assert.equal(pen20?.dot, 20);
  assert.equal(pen20?.diameter, 32);

  const marker = computeCursorRing({ tool: 'marker', color: 'Y', widthPx: 12, background: 'white' });
  assert.equal(marker?.kind, 'marker');
  assert.equal(marker?.dotOpacity, 0.38);
  assert.equal(marker?.diameter, 24);

  const eraserWhite = computeCursorRing({ tool: 'eraser', background: 'white', eraserCss: 28 });
  assert.equal(eraserWhite?.dashed, true);
  assert.equal(eraserWhite?.dot, 0);
  assert.equal(eraserWhite?.color, '#1A1A1A');
  assert.equal(eraserWhite?.diameter, 28);

  const eraserBlack = computeCursorRing({ tool: 'eraser', background: 'black', eraserCss: 10 });
  assert.equal(eraserBlack?.color, '#FFFFFF');
  assert.equal(eraserBlack?.diameter, 16);

  assert.equal(computeCursorRing({ tool: 'text' }), null);
  assert.equal(computeCursorRing({ tool: 'select' }), null);
  assert.equal(computeCursorRing({ tool: 'pan' }), null);
});

test('진입 커서(물리 px)를 모니터 로컬 CSS 좌표로 변환한다', () => {
  const css = sessionCursorToCss({
    cursor: { x: 2220, y: 300 },
    monitor: { bounds: { x: 1920, y: 0, w: 2880, h: 1800 }, scale: 1.5 },
  });
  assert.deepEqual(css, { x: 200, y: 200 });
  assert.equal(
    sessionCursorToCss({
      cursor: { x: 100, y: 100 },
      monitor: { bounds: { x: 1920, y: 0, w: 1920, h: 1080 }, scale: 1 },
    }),
    null,
  );
});

test('판서 MiniMap은 확대(view.s > 1)일 때만 보이고 영역 비율을 0..1로 정규화한다', () => {
  const doc = { w: 1920, h: 1080 };
  const vp = { w: 1920, h: 1080 };

  const at1x = drawMinimapState({ s: 1, tx: 0, ty: 0 }, vp, doc);
  assert.equal(at1x.visible, false);

  const at2xCenter = drawMinimapState({ s: 2, tx: -960, ty: -540 }, vp, doc);
  assert.equal(at2xCenter.visible, true);
  assert.deepEqual(at2xCenter.viewport, { x: 0.25, y: 0.25, w: 0.5, h: 0.5 });

  const at4xCorner = drawMinimapState({ s: 4, tx: -5760, ty: -3240 }, vp, doc);
  assert.equal(at4xCorner.visible, true);
  assert.deepEqual(at4xCorner.viewport, { x: 0.75, y: 0.75, w: 0.25, h: 0.25 });

  // 넘치는 좌표도 1 안에 가둠
  assert.deepEqual(normalizeMinimapViewport({ x: 0.9, y: 0.8, w: 0.4, h: 0.5 }), {
    x: 0.6,
    y: 0.5,
    w: 0.4,
    h: 0.5,
  });
});

test('칠판 미리 보기는 흰/검은 칠판과 지금 펜 색·굵기·검은 칠판 검정 치환을 계산한다', () => {
  const samples = buildBoardPreviewSamples({ penColor: 'O', penWidth: 4 });
  assert.equal(samples.length, 2);
  assert.deepEqual(
    samples.map((s) => [s.id, s.key, s.label, s.bg, s.strokeColor]),
    [
      ['white', 'W', '흰 칠판', '#FFFFFF', '#F97316'],
      ['black', 'K', '검은 칠판', '#121212', '#F97316'],
    ],
  );
  assert.ok(samples[0].strokeWidth >= 1.5 && samples[0].strokeWidth <= 6.5);

  const thick = buildBoardPreviewSamples({ penColor: 'K', penWidth: 30 });
  assert.equal(thick[0].strokeColor, '#1A1A1A');
  assert.equal(thick[1].strokeColor, '#FFFFFF');
  assert.equal(thick[1].substituted, true);
  assert.ok(thick[0].strokeWidth > samples[0].strokeWidth);
});
