import test from 'node:test';
import assert from 'node:assert/strict';
import {
  distPointToSegment,
  hitHandle,
  hitItemAtPoint,
  hitItemBySegment,
  itemBBox,
  itemsInRect,
  segmentToSegment,
  topItemAt,
  unionRect,
  withBBox,
} from './hitTest.js';

/** @returns {import('./inkEngine.js').StrokeItem} */
function stroke(id = 's1', width = 4) {
  return withBBox({
    id,
    kind: 'stroke',
    tool: 'pen',
    color: 'R',
    width,
    opacity: 1,
    dashed: false,
    points: [
      { x: 100, y: 100, p: 0.5, t: 0 },
      { x: 200, y: 100, p: 0.5, t: 8 },
    ],
  });
}

/**
 * @param {'line'|'rect'|'ellipse'|'arrow'} shape
 * @returns {import('./inkEngine.js').ShapeItem}
 */
function shape(shape, id = 'h1') {
  return withBBox({ id, kind: 'shape', shape, color: 'B', width: 2, opacity: 1, dashed: false, x1: 100, y1: 100, x2: 300, y2: 200, headLen: 14 });
}

test('점_선분_거리와_선분_교차', () => {
  assert.equal(distPointToSegment(5, 5, 0, 0, 10, 0), 5);
  assert.equal(distPointToSegment(-3, 4, 0, 0, 10, 0), 5, '끝점 밖이면 끝점 거리');
  assert.equal(distPointToSegment(3, 4, 0, 0, 0, 0), 5, '한 점 선분');
  assert.equal(segmentToSegment(0, 0, 10, 10, 0, 10, 10, 0), 0, 'X자 교차');
  assert.equal(segmentToSegment(0, 0, 10, 0, 0, 3, 10, 3), 3, '평행');
});

test('획_점판정은_굵기_반을_포함한다', () => {
  const s = stroke('s1', 4);
  assert.equal(hitItemAtPoint(s, 150, 102.5, 0), true, '반굵기 2 × 필압 최대 1.4 = 2.8 안');
  assert.equal(hitItemAtPoint(s, 150, 103, 0), false);
  assert.equal(hitItemAtPoint(s, 150, 110, 2), false);
  assert.equal(hitItemAtPoint(s, 150, 110, 8), true);
  const dot = withBBox({ ...s, points: [{ x: 50, y: 50, p: 0.5, t: 0 }] });
  assert.equal(hitItemAtPoint(dot, 52, 50, 1), true, '점 하나짜리 획');
});

test('지우개_두_표본_사이로_획을_가로질러도_닿는다', () => {
  const s = stroke();
  // 두 표본 (150,60) → (150,140)은 어느 쪽도 획 근처가 아니지만 선분은 획을 가로지른다
  assert.equal(hitItemBySegment(s, 150, 60, 150, 140, 8), true);
  assert.equal(hitItemBySegment(s, 250, 60, 250, 140, 8), false);
  const text = withBBox({ id: 't1', kind: 'text', color: 'R', fontFamily: 'x', fontSize: 24, x: 400, y: 400, w: 100, h: 30, text: '가' });
  assert.equal(hitItemBySegment(text, 380, 415, 520, 415, 4), true, '글자 상자를 가로지름');
  assert.equal(hitItemBySegment(text, 380, 380, 390, 390, 4), false);
});

test('사각형은_윤곽만_글자는_상자_안_전체', () => {
  const r = shape('rect');
  assert.equal(hitItemAtPoint(r, 200, 100, 3), true, '위 변');
  assert.equal(hitItemAtPoint(r, 200, 150, 3), false, '안쪽 빈 곳');
  const e = shape('ellipse');
  assert.equal(hitItemAtPoint(e, 300, 150, 3), true, '타원 오른쪽 끝');
  assert.equal(hitItemAtPoint(e, 200, 150, 3), false, '타원 가운데');
  const text = withBBox({ id: 't1', kind: 'text', color: 'R', fontFamily: 'x', fontSize: 24, x: 400, y: 400, w: 100, h: 30, text: '가' });
  assert.equal(hitItemAtPoint(text, 450, 415, 0), true);
  assert.equal(topItemAt([stroke('a'), stroke('b')], 150, 100, 2), 1, '맨 위 항목');
  assert.equal(topItemAt([stroke('a')], 900, 900, 2), -1);
});

test('끌어_고르기와_모서리_핸들', () => {
  const items = [stroke('a'), shape('rect', 'b'), withBBox({ ...stroke('c'), points: [{ x: 900, y: 900, p: 0.5, t: 0 }] })];
  assert.deepEqual(itemsInRect(items, { x: 90, y: 90, w: 50, h: 50 }), ['a', 'b']);
  const box = { x: 100, y: 100, w: 200, h: 100 };
  assert.equal(hitHandle(box, 103, 98, 8), 'nw');
  assert.equal(hitHandle(box, 300, 200, 8), 'se');
  assert.equal(hitHandle(box, 200, 150, 8), null);
  assert.deepEqual(unionRect([null, { x: 0, y: 0, w: 1, h: 1 }, { x: 5, y: 5, w: 5, h: 5 }]), { x: 0, y: 0, w: 10, h: 10 });
  assert.equal(unionRect([]), null);
});

test('화살표_bbox는_머리를_포함한다', () => {
  const a = withBBox({ id: 'a', kind: 'shape', shape: 'arrow', color: 'O', width: 4, opacity: 1, dashed: false, x1: 0, y1: 100, x2: 200, y2: 100, headLen: 14 });
  const bb = itemBBox(a);
  assert.ok(bb.y < 100 - 6, '머리 날개가 위로 벌어짐');
  assert.ok(bb.y + bb.h > 100 + 6);
  assert.equal(bb.x, -2);
  assert.equal(bb.x + bb.w, 202);
  assert.equal(hitItemAtPoint(a, 190, 96, 1), true, '날개 위');
});
