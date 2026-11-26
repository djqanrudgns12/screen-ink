import test from 'node:test';
import assert from 'node:assert/strict';
import { RDP_EPSILON, pathLength, rdp, rdpIndices, simplifyStroke } from './rdp.js';

test('RDP_직선_단순화_및_짧은획_보존', () => {
  const line = Array.from({ length: 100 }, (_, i) => ({ x: i * 3, y: i * 1.5 + (i % 2 ? 0.2 : -0.2), p: 0.5, t: i * 8 }));
  const out = simplifyStroke(line);
  assert.equal(RDP_EPSILON, 0.5);
  assert.equal(out.length, 2);
  assert.deepEqual(out[0], line[0]);
  assert.deepEqual(out[1], line[99]);

  // 길이 5px 획은 줄이지 않는다
  const short = Array.from({ length: 10 }, (_, i) => ({ x: i * 0.55, y: 0, p: 0.5, t: i * 8 }));
  assert.ok(pathLength(short) < 6);
  assert.deepEqual(simplifyStroke(short), short);
  // 점 3개 이하도 그대로
  const three = [
    { x: 0, y: 0 },
    { x: 10, y: 0.1 },
    { x: 20, y: 0 },
  ];
  assert.deepEqual(simplifyStroke(three), three);
});

test('꺾은선_꼭짓점은_남는다', () => {
  const pts = [];
  for (let i = 0; i <= 20; i++) pts.push({ x: i * 5, y: 0 });
  for (let i = 1; i <= 20; i++) pts.push({ x: 100, y: i * 5 });
  const idx = rdpIndices(pts);
  assert.deepEqual(idx, [0, 20, 40]);
});

test('살아남은_점은_원본_필압과_시각을_가진다', () => {
  const pts = [
    { x: 0, y: 0, p: 0.1, t: 0 },
    { x: 10, y: 5.1, p: 0.2, t: 8 }, // (0,0)–(20,10) 선 위 → 빠짐
    { x: 20, y: 10, p: 0.9, t: 16 },
    { x: 30, y: 0, p: 0.4, t: 24 },
  ];
  const out = rdp(pts);
  assert.deepEqual(out.map((p) => [p.p, p.t]), [
    [0.1, 0],
    [0.9, 16],
    [0.4, 24],
  ]);
  assert.notEqual(out[0], pts[0], '복사본');
});

test('경로_길이', () => {
  assert.equal(pathLength([]), 0);
  assert.equal(pathLength([{ x: 0, y: 0 }]), 0);
  assert.equal(pathLength([{ x: 0, y: 0 }, { x: 3, y: 4 }, { x: 3, y: 10 }]), 11);
});
