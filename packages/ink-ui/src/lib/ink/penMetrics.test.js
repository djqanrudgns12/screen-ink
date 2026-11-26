import test from 'node:test';
import assert from 'node:assert/strict';
import { createPenLatencyMeter, summarizeLatency } from './penMetrics.js';

test('펜지연_요약은_가까운순위_p50_p95_max', () => {
  const v = Array.from({ length: 100 }, (_, i) => i + 1); // 1~100 ms
  assert.deepEqual(summarizeLatency(v), { n: 100, p50: 50, p95: 95, max: 100 });
  assert.deepEqual(summarizeLatency([]), { n: 0, p50: 0, p95: 0, max: 0 });
  assert.deepEqual(summarizeLatency([12.34]), { n: 1, p50: 12.3, p95: 12.3, max: 12.3 });
});

test('그린_직후_대기중_표본의_지연을_한번에_기록', () => {
  const m = createPenLatencyMeter();
  m.mark(100);
  m.mark(104);
  assert.equal(m.pendingCount, 2);
  assert.equal(m.painted(120), 2);
  assert.equal(m.pendingCount, 0);
  assert.equal(m.painted(130), 0, '대기 표본이 없으면 기록 없음');
  assert.deepEqual(m.summary(), { n: 2, p50: 16, p95: 20, max: 20 });
  m.mark(Number.NaN);
  m.painted(200);
  assert.equal(m.count, 2, '잘못된 시각은 버림');
  m.reset();
  assert.equal(m.count, 0);
});

test('표본_상한을_넘으면_오래된것부터_덮어씀', () => {
  const m = createPenLatencyMeter({ maxSamples: 3 });
  for (const ms of [1, 2, 3, 40, 50]) m.record(ms);
  assert.equal(m.count, 3);
  assert.deepEqual(m.summary(), { n: 3, p50: 40, p95: 50, max: 50 });
});
