import test from 'node:test';
import assert from 'node:assert/strict';
import { gripCursor } from './selectionGrip.js';

const F = { x: 100, y: 50, width: 200, height: 100 };

test('귀는 대각, 변은 가로·세로, 안은 옮기기, 밖은 없음', () => {
  assert.equal(gripCursor(F, 100, 50), 'nwse-resize');
  assert.equal(gripCursor(F, 305, 155), 'nwse-resize', '오른쪽 아래 귀(바깥 5 px)');
  assert.equal(gripCursor(F, 300, 50), 'nesw-resize');
  assert.equal(gripCursor(F, 100, 150), 'nesw-resize');
  assert.equal(gripCursor(F, 96, 100), 'ew-resize');
  assert.equal(gripCursor(F, 200, 152), 'ns-resize');
  assert.equal(gripCursor(F, 200, 100), 'move');
  assert.equal(gripCursor(F, 50, 100), '');
});

test('아주 좁은 범위는 더 가까운 변을 잡는다(Rust grip_at과 같은 규칙)', () => {
  const thin = { x: 100, y: 100, width: 4, height: 50 };
  assert.equal(gripCursor(thin, 104, 120), 'ew-resize');
  assert.equal(gripCursor(thin, 102, 120), 'ew-resize');
});
