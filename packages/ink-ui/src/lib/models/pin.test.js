import { test } from 'node:test';
import assert from 'node:assert/strict';
import { anchorOf, isDoublePress, nextOpacity } from './pin.js';

test('투명도는 20~100에서 10씩', () => {
  assert.equal(nextOpacity(100, 1), 90);
  assert.equal(nextOpacity(20, 1), 20);
  assert.equal(nextOpacity(90, -1), 100);
  assert.equal(nextOpacity(5, 0), 20);
});

test('두 번 누르기: 400ms·4px 안', () => {
  assert.equal(isDoublePress(null, { t: 0, x: 0, y: 0 }), false);
  assert.equal(isDoublePress({ t: 0, x: 10, y: 10 }, { t: 300, x: 12, y: 9 }), true);
  assert.equal(isDoublePress({ t: 0, x: 10, y: 10 }, { t: 500, x: 10, y: 10 }), false, '너무 늦음');
  assert.equal(isDoublePress({ t: 0, x: 10, y: 10 }, { t: 100, x: 30, y: 10 }), false, '끌다가 다시 누름');
});

test('커서 자리 비율은 0~1', () => {
  assert.deepEqual(anchorOf(50, 25, 100, 50), { fx: 0.5, fy: 0.5 });
  assert.deepEqual(anchorOf(-5, 80, 100, 50), { fx: 0, fy: 1 });
  assert.deepEqual(anchorOf(0, 0, 0, 0), { fx: 0.5, fy: 0.5 });
});
