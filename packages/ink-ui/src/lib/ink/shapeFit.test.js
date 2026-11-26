import test from 'node:test';
import assert from 'node:assert/strict';
import { fitEllipse, fitLine, fitRect, fitShape, shapeFromDrag, snapAngle } from './shapeFit.js';

const RAD = Math.PI / 180;

/** 각도 deg, 길이 len 선분 끝점. */
function endAt(deg, len = 200) {
  return { x: 100 + len * Math.cos(deg * RAD), y: 100 + len * Math.sin(deg * RAD) };
}

/** 원(반지름 R, 중심 300,300) — 점마다 ±noise 비율로 번갈아 흔든다. */
function circle(noise = 0, R = 100, n = 72) {
  return Array.from({ length: n + 1 }, (_, i) => {
    const a = (i / n) * Math.PI * 2;
    const r = R * (1 + (i % 2 ? noise : -noise));
    return { x: 300 + r * Math.cos(a), y: 300 + r * Math.sin(a) };
  });
}

/**
 * 꼭짓점을 따라 촘촘히(4px) 찍은 꺾은선.
 * @param {Array<[number, number]>} corners
 */
function trace(corners) {
  /** @type {Array<{x:number,y:number}>} */
  const out = [];
  for (let i = 0; i < corners.length - 1; i++) {
    const [ax, ay] = corners[i];
    const [bx, by] = corners[i + 1];
    const n = Math.max(1, Math.ceil(Math.hypot(bx - ax, by - ay) / 4));
    for (let k = 0; k < n; k++) out.push({ x: ax + ((bx - ax) * k) / n, y: ay + ((by - ay) * k) / n });
  }
  const [lx, ly] = corners[corners.length - 1];
  out.push({ x: lx, y: ly });
  return out;
}

/** 직선 위 점들(살짝 흔들림). */
function wobblyLine(deg, wobble = 1) {
  const a = { x: 100, y: 100 };
  const b = endAt(deg, 300);
  return Array.from({ length: 41 }, (_, i) => {
    const t = i / 40;
    const off = i % 2 ? wobble : -wobble;
    return { x: a.x + (b.x - a.x) * t - off * Math.sin(deg * RAD), y: a.y + (b.y - a.y) * t + off * Math.cos(deg * RAD) };
  });
}

test('수평_4도_맞춤_및_0_5초_멈춤_직선_원_사각형_판정', () => {
  // ±4° 맞춤: 3.5° → 0°, 5° → 유지, 수직·반대 방향도
  const e35 = endAt(3.5);
  assert.deepEqual(snapAngle(100, 100, e35.x, e35.y), { x2: e35.x, y2: 100, snapped: true });
  const e5 = endAt(5);
  assert.equal(snapAngle(100, 100, e5.x, e5.y).snapped, false);
  const e92 = endAt(92);
  assert.deepEqual(snapAngle(100, 100, e92.x, e92.y), { x2: 100, y2: e92.y, snapped: true });
  const e177 = endAt(177);
  assert.equal(snapAngle(100, 100, e177.x, e177.y).y2, 100);
  assert.equal(snapAngle(100, 100, 300, 100).snapped, false, '이미 수평이면 맞춤 아님');

  // 멈춤 판정 순서: 직선 → 원 → 사각형
  const line = fitShape(wobblyLine(3));
  assert.equal(line?.shape, 'line');
  assert.equal(line?.y2, line?.y1, '직선 판정에도 ±4° 맞춤');
  assert.equal(fitShape(wobblyLine(3), { straighten: false })?.snapped, false);
  assert.equal(fitShape(circle(0.02))?.shape, 'ellipse');
  assert.equal(
    fitShape(trace([[100, 100], [300, 100], [300, 220], [100, 220], [100, 102]]))?.shape,
    'rect',
  );
});

test('직선_판정_벗어남_6퍼센트와_최소길이_20', () => {
  assert.equal(fitLine(wobblyLine(30, 1))?.shape, 'line');
  assert.equal(fitLine(wobblyLine(30, 25)), null, '길이 300의 6%(18) 넘게 벗어나면 실패');
  assert.equal(fitLine([{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 19, y: 0 }]), null, '길이 20 미만');
});

test('원_오차_5퍼센트는_타원_15퍼센트는_실패', () => {
  const ok = fitEllipse(circle(0.05));
  assert.equal(ok?.shape, 'ellipse');
  assert.equal(ok?.circle, true, '두 반축 차 < 12% → 정원');
  assert.ok(ok && Math.abs(ok.x2 - ok.x1 - (ok.y2 - ok.y1)) < 1e-9);
  assert.equal(fitEllipse(circle(0.15)), null);
  assert.equal(fitShape(circle(0.15)), null, '사각형으로도 잘못 잡지 않음');

  // 납작한 타원은 정원으로 다듬지 않음
  const flat = Array.from({ length: 73 }, (_, i) => {
    const a = (i / 72) * Math.PI * 2;
    return { x: 300 + 150 * Math.cos(a), y: 300 + 60 * Math.sin(a) };
  });
  const el = fitEllipse(flat);
  assert.equal(el?.circle, false);
  assert.ok(el && Math.abs(el.x2 - el.x1 - 300) < 1e-6 && Math.abs(el.y2 - el.y1 - 120) < 1e-6);
});

test('사각형_4꼭짓점_90도_변_가운데에서_시작해도_rect', () => {
  const fromCorner = fitRect(trace([[100, 100], [300, 100], [300, 220], [100, 220], [100, 100]]));
  assert.deepEqual(fromCorner, { shape: 'rect', x1: 100, y1: 100, x2: 300, y2: 220 });
  const fromMid = fitRect(trace([[200, 100], [300, 100], [300, 220], [100, 220], [100, 100], [196, 100]]));
  assert.equal(fromMid?.shape, 'rect');
  // 꼭짓점이 85°·95°로 살짝 찌그러져도(±12° 안) 사각형
  const skew = fitRect(trace([[100, 100], [300, 100], [310, 220], [100, 220], [100, 100]]));
  assert.equal(skew?.shape, 'rect');
});

test('기울어진_사각형_삼각형_열린_곡선은_실패', () => {
  const c = Math.cos(20 * RAD);
  const s = Math.sin(20 * RAD);
  /** @type {Array<[number, number]>} */
  const tilted = [[0, 0], [200, 0], [200, 120], [0, 120], [0, 0]].map(([x, y]) => [300 + x * c - y * s, 300 + x * s + y * c]);
  assert.equal(fitRect(trace(tilted)), null, '기울기 ±8° 밖');
  assert.equal(fitShape(trace([[100, 300], [200, 100], [300, 300], [100, 300]])), null, '삼각형');
  const arc = Array.from({ length: 37 }, (_, i) => {
    const a = (i / 36) * Math.PI;
    return { x: 300 + 100 * Math.cos(a), y: 300 + 100 * Math.sin(a) };
  });
  assert.equal(fitShape(arc), null, '반원(열림)');
  assert.equal(fitShape([{ x: 0, y: 0 }, { x: 10, y: 0 }]), null, '경로 24px 미만');
});

test('수정키_드래그_화살표_맞춤과_사각형_원본', () => {
  const end = endAt(-2.5);
  const arrow = shapeFromDrag('arrow', { x: 100, y: 100 }, end);
  assert.equal(arrow.raw.y2, end.y);
  assert.equal(arrow.snapped?.y2, 100);
  assert.equal(arrow.snapped?.shape, 'arrow');
  assert.equal(shapeFromDrag('arrow', { x: 100, y: 100 }, end, { straighten: false }).snapped, null);
  const rect = shapeFromDrag('rect', { x: 10, y: 10 }, { x: 50, y: 12 });
  assert.equal(rect.snapped, null, '사각형은 맞춤 없음');
});
