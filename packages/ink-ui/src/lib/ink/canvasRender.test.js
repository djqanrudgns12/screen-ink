import test from 'node:test';
import assert from 'node:assert/strict';
import {
  BOARD_COLORS,
  MARKER_OPACITY_ON_BLACK,
  bakeItem,
  createCanvasTextMeasurer,
  drawBackground,
  drawItem,
  renderActive,
  renderDocument,
  renderItems,
  resolveInkColor,
  toCanvasRect,
} from './canvasRender.js';
import { INK_COLORS, createInkDocument } from './inkEngine.js';
import { withBBox } from './hitTest.js';

/** 호출·속성 설정을 기록하는 가짜 2D 컨텍스트. 그리기 호출에는 그 순간 상태를 붙인다. */
function recorder() {
  /** @type {any[]} */
  const log = [];
  /** @type {Record<string, any>} */
  const t = { measureText: (/** @type {string} */ s) => ({ width: s.length * 10 }) };
  const snap = () => ({
    strokeStyle: t.strokeStyle,
    fillStyle: t.fillStyle,
    lineWidth: t.lineWidth,
    globalAlpha: t.globalAlpha,
    globalCompositeOperation: t.globalCompositeOperation,
    lineCap: t.lineCap,
    font: t.font,
  });
  for (const m of ['setTransform', 'clearRect', 'strokeRect', 'beginPath', 'closePath', 'moveTo', 'lineTo', 'quadraticCurveTo', 'ellipse', 'setLineDash', 'drawImage', 'save', 'restore']) {
    t[m] = (/** @type {any[]} */ ...a) => log.push([m, ...a]);
  }
  for (const m of ['stroke', 'fill']) t[m] = () => log.push([m, snap()]);
  t.fillRect = (/** @type {any[]} */ ...a) => log.push(['fillRect', ...a, snap()]);
  t.fillText = (/** @type {any[]} */ ...a) => log.push(['fillText', ...a, snap()]);
  const ctx = new Proxy(t, {
    set(target, k, v) {
      log.push(['set', k, v]);
      target[/** @type {string} */ (k)] = v;
      return true;
    },
  });
  return {
    ctx: /** @type {any} */ (ctx),
    log,
    /** @param {string} name */
    calls: (name) => log.filter((e) => e[0] === name),
  };
}

/**
 * @param {Partial<import('./inkEngine.js').StrokeItem>} [o]
 * @returns {import('./inkEngine.js').StrokeItem}
 */
function stroke(o = {}) {
  return withBBox({
    id: 's',
    kind: 'stroke',
    tool: 'pen',
    color: 'R',
    width: 4,
    opacity: 1,
    dashed: false,
    points: [
      { x: 10, y: 10, p: 0.5, t: 0 },
      { x: 20, y: 12, p: 0.5, t: 8 },
      { x: 30, y: 10, p: 0.5, t: 16 },
    ],
    ...o,
  });
}

const VIEW = { s: 2, tx: -10, ty: -20 };
const BASE = { view: VIEW, dpr: 1.5, canvasW: 300, canvasH: 200, background: /** @type {const} */ ('frame') };

test('검은칠판에서_K는_흰색으로_치환', () => {
  assert.equal(resolveInkColor('K', 'black'), INK_COLORS.W);
  assert.equal(resolveInkColor('K', 'white'), INK_COLORS.K);
  assert.equal(resolveInkColor('K', 'frame'), INK_COLORS.K);
  assert.equal(resolveInkColor('B', 'black'), INK_COLORS.B);
  const r = recorder();
  drawItem(r.ctx, stroke({ color: 'K' }), { background: 'black' });
  assert.equal(r.calls('stroke')[0][1].strokeStyle, '#FFFFFF');
  const t = recorder();
  drawItem(t.ctx, withBBox({ id: 't', kind: 'text', color: 'K', fontFamily: 'IBM Plex Sans KR', fontSize: 20, x: 0, y: 0, w: 10, h: 26, text: '가' }), { background: 'black' });
  assert.equal(t.calls('fillText')[0][4].fillStyle, '#FFFFFF');
});

test('형광펜은_alpha_0_38_butt_검은칠판_0_45_내보내기만_multiply', () => {
  const m = stroke({ tool: 'marker', opacity: 0.38, color: 'Y' });
  const light = recorder();
  drawItem(light.ctx, m, { background: 'white' });
  const s1 = light.calls('stroke')[0][1];
  assert.equal(s1.globalAlpha, 0.38);
  assert.equal(s1.lineCap, 'butt');
  assert.equal(s1.globalCompositeOperation, 'source-over', '층이 나뉜 화면에서는 alpha만');
  const black = recorder();
  drawItem(black.ctx, m, { background: 'black' });
  assert.equal(black.calls('stroke')[0][1].globalAlpha, MARKER_OPACITY_ON_BLACK);
  const exp = recorder();
  drawItem(exp.ctx, m, { background: 'frame', composite: true });
  assert.equal(exp.calls('stroke')[0][1].globalCompositeOperation, 'multiply');
  assert.equal(exp.calls('stroke').length, 1, '한 경로 → 겹친 곳이 진해지지 않음');
});

test('굳힌층_전체그리기는_숨긴항목_빼고_보기변환', () => {
  const r = recorder();
  const items = [stroke({ id: 'a' }), stroke({ id: 'b' }), stroke({ id: 'c' })];
  const drawn = renderItems(r.ctx, items, { ...BASE, hidden: new Set(['b']) });
  assert.equal(drawn, 2);
  assert.deepEqual(r.log[0], ['setTransform', 1, 0, 0, 1, 0, 0]);
  assert.deepEqual(r.log[1], ['clearRect', 0, 0, 300, 200]);
  assert.deepEqual(r.log[2], ['setTransform', 3, 0, 0, 3, -15, -30], 'dpr × 보기');
  assert.equal(r.calls('stroke').length, 2);
  assert.equal(r.calls('quadraticCurveTo').length, 2, '3점 획 = 중점 베지어 1개씩');

  const inc = recorder();
  bakeItem(inc.ctx, stroke(), BASE);
  assert.equal(inc.calls('clearRect').length, 0, '증분은 지우지 않음');
  assert.equal(inc.calls('stroke').length, 1);
});

test('지금획_dirty_rect_이전영역만_지우고_새영역_반환', () => {
  const r = recorder();
  const first = renderActive(r.ctx, { items: [stroke()] }, BASE, null);
  assert.equal(r.calls('clearRect').length, 0);
  // bbox(문서) = x 7.2~32.8, y 7.2~14.8 → 화면(×2 −10, ×2 −20) x 4.4~55.6, y −5.6~9.6 → 여유 8 → 백버퍼 ×1.5(0 아래는 자름)
  assert.deepEqual(first, { x: 0, y: 0, w: 96, h: 27 });
  const second = renderActive(r.ctx, { items: [] }, BASE, first);
  assert.deepEqual(r.calls('clearRect'), [['clearRect', 0, 0, 96, 27]]);
  assert.equal(second, null);
  assert.deepEqual(toCanvasRect({ x: -50, y: -50, w: 10, h: 10 }, 2, 100, 100), null, '화면 밖');
  assert.deepEqual(toCanvasRect({ x: 10.2, y: 5, w: 10, h: 10 }, 2, 100, 100), { x: 20, y: 10, w: 21, h: 20 });

  const sel = recorder();
  renderActive(sel.ctx, { items: [], selectionBox: { x: 0, y: 0, w: 100, h: 50 }, handles: true }, { ...BASE, view: { s: 1, tx: 0, ty: 0 } }, null);
  assert.equal(sel.calls('strokeRect').length, 5, '테두리 1 + 핸들 4');
});

test('점선과_화살표_채운머리_사각형_타원', () => {
  const a = recorder();
  drawItem(a.ctx, withBBox({ id: 'a', kind: 'shape', shape: 'arrow', color: 'O', width: 4, opacity: 1, dashed: true, x1: 0, y1: 0, x2: 100, y2: 0, headLen: 14 }), { background: 'frame' });
  assert.deepEqual(a.calls('setLineDash')[0], ['setLineDash', [10, 8]], '점선 = 굵기 × 2.5 / × 2');
  assert.deepEqual(a.calls('setLineDash')[1], ['setLineDash', []], '머리는 실선');
  assert.equal(a.calls('fill').length, 1);
  const shaftEnd = a.calls('lineTo')[0];
  assert.ok(shaftEnd[1] < 100 && shaftEnd[1] > 85, '몸통은 머리 밑에서 멈춤');
  const r = recorder();
  drawItem(r.ctx, withBBox({ id: 'r', kind: 'shape', shape: 'rect', color: 'B', width: 2, opacity: 1, dashed: false, x1: 50, y1: 60, x2: 10, y2: 20 }), { background: 'frame' });
  assert.deepEqual(r.calls('strokeRect')[0], ['strokeRect', 10, 20, 40, 40], '거꾸로 끈 사각형도 정규화');
  const e = recorder();
  drawItem(e.ctx, withBBox({ id: 'e', kind: 'shape', shape: 'ellipse', color: 'G', width: 2, opacity: 1, dashed: false, x1: 0, y1: 0, x2: 100, y2: 40 }), { background: 'frame' });
  assert.deepEqual(e.calls('ellipse')[0], ['ellipse', 50, 20, 50, 20, 0, 0, Math.PI * 2]);
});

test('필압이_바뀌는_펜은_구간별_굵기_한점은_둥근점', () => {
  const r = recorder();
  drawItem(
    r.ctx,
    stroke({
      points: [
        { x: 0, y: 0, p: 0.2, t: 0 },
        { x: 10, y: 0, p: 0.6, t: 8 },
        { x: 20, y: 0, p: 1, t: 16 },
      ],
    }),
    { background: 'frame' },
  );
  const widths = r.calls('stroke').map((s) => Math.round(s[1].lineWidth * 1000) / 1000);
  assert.deepEqual(widths, [3.68, 4.96], '4 × (0.6 + 0.8 × 구간 평균 필압 0.4 · 0.8)');
  const dot = recorder();
  drawItem(dot.ctx, stroke({ points: [{ x: 5, y: 5, p: 0.5, t: 0 }] }), { background: 'frame' });
  assert.equal(dot.calls('stroke')[0][1].lineCap, 'round');
  assert.equal(dot.calls('lineTo').length, 1);
});

test('배경은_정지화면_이미지_흰_검은_칠판_채움', () => {
  const img = { width: 3840, height: 2160 };
  const f = recorder();
  drawBackground(f.ctx, { background: 'frame', image: /** @type {any} */ (img), view: VIEW, dpr: 1.5, docSize: { w: 1920, h: 1080 }, canvasW: 300, canvasH: 200 });
  assert.deepEqual(f.calls('drawImage')[0], ['drawImage', img, 0, 0, 1920, 1080]);
  assert.deepEqual(f.calls('setTransform').at(-1), ['setTransform', 3, 0, 0, 3, -15, -30], '배경도 같은 변환(판서와 프레임 동기)');
  const b = recorder();
  drawBackground(b.ctx, { background: 'black', view: VIEW, dpr: 1, docSize: { w: 10, h: 10 }, canvasW: 300, canvasH: 200 });
  assert.equal(b.calls('fillRect')[0][5].fillStyle, BOARD_COLORS.black);
  assert.equal(b.calls('drawImage').length, 0);
});

test('글자는_줄마다_fillText_줄높이_1_3_측정기', () => {
  const r = recorder();
  drawItem(r.ctx, withBBox({ id: 't', kind: 'text', color: 'R', fontFamily: 'IBM Plex Sans KR', fontSize: 20, x: 100, y: 50, w: 10, h: 52, text: '핵심\n공식' }), { background: 'frame' });
  const ft = r.calls('fillText');
  assert.equal(ft.length, 2);
  assert.deepEqual(ft.map((c) => [c[1], c[2], Math.round(c[3] * 100) / 100]), [
    ['핵심', 100, 53],
    ['공식', 100, 79],
  ]);
  assert.equal(ft[0][4].font, '20px "IBM Plex Sans KR", sans-serif');
  const measure = createCanvasTextMeasurer(recorder().ctx);
  assert.deepEqual(measure('가나다\n가', 20, 'x'), { w: 30, h: 52 });
});

test('내보내기는_물리해상도로_배경과_항목_합성', () => {
  const doc = createInkDocument({ docSize: { w: 1280, h: 720 }, physSize: { w: 1920, h: 1080 }, scale: 1.5 });
  doc.pages[0].items.push(stroke(), stroke({ id: 'm', tool: 'marker', opacity: 0.38 }));
  const img = { width: 1920, height: 1080 };
  const r = recorder();
  renderDocument(r.ctx, doc, { image: /** @type {any} */ (img) });
  assert.deepEqual(r.calls('drawImage')[0], ['drawImage', img, 0, 0, 1920, 1080]);
  assert.ok(r.calls('setTransform').some((c) => c[1] === 1.5 && c[4] === 1.5), '문서 → 물리 ×1.5');
  assert.equal(r.calls('stroke').length, 2);
  assert.equal(r.calls('stroke')[1][1].globalCompositeOperation, 'multiply');
  assert.deepEqual(r.calls('fillRect')[0].slice(1, 5), [0, 0, 1920, 1080], '모니터 전체');
  const board = createInkDocument({ mode: 'board', docSize: { w: 10, h: 10 } });
  board.pages[0].background = 'black';
  const b = recorder();
  renderDocument(b.ctx, board);
  assert.equal(b.calls('fillRect')[0][5].fillStyle, BOARD_COLORS.black);
});

test('확대중_CtrlC_CtrlS도_늘_모니터_전체_view는_쓰지_않는다', () => {
  const doc = createInkDocument({ docSize: { w: 1280, h: 720 }, physSize: { w: 1920, h: 1080 }, scale: 1.5 });
  doc.pages[0].items.push(stroke(), stroke({ id: 'far', points: [{ x: 1200, y: 700, p: 0.5, t: 0 }] }));
  const img = { width: 1920, height: 1080 };
  const plain = recorder();
  renderDocument(plain.ctx, doc, { image: /** @type {any} */ (img) });
  const zoomedDoc = { ...doc, view: { s: 4, tx: -2000, ty: -1500 } };
  const zoomed = recorder();
  renderDocument(zoomed.ctx, zoomedDoc, { image: /** @type {any} */ (img) });
  assert.deepEqual(zoomed.log, plain.log, '확대·이동과 상관없이 같은 합성');
  assert.equal(zoomed.calls('stroke').length, 2, '화면 밖(확대로 안 보이던) 항목도 포함');
  assert.deepEqual(zoomed.calls('drawImage')[0], ['drawImage', img, 0, 0, 1920, 1080]);
});

test('칠판_여러쪽_내보내기는_지금_쪽만_합성한다', () => {
  const board = createInkDocument({
    mode: 'board',
    docSize: { w: 1280, h: 720 },
    physSize: { w: 1280, h: 720 },
    pages: [
      { id: 'page-1', background: 'white', items: [stroke({ id: 'p1-s1' })] },
      { id: 'page-2', background: 'black', items: [stroke({ id: 'p2-s1' }), stroke({ id: 'p2-s2' })] },
    ],
    activePage: 1,
  });
  const r = recorder();
  renderDocument(r.ctx, board);
  assert.equal(r.calls('fillRect')[0][5].fillStyle, BOARD_COLORS.black);
  assert.equal(r.calls('stroke').length, 2, '지금 쪽(2쪽)의 획 2개만 그림');
});
