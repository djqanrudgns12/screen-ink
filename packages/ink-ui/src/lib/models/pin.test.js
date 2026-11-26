import { test } from 'node:test';
import assert from 'node:assert/strict';
import { anchorOf, nextOpacity } from './pin.js';

test('투명도는 20~100에서 10씩', () => {
  assert.equal(nextOpacity(100, 1), 90);
  assert.equal(nextOpacity(20, 1), 20);
  assert.equal(nextOpacity(90, -1), 100);
  assert.equal(nextOpacity(5, 0), 20);
});

test('커서 자리 비율은 0~1', () => {
  assert.deepEqual(anchorOf(50, 25, 100, 50), { fx: 0.5, fy: 0.5 });
  assert.deepEqual(anchorOf(-5, 80, 100, 50), { fx: 0, fy: 1 });
  assert.deepEqual(anchorOf(0, 0, 0, 0), { fx: 0.5, fy: 0.5 });
});
