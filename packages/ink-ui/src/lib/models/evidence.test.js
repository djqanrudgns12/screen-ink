import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EVIDENCE_COLORS, evidenceBoxes, evidenceColor } from './evidence.js';

test('근거 색은 가장 구체적인 근거 순(A > V > M > W·H)', () => {
  assert.equal(evidenceColor('W·A·V'), EVIDENCE_COLORS.A);
  assert.equal(evidenceColor('W·V'), EVIDENCE_COLORS.V);
  assert.equal(evidenceColor('M'), EVIDENCE_COLORS.M);
  assert.equal(evidenceColor('W·H'), EVIDENCE_COLORS.W);
  assert.equal(evidenceColor(''), EVIDENCE_COLORS.W);
});

test('이 모니터에 걸친 마디만, CSS 좌표와 글자로', () => {
  const monitor = { bounds: { x: 1920, y: 0, w: 1920, h: 1080 }, scale: 1.5 };
  const boxes = evidenceBoxes(
    [
      { rect: { x: 2220, y: 150, w: 300, h: 60 }, label: '상자', score: 0.812, sources: 'W·V', current: true, hidden: false },
      { rect: { x: 100, y: 100, w: 50, h: 50 }, label: '창', score: 0.5, sources: 'W', current: false, hidden: false },
    ],
    monitor,
  );
  assert.equal(boxes.length, 1, '다른 모니터 마디는 뺌');
  assert.equal(boxes[0].x, 200);
  assert.equal(boxes[0].width, 200);
  assert.equal(boxes[0].tag, '상자 W·V 0.81');
  assert.equal(boxes[0].color, EVIDENCE_COLORS.V);
  assert.equal(boxes[0].current, true);
});
