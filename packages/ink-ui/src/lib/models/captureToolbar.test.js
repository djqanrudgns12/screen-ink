import { test } from 'node:test';
import assert from 'node:assert/strict';
import { TOOLBAR_ACTIONS, TOOLBAR_EDGE, TOOLBAR_GAP, comboMap, placeToolbar, toolbarOwner, toolbarTooltip } from './captureToolbar.js';

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

test('단추 순서: 취소 · 저장 · 복사(주황)', () => {
  assert.deepEqual(TOOLBAR_ACTIONS.map((a) => a.id), ['cancel', 'save', 'copy']);
  assert.equal(TOOLBAR_ACTIONS.filter((a) => a.primary).map((a) => a.id).join(), 'copy');
});
