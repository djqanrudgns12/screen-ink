import test from 'node:test';
import assert from 'node:assert/strict';
import { SMOOTHING_PRESETS, clampDt, createOneEuroFilter, createPointSmoother, smoothPoints } from './oneEuro.js';

/** 결정적 의사 난수(-1 ~ 1). */
function prng(seed = 7) {
  let s = seed;
  return () => {
    s = (s * 1103515245 + 12345) % 2147483648;
    return (s / 2147483648) * 2 - 1;
  };
}

/** 느린 수평선(초당 60px) + y 지터 ±1.5px, 8ms 간격. */
function jitterLine(n = 200) {
  const rnd = prng();
  return Array.from({ length: n }, (_, i) => ({ x: i * 0.48, y: 100 + rnd() * 1.5, p: 0.5, t: i * 8 }));
}

/** @param {Array<{y:number}>} pts */
function yVariance(pts) {
  const tail = pts.slice(20);
  const mean = tail.reduce((a, p) => a + p.y, 0) / tail.length;
  return tail.reduce((a, p) => a + (p.y - mean) ** 2, 0) / tail.length;
}

test('원유로_보정_강도별_지연과_6px_미만_한글점_우회', () => {
  const raw = jitterLine();
  const vOff = yVariance(smoothPoints(raw, 'off'));
  const vLight = yVariance(smoothPoints(raw, 'light'));
  const vStrong = yVariance(smoothPoints(raw, 'strong'));
  assert.ok(vLight < vOff * 0.5, `약: ${vLight} < 끔 ${vOff}의 절반`);
  assert.ok(vStrong < vLight, `강 ${vStrong} < 약 ${vLight}`);

  // 빠른 획(초당 2000px)의 끝 지연: 약 < 강, 약은 화면 몇 px 안
  const fast = Array.from({ length: 30 }, (_, i) => ({ x: i * 16, y: 0, t: i * 8 }));
  const lagLight = fast.at(-1).x - smoothPoints(fast, 'light').at(-1).x;
  const lagStrong = fast.at(-1).x - smoothPoints(fast, 'strong').at(-1).x;
  assert.ok(lagLight < lagStrong, `약 지연 ${lagLight} < 강 지연 ${lagStrong}`);

  // 4px 한글 점(길이 < 6px)은 원본 좌표 그대로
  const dot = [
    { x: 10, y: 10, t: 0 },
    { x: 12.3, y: 11.1, t: 8 },
    { x: 13.1, y: 13.6, t: 16 },
  ];
  assert.deepEqual(smoothPoints(dot, 'strong'), dot);
});

test('끔은_원본_그대로_필압과_시각_유지', () => {
  const raw = jitterLine(20);
  assert.deepEqual(smoothPoints(raw, 'off'), raw);
  const sm = createPointSmoother('light');
  const out = sm.push({ x: 5, y: 6, p: 0.8, t: 3 });
  assert.equal(out.p, 0.8);
  assert.equal(out.t, 3);
  assert.equal(createPointSmoother(/** @type {any} */ ('weird')).level, 'off');
});

test('표본_간격_0나눔_방지와_긴_멈춤_묶기', () => {
  assert.equal(clampDt(0), 1 / 1000);
  assert.equal(clampDt(-5), 1 / 1000);
  assert.equal(clampDt(8), 0.008);
  assert.equal(clampDt(5000), 0.1);
  const f = createOneEuroFilter(SMOOTHING_PRESETS.light);
  assert.equal(f.filter(10, 0), 10);
  const same = f.filter(20, 0); // 같은 시각 두 표본도 NaN이 나지 않음
  assert.ok(Number.isFinite(same));
  f.reset();
  assert.equal(f.filter(50, 100), 50, 'reset 뒤 첫 표본은 그대로');
});
