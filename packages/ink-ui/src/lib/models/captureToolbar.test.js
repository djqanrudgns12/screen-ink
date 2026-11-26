import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  ANNOT_COLORS,
  ANNOT_WIDTHS,
  MOSAIC_BLOCKS,
  TOOLBAR_ACTIONS,
  TOOLBAR_ANNOTATE_TOOLS,
  TOOLBAR_EDGE,
  TOOLBAR_GAP,
  TOOLBAR_HISTORY,
  TOOLBAR_OPTIONS_GAP,
  TOOLBAR_STYLE_TRIGGER,
  annotOptionsKind,
  comboMap,
  estimateToolbarWidth,
  formatSizeBadge,
  placeSizeBadge,
  placeToolOptions,
  placeToolbar,
  toggleAnnotTool,
  toolbarOwner,
  toolbarTooltip,
} from './captureToolbar.js';

const VIEW = { w: 1920, h: 1080 };
const SIZE = { w: 120, h: 40 };

test('범위 오른쪽 아래 바깥에 붙고 오른쪽 끝을 맞춘다', () => {
  const p = placeToolbar({ x: 300, y: 200, width: 400, height: 300 }, VIEW, SIZE);
  assert.deepEqual(p, { x: 700 - 120, y: 500 + TOOLBAR_GAP, side: 'below' });
});

test('아래가 모자라면 위, 둘 다 없으면 안쪽', () => {
  const nearBottom = placeToolbar({ x: 300, y: 600, width: 400, height: 440 }, VIEW, SIZE);
  assert.equal(nearBottom.side, 'above');
  assert.equal(nearBottom.y, 600 - TOOLBAR_GAP - 40);
  const full = placeToolbar({ x: 0, y: 0, width: 1920, height: 1080 }, VIEW, SIZE);
  assert.equal(full.side, 'inside');
  assert.equal(full.y, 1080 - TOOLBAR_GAP - 40);
  assert.equal(full.x, 1920 - TOOLBAR_EDGE - 120, '안쪽 오른쪽 아래(가장자리 8px 안)');
});

test('좁은 범위·왼쪽 끝 범위도 화면 안으로 밀어 넣는다', () => {
  const tiny = placeToolbar({ x: 2, y: 100, width: 20, height: 20 }, VIEW, SIZE);
  assert.equal(tiny.x, TOOLBAR_EDGE);
  const rightEdge = placeToolbar({ x: 1800, y: 100, width: 200, height: 50 }, VIEW, SIZE);
  assert.equal(rightEdge.x, 1920 - TOOLBAR_EDGE - 120, '이 모니터 밖으로 나간 범위는 보이는 끝 기준');
});

test('여러 모니터·혼합 배율에 걸치면 오른쪽 아래 귀가 있는 모니터', () => {
  const mons = [
    { id: 0, bounds: { x: 0, y: 0, w: 1920, h: 1080 }, scale: 1 },
    { id: 1, bounds: { x: 1920, y: 0, w: 2880, h: 1800 }, scale: 1.5 },
  ];
  assert.equal(toolbarOwner({ x: 1500, y: 300, w: 800, h: 200 }, mons), 1);
  assert.equal(toolbarOwner({ x: 100, y: 100, w: 300, h: 300 }, mons), 0);
  const offset = [
    { id: 0, bounds: { x: 0, y: 0, w: 1920, h: 1080 } },
    { id: 1, bounds: { x: 1920, y: 300, w: 1280, h: 720 } },
  ];
  assert.equal(toolbarOwner({ x: 1000, y: 900, w: 1500, h: 500 }, offset), 0, '겹침이 큰 쪽');
  assert.equal(toolbarOwner(null, mons), null);
});

test('툴팁은 지금 설정된 키와 도구 한 글자 키를 읽기 표기로 만든다', () => {
  const copy = TOOLBAR_ACTIONS.find((a) => a.id === 'copy');
  const save = TOOLBAR_ACTIONS.find((a) => a.id === 'save');
  const rect = TOOLBAR_ANNOTATE_TOOLS.find((t) => t.id === 'rect');
  assert.equal(toolbarTooltip(copy, comboMap([{ id: 'c-copy', combo: 'Space' }])), '복사 · Space');
  assert.equal(toolbarTooltip(save, {}, 'windows'), '저장 · Ctrl S', '설정 전 = 확정본 기본값');
  assert.equal(toolbarTooltip(save, {}, 'macos'), '저장 · ⌘S');
  assert.equal(toolbarTooltip(copy, { 'c-copy': '' }), '복사', '키를 비웠으면 이름만');
  assert.equal(toolbarTooltip(rect, {}, 'macos'), '사각형 · R');
  assert.equal(toolbarTooltip(rect, {}, 'macos', 'Rectangle'), 'Rectangle · R');
});

test('크기 배지 형식과 자리 규칙(범위 왼쪽 위 바깥 → 안쪽)', () => {
  assert.equal(formatSizeBadge({ w: 1280, h: 720 }), '1280 × 720');
  assert.equal(formatSizeBadge({ w: 0, h: 100 }), '');
  const badge = { w: 96, h: 24 };
  const outside = placeSizeBadge({ x: 200, y: 120, width: 500, height: 300 }, VIEW, badge);
  assert.deepEqual(outside, { x: 200, y: 120 - TOOLBAR_GAP - 24, inside: false });
  const topEdge = placeSizeBadge({ x: 4, y: 4, width: 500, height: 300 }, VIEW, badge);
  assert.equal(topEdge.inside, true);
  assert.equal(topEdge.x, TOOLBAR_EDGE);
  assert.equal(topEdge.y, 4 + TOOLBAR_GAP);
});

test('툴바 주석도구 토글 및 색·굵기 패널(R2-FIX-11) 단추 위 배치', () => {
  assert.deepEqual(TOOLBAR_ANNOTATE_TOOLS.map((t) => t.id), ['rect', 'ellipse', 'arrow', 'pen', 'marker', 'mosaic', 'text', 'eraser']);
  assert.deepEqual(TOOLBAR_ANNOTATE_TOOLS.map((t) => t.key), ['R', 'O', 'A', 'P', 'H', 'M', 'T', 'E']);
  assert.deepEqual(TOOLBAR_HISTORY.map((t) => t.id), ['undo', 'redo']);
  assert.equal(TOOLBAR_STYLE_TRIGGER.id, 'style');

  assert.equal(toggleAnnotTool(null, 'pen'), 'pen');
  assert.equal(toggleAnnotTool('pen', 'pen'), null);
  assert.equal(toggleAnnotTool('pen', 'mosaic'), 'mosaic');
  assert.equal(toggleAnnotTool('pen', 'zoom'), 'pen');

  assert.equal(annotOptionsKind('rect'), 'stroke');
  assert.equal(annotOptionsKind('marker'), 'marker');
  assert.equal(annotOptionsKind('text'), 'text');
  assert.equal(annotOptionsKind('mosaic'), 'mosaic');
  assert.equal(annotOptionsKind('eraser'), 'eraser');
  assert.equal(annotOptionsKind(null), 'none');

  assert.deepEqual(ANNOT_COLORS, ['R', 'Y', 'G', 'O', 'B', 'L']);
  assert.deepEqual(ANNOT_WIDTHS.map((w) => w.px), [2, 4, 8]);
  assert.deepEqual(MOSAIC_BLOCKS.map((b) => b.px), [6, 10, 16]);

  const w = estimateToolbarWidth([TOOLBAR_ANNOTATE_TOOLS, [TOOLBAR_STYLE_TRIGGER], TOOLBAR_HISTORY, TOOLBAR_ACTIONS]);
  assert.equal(w, 15 * 34 + 3 * 9 + 8);
  assert.equal(estimateToolbarWidth([[], [], TOOLBAR_ACTIONS]), 4 * 34 + 8, '빈 묶음은 구분선 없음');

  // R2-FIX-11: 고른 도구 단추 위에 붙는 카드 (위 우선, 모자라면 아래)
  const bar = { w: 540, h: 40 };
  const panel = { w: 280, h: 210 };
  const above = placeToolOptions({ x: 500, y: 508, side: 'below' }, bar, panel, VIEW, 120);
  assert.equal(above.side, 'above');
  assert.equal(above.x, 500 + 120 - 140);
  assert.equal(above.y, 508 - TOOLBAR_OPTIONS_GAP - 210);
  assert.equal(above.arrowX, 140);

  // 위 공간이 모자라면 툴바 아래로 뒤집힌다
  const nearTop = placeToolOptions({ x: 500, y: 60, side: 'above' }, bar, panel, VIEW, 40);
  assert.equal(nearTop.side, 'below');
  assert.equal(nearTop.y, 60 + 40 + TOOLBAR_OPTIONS_GAP);

  // 화면 왼쪽 끝에서도 잘리지 않고 화살표만 이동
  const leftEdge = placeToolOptions({ x: 8, y: 400, side: 'below' }, bar, panel, VIEW, 20);
  assert.equal(leftEdge.x, TOOLBAR_EDGE);
  assert.ok(leftEdge.arrowX >= 18 && leftEdge.arrowX < panel.w);

  for (const p of [above, nearTop, leftEdge]) {
    assert.ok(p.x >= TOOLBAR_EDGE && p.x + panel.w <= VIEW.w - TOOLBAR_EDGE + 1e-9, '가로 넘침 0');
    assert.ok(p.y >= TOOLBAR_EDGE && p.y + panel.h <= VIEW.h - TOOLBAR_EDGE + 1e-9, '세로 넘침 0');
  }

  assert.equal(toolbarTooltip(TOOLBAR_HISTORY[0], {}), '되돌리기 · Ctrl Z');
  assert.equal(toolbarTooltip(TOOLBAR_HISTORY[1], { 'd-redo': 'Ctrl+Shift+Z' }), '다시 하기 · Ctrl Shift Z');
  assert.equal(toolbarTooltip(TOOLBAR_HISTORY[0], {}, 'macos'), '되돌리기 · ⌘Z');
});

test('단추 순서: 취소 · 고정 · 저장 · 복사(주황)', () => {
  assert.deepEqual(TOOLBAR_ACTIONS.map((a) => a.id), ['cancel', 'pin', 'save', 'copy']);
  assert.equal(toolbarTooltip(TOOLBAR_ACTIONS[1], {}), '화면에 고정', '키 없는 단추는 이름만');
  assert.equal(TOOLBAR_ACTIONS.filter((a) => a.primary).map((a) => a.id).join(), 'copy');
});
