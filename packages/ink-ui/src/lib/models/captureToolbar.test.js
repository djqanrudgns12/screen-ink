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
  annotOptionsKind,
  comboMap,
  estimateToolbarWidth,
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

test('여러 모니터에 걸치면 오른쪽 아래 귀가 있는 모니터', () => {
  const mons = [
    { id: 0, bounds: { x: 0, y: 0, w: 1920, h: 1080 } },
    { id: 1, bounds: { x: 1920, y: 0, w: 1920, h: 1080 } },
  ];
  assert.equal(toolbarOwner({ x: 1500, y: 300, w: 800, h: 200 }, mons), 1);
  assert.equal(toolbarOwner({ x: 100, y: 100, w: 300, h: 300 }, mons), 0);
  // 귀가 두 모니터 사이 빈 곳(세로 어긋난 배치)이면 가운데 → 겹침 면적
  const offset = [
    { id: 0, bounds: { x: 0, y: 0, w: 1920, h: 1080 } },
    { id: 1, bounds: { x: 1920, y: 300, w: 1280, h: 720 } },
  ];
  // 귀(2499,1399)·가운데(1750,1150) 모두 빈 곳 → 겹침: 0번 920×180 > 1번 580×120
  assert.equal(toolbarOwner({ x: 1000, y: 900, w: 1500, h: 500 }, offset), 0, '겹침이 큰 쪽');
  assert.equal(toolbarOwner(null, mons), null);
});

test('툴팁은 지금 설정된 키를 읽기 표기로', () => {
  const copy = TOOLBAR_ACTIONS.find((a) => a.id === 'copy');
  const save = TOOLBAR_ACTIONS.find((a) => a.id === 'save');
  assert.equal(toolbarTooltip(copy, comboMap([{ id: 'c-copy', combo: 'Space' }])), '복사 · Space');
  assert.equal(toolbarTooltip(save, {}, 'windows'), '저장 · Ctrl S', '설정 전 = 확정본 기본값');
  assert.equal(toolbarTooltip(save, {}, 'macos'), '저장 · ⌘S');
  assert.equal(toolbarTooltip(copy, { 'c-copy': '' }), '복사', '키를 비웠으면 이름만');
});

test('툴바_주석도구_토글_및_보조팔레트_배치', () => {
  // 순서(04 S08): 사각형 · 타원 · 화살표 · 펜 · 형광펜 · 모자이크 · 글자 · 지우개 │ 실행 취소 · 다시 실행
  assert.deepEqual(TOOLBAR_ANNOTATE_TOOLS.map((t) => t.id), ['rect', 'ellipse', 'arrow', 'pen', 'marker', 'mosaic', 'text', 'eraser']);
  assert.deepEqual(TOOLBAR_HISTORY.map((t) => t.id), ['undo', 'redo']);
  // 같은 도구를 다시 누르면 해제(범위 조정으로), 다른 도구는 바꿈, 모르는 id는 그대로
  assert.equal(toggleAnnotTool(null, 'pen'), 'pen');
  assert.equal(toggleAnnotTool('pen', 'pen'), null);
  assert.equal(toggleAnnotTool('pen', 'mosaic'), 'mosaic');
  assert.equal(toggleAnnotTool('pen', 'zoom'), 'pen');
  assert.equal(annotOptionsKind('rect'), 'stroke');
  assert.equal(annotOptionsKind('mosaic'), 'mosaic');
  assert.equal(annotOptionsKind('eraser'), 'none');
  assert.equal(annotOptionsKind(null), 'none');
  assert.deepEqual(ANNOT_COLORS, ['R', 'Y', 'G', 'O', 'B', 'L']);
  assert.deepEqual(ANNOT_WIDTHS.map((w) => w.px), [2, 4, 8]);
  assert.deepEqual(MOSAIC_BLOCKS.map((b) => b.px), [6, 10, 16]);
  // 너비 추정: 단추 14개 + 구분선 2개
  const w = estimateToolbarWidth([TOOLBAR_ANNOTATE_TOOLS, TOOLBAR_HISTORY, TOOLBAR_ACTIONS]);
  assert.equal(w, 14 * 34 + 2 * 9 + 8);
  assert.equal(estimateToolbarWidth([[], [], TOOLBAR_ACTIONS]), 4 * 34 + 8, '빈 묶음은 구분선 없음');
  // 툴바가 범위 아래면 보조 알약은 그 아래, 자리가 없으면 위 — 화면 밖 넘침 0
  const bar = { w: 500, h: 40 };
  const opts = { w: 220, h: 28 };
  const below = placeToolOptions({ x: 600, y: 508, side: 'below' }, bar, opts, VIEW);
  assert.deepEqual(below, { x: 600 + 500 - 220, y: 508 + 40 + TOOLBAR_OPTIONS_GAP, side: 'below' });
  const nearBottom = placeToolOptions({ x: 600, y: 1032, side: 'below' }, bar, opts, VIEW);
  assert.equal(nearBottom.side, 'above');
  assert.equal(nearBottom.y, 1032 - TOOLBAR_OPTIONS_GAP - 28);
  const above = placeToolOptions({ x: 0, y: 300, side: 'above' }, bar, opts, VIEW);
  assert.equal(above.side, 'above');
  const leftEdge = placeToolOptions({ x: 8, y: 300, side: 'above' }, { w: 100, h: 40 }, opts, VIEW);
  assert.equal(leftEdge.x, TOOLBAR_EDGE, '왼쪽 가장자리 안으로');
  for (const p of [below, nearBottom, above, leftEdge]) {
    assert.ok(p.x >= TOOLBAR_EDGE && p.x + opts.w <= VIEW.w - TOOLBAR_EDGE + 1e-9, '가로 넘침 0');
    assert.ok(p.y >= 0 && p.y + opts.h <= VIEW.h, '세로 넘침 0');
  }
  // 실행 취소·다시 실행 툴팁 = 판서 키 칸(지금 설정)
  assert.equal(toolbarTooltip(TOOLBAR_HISTORY[0], {}), '실행 취소 · Ctrl Z');
  assert.equal(toolbarTooltip(TOOLBAR_HISTORY[1], { 'd-redo': 'Ctrl+Shift+Z' }), '다시 실행 · Ctrl Shift Z');
  assert.equal(toolbarTooltip(TOOLBAR_HISTORY[0], {}, 'macos'), '실행 취소 · ⌘Z');
});

test('단추 순서: 취소 · 고정 · 저장 · 복사(주황)', () => {
  assert.deepEqual(TOOLBAR_ACTIONS.map((a) => a.id), ['cancel', 'pin', 'save', 'copy']);
  assert.equal(toolbarTooltip(TOOLBAR_ACTIONS[1], {}), '화면에 고정', '키 없는 단추는 이름만');
  assert.equal(TOOLBAR_ACTIONS.filter((a) => a.primary).map((a) => a.id).join(), 'copy');
});
