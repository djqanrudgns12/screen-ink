import test from 'node:test';
import assert from 'node:assert/strict';
import {
  IDENTITY_VIEW,
  centerOn,
  clampView,
  docRectToPhys,
  docSizeOf,
  docToCanvasPx,
  docToPhys,
  docToView,
  edgePushDelta,
  nextZoomStep,
  panBy,
  physToDoc,
  screenWidthToDoc,
  viewToDoc,
  visibleDocFraction,
  zoomAt,
} from './coords.js';

const VP = { w: 1920, h: 1080 };
const DOC = { w: 1920, h: 1080 };

/** @param {import('./coords.js').ViewTransform} v */
function assertNoMargin(v) {
  assert.ok(v.tx <= 1e-9, `tx ${v.tx} ≤ 0`);
  assert.ok(v.ty <= 1e-9, `ty ${v.ty} ≤ 0`);
  assert.ok(v.tx >= VP.w - DOC.w * v.s - 1e-9, '오른쪽 여백 없음');
  assert.ok(v.ty >= VP.h - DOC.h * v.s - 1e-9, '아래 여백 없음');
}

test('휠_확대_커서_고정점_유지_및_문서_가두기', () => {
  const c1 = { x: 960, y: 540 };
  const c2 = { x: 300, y: 200 };
  let v = IDENTITY_VIEW;
  const before1 = viewToDoc(c1.x, c1.y, v);
  v = zoomAt(v, 1.75, c1, VP, DOC);
  const after1 = viewToDoc(c1.x, c1.y, v);
  assert.ok(Math.abs(after1.dx - before1.dx) < 1e-6 && Math.abs(after1.dy - before1.dy) < 1e-6);
  assertNoMargin(v);

  const before2 = viewToDoc(c2.x, c2.y, v);
  v = zoomAt(v, 4, c2, VP, DOC);
  const after2 = viewToDoc(c2.x, c2.y, v);
  assert.ok(Math.abs(after2.dx - before2.dx) < 1e-6 && Math.abs(after2.dy - before2.dy) < 1e-6);
  assertNoMargin(v);

  // 되돌아오기: 4 → 1.75 → 1(가장자리 커서) — 가둔 뒤에도 화면 밖 여백 0
  v = zoomAt(v, 1.75, c2, VP, DOC);
  assertNoMargin(v);
  v = zoomAt(v, 1, { x: 10, y: 10 }, VP, DOC);
  assert.deepEqual(v, { s: 1, tx: 0, ty: 0 });

  // 가장자리에서 확대하면 가둠 때문에 고정점보다 여백 0이 우선
  const edge = zoomAt(IDENTITY_VIEW, 2, { x: 1919, y: 1079 }, VP, DOC);
  assertNoMargin(edge);
});

test('문서_화면_캔버스_좌표_왕복', () => {
  const view = { s: 2, tx: -400, ty: -300 };
  const { vx, vy } = docToView(250.5, 120.25, view);
  assert.deepEqual({ vx, vy }, { vx: 101, vy: -59.5 });
  const back = viewToDoc(vx, vy, view);
  assert.deepEqual(back, { dx: 250.5, dy: 120.25 });
  assert.deepEqual(docToCanvasPx(250.5, 120.25, view, 1.5), { cx: 151.5, cy: -89.25 });
});

test('물리_문서_변환_배율150_오른쪽_모니터', () => {
  const mon = { bounds: { x: 1920, y: 0, w: 2880, h: 1800 }, scale: 1.5 };
  assert.deepEqual(docSizeOf(mon), { w: 1920, h: 1200 });
  assert.deepEqual(physToDoc(1920 + 300, 150, mon), { dx: 200, dy: 100 });
  assert.deepEqual(docToPhys(200, 100, mon), { px: 2220, py: 150 });
  // 크롭은 덮는 정수 사각형
  // 10.2 → 1935.3(내림 1935), 11.2 → 1936.8(올림 1937)
  assert.deepEqual(docRectToPhys({ x: 10.2, y: 0, w: 1, h: 1 }, mon), { x: 1935, y: 0, w: 2, h: 2 });
});

test('확대_단계_첫휠은_시작배율로_점프', () => {
  assert.equal(nextZoomStep(1, 1, 1.75), 1.75);
  assert.equal(nextZoomStep(1, 1, 3), 3);
  assert.equal(nextZoomStep(1, 1, 9), 1.75, '모르는 시작 배율 → 기본 1.75');
  assert.equal(nextZoomStep(1.75, 1), 2);
  assert.equal(nextZoomStep(4, 1), 6);
  assert.equal(nextZoomStep(8, 1), 8);
  assert.equal(nextZoomStep(8, -1), 6);
  assert.equal(nextZoomStep(1.25, -1), 1);
  assert.equal(nextZoomStep(1, -1), 1);
});

test('가장자리_밀기_30px_안에서_초당_720px', () => {
  assert.deepEqual(edgePushDelta({ x: 10, y: 500 }, VP, 1000 / 60), { dx: 12, dy: 0 });
  assert.deepEqual(edgePushDelta({ x: 1900, y: 1070 }, VP, 1000 / 60), { dx: -12, dy: -12 });
  assert.deepEqual(edgePushDelta({ x: 500, y: 500 }, VP, 16), { dx: 0, dy: 0 });
  assert.deepEqual(edgePushDelta(null, VP, 16), { dx: 0, dy: 0 });
  // 밀어도 가둔다
  const v = panBy({ s: 2, tx: -5, ty: 0 }, 12, 0, VP, DOC);
  assert.equal(v.tx, 0);
});

test('화면기준_굵기와_최소_문서굵기', () => {
  assert.equal(screenWidthToDoc(4, 2), 2);
  assert.equal(screenWidthToDoc(4, 1), 4);
  assert.equal(screenWidthToDoc(1, 8), 0.25);
});

test('가운데로_옮기기와_미니맵_보이는_비율', () => {
  const v = { s: 2, tx: 0, ty: 0 };
  const c = centerOn(v, { x: 1200, y: 700 }, VP, DOC);
  const mid = viewToDoc(VP.w / 2, VP.h / 2, c);
  assert.deepEqual(mid, { dx: 600, dy: 350 });
  const frac = visibleDocFraction({ s: 2, tx: -960, ty: -540 }, VP, DOC);
  assert.deepEqual(frac, { x: 0.25, y: 0.25, w: 0.5, h: 0.5 });
  assert.deepEqual(clampView({ s: 12, tx: 99, ty: 99 }, VP, DOC), { s: 8, tx: 0, ty: 0 });
});
