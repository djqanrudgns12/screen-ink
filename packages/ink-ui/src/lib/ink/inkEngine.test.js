import test from 'node:test';
import assert from 'node:assert/strict';
import {
  INK_NOTICES,
  InkEngine,
  MAX_UNDO,
  applyCommand,
  createInkDocument,
  documentOptionsFromSession,
  invertCommand,
} from './inkEngine.js';
import { withBBox } from './hitTest.js';

const DOC = { w: 1920, h: 1080 };

/** @param {ConstructorParameters<typeof InkEngine>[0]} [opts] */
function engine(opts = {}) {
  return new InkEngine({ docOptions: { docSize: DOC, ...(opts.docOptions ?? {}) }, ...opts });
}

/**
 * 누르고 끌고 떼기. 점마다 8ms.
 * @param {InkEngine} e
 * @param {Array<[number, number]>} pts
 * @param {{ t0?: number, button?: number, mods?: Record<string, boolean>, holdMs?: number }} [o]
 */
function drag(e, pts, o = {}) {
  const mods = o.mods ?? {};
  let t = o.t0 ?? 0;
  const fx = [];
  const [x0, y0] = pts[0];
  fx.push(...e.pointerDown({ x: x0, y: y0, t, button: o.button ?? 0, ...mods }));
  for (const [x, y] of pts.slice(1)) {
    t += 8;
    fx.push(...e.pointerMove({ x, y, t, ...mods }));
  }
  if (o.holdMs) {
    t += o.holdMs;
    e.tick(t);
  }
  const [xl, yl] = pts[pts.length - 1];
  fx.push(...e.pointerUp({ x: xl, y: yl, t, button: o.button ?? 0, ...mods }));
  return { fx, t };
}

/**
 * @param {number} x1 @param {number} y1 @param {number} x2 @param {number} y2
 * @param {number} [n]
 * @returns {Array<[number, number]>}
 */
function line(x1, y1, x2, y2, n = 30) {
  return Array.from({ length: n + 1 }, (_, i) => [x1 + ((x2 - x1) * i) / n, y1 + ((y2 - y1) * i) / n]);
}

/**
 * @param {InkEngine} e
 * @param {string} code
 * @param {Record<string, any>} [extra]
 */
function key(e, code, extra = {}) {
  return e.keyDown({ code, key: '', ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, t: 10_000, ...extra });
}

/** @param {InkEngine} e @param {number} x @param {number} y @param {number} [button] */
function click(e, x, y, button = 0) {
  const fx = e.pointerDown({ x, y, t: 0, button });
  fx.push(...e.pointerUp({ x, y, t: 0, button }));
  return fx;
}

test('보정_후_CtrlZ는_원래_손획으로_복귀', () => {
  // draw.shapeApply 'auto'(바로 바꾸기) — A §2.3 그대로
  const e = engine({ settings: { 'draw.shapeApply': 'auto' } });
  const { fx } = drag(e, line(100, 300, 400, 305), { holdMs: 560 });
  assert.equal(e.undoStack.length, 2, 'add + correct');
  assert.deepEqual(
    e.undoStack.map((c) => c.type),
    ['add', 'correct'],
  );
  assert.equal(e.items.length, 1);
  const shaped = e.items[0];
  assert.equal(shaped.kind, 'shape');
  assert.ok(shaped.kind === 'shape' && shaped.shape === 'line' && shaped.y2 === shaped.y1, '직선 + ±4° 수평 맞춤');
  assert.deepEqual(fx, [{ type: 'notice', text: `${INK_NOTICES.line} · Ctrl+Z로 원래 획` }]);

  key(e, 'KeyZ', { ctrlKey: true });
  assert.equal(e.items.length, 1);
  assert.equal(e.items[0].kind, 'stroke', 'Ctrl+Z 1번 = 원래 손획');
  key(e, 'KeyZ', { ctrlKey: true });
  assert.equal(e.items.length, 0, 'Ctrl+Z 2번 = 획 삭제');
  key(e, 'KeyY', { ctrlKey: true });
  key(e, 'KeyY', { ctrlKey: true });
  assert.equal(e.items[0].kind, 'shape', '다시 실행 2번 = 보정까지');
});

test('멈춤도형_제안형_Enter_수락하면_correct_CtrlZ는_원래_손획', () => {
  const e = engine(); // 기본 draw.shapeApply = 'suggest'
  assert.equal(e.config.shapeApply, 'suggest');
  const pts = line(100, 300, 400, 305);
  // 누른 채 0.5초 멈추면 손획 위에 제안 도형 윤곽(점선·옅게)
  e.pointerDown({ x: 100, y: 300, t: 0, button: 0 });
  let t = 0;
  for (const [x, y] of pts.slice(1)) e.pointerMove({ x, y, t: (t += 8) });
  e.tick(t + 560);
  const during = e.getPreview();
  assert.equal(during.items[0].kind, 'stroke', '손획은 그대로');
  assert.equal(during.ghosts.length, 1);
  assert.ok(during.ghosts[0].kind === 'shape' && during.ghosts[0].dashed && during.ghosts[0].opacity < 1);
  const fx = e.pointerUp({ x: 400, y: 305, t: t + 560, button: 0 });
  assert.deepEqual(fx, [{ type: 'suggest', text: '직선으로 바꿀까요? · Enter', shape: 'line' }]);
  assert.equal(e.items[0].kind, 'stroke');
  assert.equal(e.undoStack.length, 1, '제안만으로는 문서 변화 없음');
  assert.equal(e.islandPatch.suggestion, '직선으로 바꿀까요? · Enter');
  assert.equal(e.getPreview().ghosts.length, 1);
  assert.equal(e.needsTick(), true, '4초 시간 재기');

  const r = key(e, 'Enter');
  assert.equal(r.handled, true);
  assert.deepEqual(r.effects, [
    { type: 'suggestEnd', accepted: true },
    { type: 'notice', text: `${INK_NOTICES.line} · Ctrl+Z로 원래 획` },
  ]);
  assert.equal(e.items[0].kind, 'shape');
  assert.deepEqual(
    e.undoStack.map((c) => c.type),
    ['add', 'correct'],
  );
  assert.equal(e.suggestion, null);
  key(e, 'KeyZ', { ctrlKey: true });
  assert.equal(e.items[0].kind, 'stroke', 'Ctrl+Z 1번 = 원래 손획');
  key(e, 'KeyZ', { ctrlKey: true });
  assert.equal(e.items.length, 0);

  // 섬 제안 클릭 = acceptSuggestion()
  const c = engine();
  drag(c, line(100, 300, 400, 305), { holdMs: 560 });
  assert.deepEqual(c.acceptSuggestion()[0], { type: 'suggestEnd', accepted: true });
  assert.equal(c.items[0].kind, 'shape');
  assert.deepEqual(c.acceptSuggestion(), [], '제안이 없으면 아무 일 없음');
});

test('멈춤도형_제안은_새획_Esc_도구바꾸기_4초면_버림', () => {
  const END = { type: 'suggestEnd', accepted: false };
  /** @param {InkEngine} e */
  const suggestOn = (e) => {
    const { t } = drag(e, line(100, 300, 400, 305), { holdMs: 560 });
    assert.ok(e.suggestion);
    return t;
  };
  /** @param {InkEngine} e */
  const unchanged = (e) => {
    assert.equal(e.suggestion, null);
    assert.equal(e.items[0].kind, 'stroke');
    assert.equal(e.undoStack[0].type, 'add');
    assert.ok(!e.undoStack.some((c) => c.type === 'correct'), 'correct 명령 없음');
  };

  const a = engine();
  suggestOn(a);
  assert.deepEqual(a.pointerDown({ x: 600, y: 600, t: 5000, button: 0 }), [END], '새 획 시작');
  a.pointerUp({ x: 600, y: 600, t: 5000, button: 0 });
  unchanged(a);

  const b = engine();
  suggestOn(b);
  assert.deepEqual(key(b, 'Escape').effects, [END], 'Esc = 제안 버림(나가지 않음)');
  unchanged(b);

  const c = engine();
  suggestOn(c);
  assert.deepEqual(key(c, 'KeyM').effects, [END], '도구 바꾸기');
  assert.equal(c.tool, 'marker');
  unchanged(c);

  const d = engine();
  const t = suggestOn(d);
  assert.deepEqual(d.tick(t + 3999), []);
  assert.ok(d.suggestion, '4초 전에는 남음');
  assert.deepEqual(d.tick(t + 4000), [END], '4초 지남');
  unchanged(d);
  assert.equal(d.needsTick(), false);

  const u = engine();
  suggestOn(u);
  key(u, 'KeyZ', { ctrlKey: true });
  assert.equal(u.suggestion, null, '되돌리기도 제안을 버림');
  assert.equal(u.items.length, 0);
  assert.deepEqual(key(u, 'Enter').effects, [], '버린 뒤 Enter는 아무 일 없음');

  // 수정 키 도형·끔이면 제안 없음
  const off = engine({ settings: { 'draw.shapeOnHold': false } });
  drag(off, line(100, 300, 400, 305), { holdMs: 560 });
  assert.equal(off.suggestion, null);
});

test('확대_2배에서_4px는_문서_2px로_저장', () => {
  const e = engine({ settings: { 'draw.width': 4 } });
  drag(e, line(100, 100, 300, 100, 10));
  assert.equal(e.items[0].kind === 'stroke' && e.items[0].width, 4);
  e.setView({ s: 2, tx: 0, ty: 0 });
  assert.equal(e.docWidth, 2);
  drag(e, line(100, 200, 300, 200, 10));
  const s = e.items[1];
  assert.equal(s.kind === 'stroke' && s.width, 2);
  assert.ok(s.kind === 'stroke' && s.points[0].x === 50 && s.points[0].y === 100, '화면 → 문서 좌표');
  // 글자 크기 = 굵기 × 6 / 배율
  key(e, 'KeyT');
  click(e, 400, 400);
  assert.equal(e.editing?.fontSize, 12);
});

test('스마트_우클릭_1배는_종료_확대중은_손모드_좌클릭_복귀', () => {
  const e = engine();
  assert.deepEqual(click(e, 500, 500, 2), [{ type: 'exit', reason: 'rightClick' }]);

  e.setView({ s: 2, tx: -500, ty: -300 });
  assert.deepEqual(click(e, 500, 500, 2), []);
  assert.equal(e.tool, 'pan');
  e.pointerMove({ x: 520, y: 510, t: 8 }); // 버튼 없이 움직여도 따라감
  assert.deepEqual(e.view, { s: 2, tx: -480, ty: -290 });
  click(e, 520, 510, 2);
  assert.equal(e.tool, 'pan', '손 모드에서 다시 우클릭 = 유지');
  assert.equal(e.view.s, 2, '1×로 풀지 않음');

  click(e, 600, 600, 0);
  assert.equal(e.tool, 'pen', '좌클릭 = 직전 도구');
  assert.equal(e.items.length, 0, '복귀 클릭에서는 획을 시작하지 않음');

  const off = engine({ settings: { 'draw.rightClickPan': false } });
  off.setView({ s: 2, tx: 0, ty: 0 });
  assert.deepEqual(click(off, 10, 10, 2), [{ type: 'exit', reason: 'rightClick' }]);

  // 손 모드 중 휠로 1×가 되면 손 모드도 풀린다
  e.setTool('marker');
  click(e, 500, 500, 2);
  e.setView({ s: 1, tx: 0, ty: 0 });
  assert.equal(e.tool, 'marker');
});

test('글자_IME_조합중_도구키_무시_및_빈글자_미생성', () => {
  const e = engine();
  key(e, 'KeyT');
  assert.equal(e.tool, 'text');
  click(e, 300, 300);
  assert.ok(e.editing);
  assert.deepEqual(key(e, 'KeyP', { isComposing: true }), { handled: false, effects: [] });
  e.compositionStart();
  key(e, 'KeyM', { t: 1000 });
  e.compositionEnd(1000);
  key(e, 'KeyE', { t: 1020 }); // 조합 끝 40ms 안
  assert.equal(e.tool, 'text');
  assert.deepEqual(key(e, 'KeyP', { t: 2000 }), { handled: false, effects: [] }, '입력 중 글자 키는 입력 상자로');
  assert.equal(e.tool, 'text');

  e.textInput('   ');
  key(e, 'Escape', { t: 2000 });
  assert.equal(e.editing, null);
  assert.equal(e.items.length, 0, '공백만이면 만들지 않음');

  // 입력 상자 밖에서도 조합 중 키는 무시
  key(e, 'KeyP', { keyCode: 229 });
  assert.equal(e.tool, 'text');

  click(e, 300, 300);
  e.textInput('핵심 공식');
  key(e, 'Escape');
  assert.equal(e.items.length, 1);
  const t = e.items[0];
  assert.ok(t.kind === 'text' && t.text === '핵심 공식' && t.fontSize === 24 && t.color === 'R');
});

test('실행취소_200단계_상한_및_지우개_한번끌기_단일명령', () => {
  const e = engine();
  for (let i = 0; i < 205; i++) click(e, 10 + (i % 100) * 10, 100 + Math.floor(i / 100) * 20);
  assert.equal(e.items.length, 205);
  assert.equal(e.undoStack.length, MAX_UNDO);
  while (e.undo());
  assert.equal(e.items.length, 5, '가장 오래된 5개는 굳어서 남는다');

  const er = engine();
  for (const x of [100, 150, 200]) drag(er, line(x, 100, x, 300, 10));
  key(er, 'KeyD');
  assert.equal(er.tool, 'eraser');
  drag(er, line(50, 200, 250, 200, 4)); // 표본 간격 50px — 표본 사이로 지나가도 닿음
  assert.equal(er.items.length, 0);
  assert.equal(er.undoStack.length, 4, '3 add + remove 1개');
  assert.equal(er.undoStack[3].type, 'remove');
  key(er, 'KeyZ', { ctrlKey: true });
  assert.equal(er.items.length, 3, 'Ctrl+Z 1번에 3획 복구');
  assert.deepEqual(
    er.items.map((i) => i.kind === 'stroke' && i.points[0].x),
    [100, 150, 200],
    '원래 순서',
  );
});

test('Esc_단계별_나가기_순서와_바로_나가기', () => {
  const e = engine();
  e.setView({ s: 2, tx: 0, ty: 0 });
  drag(e, line(100, 100, 300, 100, 10));
  key(e, 'KeyS');
  click(e, 200, 100);
  assert.equal(e.selection.length, 1);
  click(e, 200, 100, 2);
  assert.equal(e.tool, 'pan');
  assert.deepEqual(key(e, 'Slash', { shiftKey: true }).effects, [{ type: 'help', open: true }]);

  assert.deepEqual(key(e, 'Escape').effects, [{ type: 'help', open: false }], '① 도움말');
  key(e, 'Escape');
  assert.equal(e.selection.length, 0, '③ 고르기 해제');
  assert.equal(e.tool, 'pan');
  key(e, 'Escape');
  assert.equal(e.tool, 'select', '④ 손 모드 해제');
  assert.equal(e.view.s, 2);
  key(e, 'Escape');
  assert.equal(e.view.s, 1, '⑤ 1×');
  assert.deepEqual(key(e, 'Escape').effects, [{ type: 'exit', reason: 'esc' }], '⑥ 끝내기');

  key(e, 'KeyT');
  click(e, 50, 50);
  e.textInput('가');
  assert.deepEqual(key(e, 'Escape').effects, [], '② 글자 확정만');
  assert.equal(e.items.at(-1)?.kind, 'text');
  assert.deepEqual(key(e, 'Escape').effects, [{ type: 'exit', reason: 'esc' }]);

  const x = engine({ settings: { 'draw.esc': 'exit' } });
  x.setView({ s: 3, tx: 0, ty: 0 });
  key(x, 'KeyT');
  click(x, 50, 50);
  x.textInput('나');
  assert.deepEqual(key(x, 'Escape').effects, [{ type: 'exit', reason: 'esc' }], 'exit: 바로 나가기');
  assert.equal(x.items[0].kind === 'text' && x.items[0].text, '나', '입력 중 글자는 확정하고 나감');
});

test('수정키_도형_Shift직선_Ctrl사각_CtrlShift화살표_Tab타원_Alt점선', () => {
  const e = engine();
  drag(e, line(100, 100, 300, 250, 5), { mods: { ctrlKey: true } });
  drag(e, line(100, 400, 300, 600, 5), { mods: { ctrlKey: true, shiftKey: true } });
  key(e, 'Tab');
  drag(e, line(500, 100, 700, 250, 5));
  e.keyUp({ code: 'Tab' });
  drag(e, line(500, 400, 700, 600, 5), { mods: { shiftKey: true, altKey: true } });
  drag(e, line(800, 100, 900, 200, 5), { mods: { altKey: true } });
  const kinds = e.items.map((i) => (i.kind === 'shape' ? `${i.shape}${i.dashed ? '-dashed' : ''}` : `${i.kind}${i.kind === 'stroke' && i.dashed ? '-dashed' : ''}`));
  assert.deepEqual(kinds, ['rect', 'arrow', 'ellipse', 'line-dashed', 'stroke-dashed']);
  const arrow = e.items[1];
  assert.ok(arrow.kind === 'shape' && arrow.headLen === 14, '화살표 머리 = 굵기 4 × 3.5');
  const rect = e.items[0];
  assert.ok(rect.kind === 'shape' && rect.x1 === 100 && rect.y2 === 250);

  // 드래그 중 수정 키를 누르거나 떼면 미리보기가 바로 바뀐다
  e.pointerDown({ x: 100, y: 800, t: 0, button: 0 });
  e.pointerMove({ x: 200, y: 850, t: 8 });
  assert.equal(e.getPreview().items[0].kind, 'stroke');
  e.keyDown({ code: 'ShiftLeft', shiftKey: true, ctrlKey: false, altKey: false, metaKey: false });
  assert.equal(/** @type {any} */ (e.getPreview().items[0]).shape, 'line');
  e.keyDown({ code: 'ControlLeft', shiftKey: true, ctrlKey: true, altKey: false, metaKey: false });
  assert.equal(/** @type {any} */ (e.getPreview().items[0]).shape, 'arrow');
  e.keyUp({ code: 'ShiftLeft', shiftKey: false, ctrlKey: true, altKey: false, metaKey: false });
  e.pointerUp({ x: 200, y: 850, t: 16, button: 0, ctrlKey: true });
  assert.equal(/** @type {any} */ (e.items.at(-1)).shape, 'rect');

  // 맥: Cmd = 사각형, Ctrl은 아님
  const mac = engine({ platform: 'macos' });
  drag(mac, line(100, 100, 300, 250, 5), { mods: { metaKey: true } });
  drag(mac, line(100, 400, 300, 600, 5), { mods: { ctrlKey: true } });
  assert.deepEqual(
    mac.items.map((i) => i.kind),
    ['shape', 'stroke'],
  );
});

test('Shift직선_3도는_add와_correct_2명령_수평으로_맞췄어요', () => {
  const e = engine();
  const y2 = 100 + 200 * Math.tan((3 * Math.PI) / 180);
  const { fx } = drag(e, line(100, 100, 300, y2, 5), { mods: { shiftKey: true } });
  assert.deepEqual(fx, [{ type: 'notice', text: INK_NOTICES.horizontal }]);
  assert.deepEqual(
    e.undoStack.map((c) => c.type),
    ['add', 'correct'],
  );
  assert.equal(/** @type {any} */ (e.items[0]).y2, 100);
  e.undo();
  assert.ok(Math.abs(/** @type {any} */ (e.items[0]).y2 - y2) < 1e-9, '되돌리면 3° 원래 선');

  const v = engine();
  drag(v, line(100, 100, 100 + 200 * Math.tan((2 * Math.PI) / 180), 300, 5), { mods: { shiftKey: true } });
  assert.equal(/** @type {any} */ (v.items[0]).x2, 100, '수직 맞춤');

  const off = engine({ settings: { 'draw.straighten': false } });
  assert.deepEqual(drag(off, line(100, 100, 300, y2, 5), { mods: { shiftKey: true } }).fx, []);
  assert.equal(off.undoStack.length, 1);
});

test('짧은_한글점은_원본점_유지_긴획은_보정과_점줄이기', () => {
  const e = engine({ settings: { 'draw.smoothing': 'strong' } });
  const dotPts = /** @type {Array<[number, number]>} */ ([
    [500, 500],
    [502.1, 501.3],
    [503, 503.4],
  ]);
  drag(e, dotPts);
  const dot = e.items[0];
  assert.ok(dot.kind === 'stroke');
  assert.deepEqual(
    dot.points.map((p) => [p.x, p.y]),
    dotPts,
    '길이 < 6px: 떨림 보정·점 줄이기 건너뜀',
  );
  drag(e, line(100, 100, 700, 400, 60));
  const long = e.items[1];
  assert.ok(long.kind === 'stroke' && long.points.length < 10, `RDP로 줄임: ${long.kind === 'stroke' && long.points.length}`);
  assert.deepEqual(long.kind === 'stroke' && [long.points[0].x, long.points[0].y], [100, 100]);
  assert.equal(long.kind === 'stroke' && long.points[0].p, 0.5, '마우스 필압 0.5');

  // 펜 필압
  const pen = engine();
  pen.pointerDown({ x: 10, y: 10, t: 0, button: 0, pointerType: 'pen', pressure: 0.9 });
  pen.pointerUp({ x: 10, y: 10, t: 0, button: 0, pointerType: 'pen', pressure: 0 });
  assert.equal(/** @type {any} */ (pen.items[0]).points[0].p, 0.9);
});

test('칠판_W_K_전환과_판서중_토글', () => {
  const d = engine();
  assert.equal(d.background, 'frame');
  key(d, 'KeyK');
  assert.equal(d.background, 'black');
  key(d, 'KeyK');
  assert.equal(d.background, 'frame', '같은 키 = 원래 화면');
  key(d, 'KeyW');
  assert.equal(d.background, 'white');
  key(d, 'KeyK');
  assert.equal(d.background, 'black');
  d.undo();
  assert.equal(d.background, 'white', 'setBg도 되돌리기');

  const b = engine({ docOptions: { mode: 'board' } });
  assert.equal(b.background, 'white');
  key(b, 'KeyK');
  assert.equal(b.background, 'black');
  key(b, 'KeyW');
  assert.equal(b.background, 'white');
  const n = b.undoStack.length;
  key(b, 'KeyW');
  assert.equal(b.undoStack.length, n, '이미 흰 칠판이면 명령 없음');
  assert.deepEqual(b.islandPatch, { color: 'R', tool: 'pen', widthPx: 4, zoom: 1, background: 'white', board: true, pageIndex: 0, pageCount: 1, suggestion: null });
});

test('색키와_Ctrl휠은_고른항목_setAttr_되돌리기', () => {
  const e = engine();
  drag(e, line(100, 100, 300, 100, 10));
  drag(e, line(100, 200, 300, 200, 10));
  key(e, 'KeyS');
  drag(e, line(50, 50, 400, 300, 4)); // 끌어 고르기
  assert.equal(e.selection.length, 2);
  key(e, 'KeyB');
  assert.equal(e.color, 'B');
  assert.deepEqual(
    e.items.map((i) => /** @type {any} */ (i).color),
    ['B', 'B'],
  );
  assert.equal(e.undoStack.at(-1)?.type, 'setAttr');
  e.undo();
  assert.deepEqual(
    e.items.map((i) => /** @type {any} */ (i).color),
    ['R', 'R'],
  );

  e.setView({ s: 2, tx: 0, ty: 0 });
  e.wheel({ x: 10, y: 10, deltaY: -100, ctrlKey: true });
  assert.deepEqual(
    e.items.map((i) => /** @type {any} */ (i).width),
    [4.5, 4.5],
    '화면 1px = 문서 0.5px',
  );
  assert.equal(e.widthPx, 4, '고른 항목이 있으면 펜 굵기는 그대로');
  e.undo();
  assert.deepEqual(
    e.items.map((i) => /** @type {any} */ (i).width),
    [4, 4],
  );
});

test('고르기_끌어_이동은_move_모서리는_resize', () => {
  const e = engine();
  drag(e, line(100, 100, 200, 100, 10));
  key(e, 'KeyS');
  e.pointerDown({ x: 150, y: 100, t: 0, button: 0 });
  e.pointerMove({ x: 200, y: 150, t: 8 });
  assert.ok(e.hidden.has(e.items[0].id), '끄는 동안 아래층에서 숨김');
  assert.equal(/** @type {any} */ (e.getPreview().items[0]).points[0].x, 150);
  e.pointerUp({ x: 200, y: 150, t: 16, button: 0 });
  assert.equal(e.hidden.size, 0);
  assert.deepEqual(e.undoStack.at(-1), { type: 'move', page: 0, ids: [e.items[0].id], dx: 50, dy: 50 });
  assert.equal(/** @type {any} */ (e.items[0]).points[0].x, 150);
  e.undo();
  assert.equal(/** @type {any} */ (e.items[0]).points[0].x, 100);

  const r = engine();
  drag(r, line(100, 100, 300, 200, 4), { mods: { ctrlKey: true } });
  key(r, 'KeyS');
  click(r, 200, 100);
  assert.equal(r.selection.length, 1);
  const box = r.selectionBox();
  assert.ok(box);
  drag(r, line(box.x + box.w, box.y + box.h, 402, 302, 3));
  const cmd = r.undoStack.at(-1);
  assert.equal(cmd?.type, 'resize');
  assert.ok(cmd?.type === 'resize');
  assert.deepEqual(cmd.before, { x: 100, y: 100, w: 200, h: 100 });
  const it = /** @type {any} */ (r.items[0]);
  assert.ok(Math.abs(it.x2 - 402) < 1e-6 && Math.abs(it.y2 - 302) < 1e-6, '오른쪽 아래 핸들 = 맞은편 고정');
  r.undo();
  assert.deepEqual([/** @type {any} */ (r.items[0]).x2, /** @type {any} */ (r.items[0]).y2], [300, 200]);
});

test('휠_첫휠_시작배율_Ctrl휠_굵기_Space_1배_방향키', () => {
  const e = engine({ settings: { 'draw.startZoom': 1.75 } });
  e.wheel({ x: 960, y: 540, deltaY: -100 });
  assert.equal(e.view.s, 1.75);
  for (let i = 0; i < 5; i++) e.wheel({ x: 960, y: 540, deltaY: -10 }); // 작은 터치패드 휠 50 누적 = 한 단계
  assert.equal(e.view.s, 2);
  e.wheel({ x: 960, y: 540, deltaY: 100 });
  assert.equal(e.view.s, 1.75);
  e.wheel({ x: 10, y: 10, deltaY: -100, ctrlKey: true });
  assert.equal(e.widthPx, 5);
  for (let i = 0; i < 40; i++) e.wheel({ x: 10, y: 10, deltaY: 100, ctrlKey: true });
  assert.equal(e.widthPx, 1, '굵기 1~30');
  key(e, 'ArrowUp', { ctrlKey: true });
  assert.equal(e.widthPx, 2, 'Ctrl+Up');
  key(e, 'ArrowUp');
  assert.equal(e.view.s, 2, 'Up = 확대');
  key(e, 'ArrowDown');
  assert.equal(e.view.s, 1.75);
  assert.deepEqual(key(e, 'Space').effects, [{ type: 'centerCursor' }]);
  assert.deepEqual(e.view, { s: 1, tx: 0, ty: 0 }, 'Space = 1× 복귀');
});

test('모두지우기_E는_clear_명령과_되돌리기_알림', () => {
  const e = engine();
  assert.deepEqual(key(e, 'KeyE').effects, [], '빈 판서에서는 아무 일 없음');
  drag(e, line(100, 100, 300, 100, 10));
  drag(e, line(100, 200, 300, 200, 10));
  assert.deepEqual(key(e, 'KeyE').effects, [{ type: 'notice', text: INK_NOTICES.cleared, action: 'undo' }]);
  assert.equal(e.items.length, 0);
  key(e, 'KeyZ', { ctrlKey: true });
  assert.equal(e.items.length, 2);
  assert.deepEqual(key(e, 'KeyE', { repeat: true }).effects, [], '누르고 있어도 한 번');
});

test('명령_9종_적용과_되돌리기_왕복', () => {
  const s = withBBox({ id: 'it-1', kind: 'stroke', tool: 'pen', color: 'R', width: 4, opacity: 1, dashed: false, points: [{ x: 100, y: 100, p: 0.5, t: 0 }, { x: 200, y: 200, p: 0.5, t: 8 }] });
  const r = withBBox({ id: 'it-2', kind: 'shape', shape: 'rect', color: 'B', width: 2, opacity: 1, dashed: false, x1: 100, y1: 100, x2: 200, y2: 200 });
  const tx = withBBox({ id: 'it-3', kind: 'text', color: 'G', fontFamily: 'IBM Plex Sans KR', fontSize: 24, x: 10, y: 10, w: 48, h: 31.2, text: '가나' });
  const base = createInkDocument({ docSize: DOC, pages: [{ id: 'page-1', background: 'frame', items: [s, r, tx] }] });
  /** @type {import('./inkEngine.js').InkCommand[]} */
  const cmds = [
    { type: 'add', page: 0, item: withBBox({ ...s, id: 'it-9' }), index: 1 },
    { type: 'remove', page: 0, entries: [{ index: 0, item: s }, { index: 2, item: tx }] },
    { type: 'move', page: 0, ids: ['it-1', 'it-3'], dx: 0.5, dy: -2.25 },
    { type: 'resize', page: 0, id: 'it-2', before: { x: 100, y: 100, w: 100, h: 100 }, after: { x: 100, y: 100, w: 200, h: 400 } },
    { type: 'resize', page: 0, id: 'it-3', before: { x: 10, y: 10, w: 48, h: 31.2 }, after: { x: 10, y: 10, w: 96, h: 62.4 }, fontBefore: 24, fontAfter: 48 },
    { type: 'setAttr', page: 0, ids: ['it-1', 'it-2'], patch: { color: 'Y' }, prev: { 'it-1': { color: 'R' }, 'it-2': { color: 'B' } } },
    { type: 'correct', page: 0, before: s, after: { ...r, id: 'it-1' } },
    { type: 'clear', page: 0, items: [s, r, tx] },
    { type: 'setBg', page: 0, before: 'frame', after: 'black' },
    { type: 'addPage', index: 1, page: { id: 'page-2', background: 'frame', items: [] } },
  ];
  for (const cmd of cmds) {
    const applied = applyCommand(base, cmd);
    assert.notDeepEqual(applied, base, `${cmd.type} 적용하면 바뀜`);
    assert.deepEqual(invertCommand(applied, cmd), base, `${cmd.type} 되돌리면 원래대로`);
  }
  const added = applyCommand(base, cmds[0]);
  assert.deepEqual(
    added.pages[0].items.map((i) => i.id),
    ['it-1', 'it-9', 'it-2', 'it-3'],
  );
  const grown = applyCommand(base, cmds[4]);
  assert.equal(/** @type {any} */ (grown.pages[0].items[2]).fontSize, 48);
  assert.equal(applyCommand(base, cmds[9]).activePage, 1);
  assert.notEqual(base.pages[0].items, applyCommand(base, cmds[2]).pages[0].items, '원본은 바꾸지 않음(불변)');
  assert.equal(base.pages[0].items[0], s);
});

test('보관_내보내기와_같은모니터_배율변경_다른모니터_복원', () => {
  const a = engine({ docOptions: { id: 'draw-17', monitorId: 1, layoutFingerprint: 'fp', frameId: 17 } });
  drag(a, line(300, 300, 600, 300, 10));
  a.setView({ s: 2, tx: -100, ty: -100 });
  const kept = a.exportDocument();
  assert.equal(kept.schemaVersion, 1);
  assert.equal(kept.id, 'draw-17');
  assert.deepEqual(kept.view, { s: 2, tx: -100, ty: -100 });
  assert.equal(kept.pages[0].items.length, 1);
  assert.deepEqual(JSON.parse(JSON.stringify(kept)), kept, 'JSON 그대로 왕복');

  const same = engine();
  assert.deepEqual(same.restoreKept({ document: kept }, 'same'), [{ type: 'notice', text: INK_NOTICES.restored }]);
  assert.equal(same.items.length, 1);
  assert.equal(same.canUndo, false);
  drag(same, line(10, 10, 50, 10, 4));
  assert.notEqual(same.items[1].id, same.items[0].id, '복원 뒤 새 id가 겹치지 않음');

  const rescaled = engine({ docOptions: { docSize: { w: 1280, h: 720 }, scale: 1.5 } });
  assert.deepEqual(rescaled.restoreKept(kept, 'rescaled'), [{ type: 'notice', text: INK_NOTICES.rescaled }]);
  const st = /** @type {any} */ (rescaled.items[0]);
  assert.ok(Math.abs(st.points[0].x - 200) < 1e-9 && Math.abs(st.width - 4 * (1280 / 1920)) < 1e-9, '비율대로 위치·굵기');
  assert.deepEqual(rescaled.doc.docSize, { w: 1280, h: 720 }, '지금 모니터 정보 유지');

  const other = engine();
  assert.deepEqual(other.restoreKept(kept, 'differentMonitor'), [{ type: 'notice', text: INK_NOTICES.differentMonitor }]);
  assert.equal(other.items.length, 0);
  assert.deepEqual(other.restoreKept(null, 'none'), []);
});

test('가장자리_밀기와_멈춤판정은_tick으로_돈다', () => {
  const e = engine();
  e.setView({ s: 2, tx: -500, ty: -300 });
  e.pointerMove({ x: 5, y: 500, t: 0 });
  assert.equal(e.needsTick(), true);
  e.tick(1000);
  e.tick(1000 + 1000 / 60);
  assert.ok(Math.abs(e.view.tx - -488) < 1e-6, `12px/프레임: ${e.view.tx}`);
  e.pointerMove({ x: 900, y: 500, t: 20 });
  assert.equal(e.needsTick(), false);
  e.pointerLeave();
  assert.equal(e.needsTick(), false, '창 밖이면 멈춤');

  const one = engine();
  one.pointerMove({ x: 5, y: 5, t: 0 });
  assert.equal(one.needsTick(), false, '1×에서는 밀지 않음');
  one.pointerDown({ x: 100, y: 100, t: 0, button: 0 });
  assert.equal(one.needsTick(), true, '그리는 중 = 멈춤 판정 대기');
});

test('글자_바깥클릭_확정과_더블클릭_다시고치기', () => {
  const e = engine();
  key(e, 'KeyT');
  click(e, 100, 100);
  e.textInput('핵심 공식');
  click(e, 900, 900);
  assert.equal(e.editing, null, '바깥 클릭 = 입력 끝');
  assert.equal(e.items.length, 1);
  key(e, 'KeyS');
  click(e, 110, 110);
  e.doubleClick({ x: 110, y: 110, t: 0 });
  assert.equal(e.editing?.text, '핵심 공식');
  assert.ok(e.hidden.has(e.items[0].id), '고치는 동안 원래 글자 숨김');
  e.textInput('수정');
  key(e, 'Escape');
  assert.equal(/** @type {any} */ (e.items[0]).text, '수정');
  assert.equal(e.undoStack.at(-1)?.type, 'setAttr');
  e.undo();
  assert.equal(/** @type {any} */ (e.items[0]).text, '핵심 공식');
  e.doubleClick({ x: 110, y: 110, t: 0 });
  e.textInput('');
  key(e, 'Escape');
  assert.equal(e.items.length, 0, '비우면 지움');
});

test('칠판_여러장_자리와_되돌리기', () => {
  const b = engine({ docOptions: { mode: 'board' } });
  drag(b, line(100, 100, 200, 100, 4));
  b.addPage();
  assert.equal(b.doc.pages.length, 2);
  assert.equal(b.doc.activePage, 1);
  assert.equal(b.items.length, 0);
  drag(b, line(100, 300, 200, 300, 4));
  b.gotoPage(0);
  assert.equal(b.items.length, 1);
  b.undo();
  assert.equal(b.doc.activePage, 1, '되돌리는 장으로 옮겨 감');
  assert.equal(b.items.length, 0);
  b.undo();
  assert.equal(b.doc.pages.length, 1);
  assert.equal(b.doc.activePage, 0);
});

test('세션에서_문서_만들기와_렌더_변경_표시', () => {
  const opts = documentOptionsFromSession({ session: 17, board: false, monitor: { id: 1, bounds: { x: 1920, y: 0, w: 2880, h: 1800 }, scale: 1.5 }, layoutFingerprint: 'fp' });
  assert.deepEqual(opts, { id: 'draw-17', mode: 'draw', monitorId: 1, layoutFingerprint: 'fp', docSize: { w: 1920, h: 1200 }, physSize: { w: 2880, h: 1800 }, scale: 1.5, frameId: 17 });
  const e = new InkEngine({ docOptions: opts, viewport: { w: 1920, h: 1200 } });
  let calls = 0;
  const off = e.subscribe(() => calls++);
  const rev = e.revs.doc;
  drag(e, line(100, 100, 300, 100, 4));
  assert.equal(e.revs.doc, rev + 1);
  assert.equal(e.lastChange.kind, 'append', '획 하나 추가 = 아래층 증분 그리기');
  key(e, 'KeyK');
  assert.equal(e.lastChange.kind, 'full');
  assert.ok(calls > 0);
  off();
  const n = calls;
  key(e, 'KeyK');
  assert.equal(calls, n, '구독 해제');
});
