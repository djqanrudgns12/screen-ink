import test from 'node:test';
import assert from 'node:assert/strict';
import {
  MOSAIC_BLOCK_DEFAULT,
  annotDocOptions,
  annotEngineSettings,
  annotKeyRoute,
  annotTarget,
  anchorToCss,
  applyToolMods,
  clampMosaicBlock,
  collectTextFonts,
  cssToDoc,
  ensureAnnotationFontsLoaded,
  exportAnnotation,
  exportAnnotationAsync,
  exportTransform,
  layerView,
  matchCaptureAnnotKey,
  mosaicItem,
  mosaicSourcePlan,
  mosaicToPhys,
  normalizeAnnotTool,
  renderAnnotationLayer,
  shouldBlockCapturePointer,
  syncAnnotToolFromView,
  wheelStep,
} from './captureAnnotate.js';
import { createInkEngine, itemCount } from '../ink/inkEngine.js';
import { buildDrawKeymap } from '../ink/drawKeys.js';
import { withBBox } from '../ink/hitTest.js';

const MON150 = { id: 1, bounds: { x: 1920, y: 0, w: 2880, h: 1800 }, scale: 1.5 };
const MON100 = { id: 0, bounds: { x: 0, y: 0, w: 1920, h: 1080 }, scale: 1 };

/** 호출을 기록하는 가짜 2D 컨텍스트(필요한 것만). */
function recorder() {
  /** @type {any[]} */
  const log = [];
  /** @type {Record<string, any>} */
  const t = { measureText: (/** @type {string} */ s) => ({ width: s.length * 10 }) };
  for (const m of ['setTransform', 'clearRect', 'strokeRect', 'beginPath', 'closePath', 'moveTo', 'lineTo', 'quadraticCurveTo', 'ellipse', 'setLineDash', 'drawImage', 'save', 'restore', 'stroke', 'fill', 'fillRect', 'fillText']) {
    t[m] = (/** @type {any[]} */ ...a) => log.push([m, ...a, { alpha: t.globalAlpha, op: t.globalCompositeOperation }]);
  }
  t.getImageData = (/** @type {number} */ x, /** @type {number} */ y, /** @type {number} */ w, /** @type {number} */ h) => ({ data: new Uint8ClampedArray(w * h * 4) });
  return { ctx: /** @type {any} */ (t), log, names: () => log.map((e) => e[0]) };
}

test('도구 → 엔진: 도형은 손그림 + 수정 키 도형, 모자이크는 이 층이', () => {
  assert.deepEqual(annotTarget('rect'), { engineTool: 'pen', shape: 'rect', mosaic: false });
  assert.deepEqual(annotTarget('marker'), { engineTool: 'marker', shape: null, mosaic: false });
  assert.deepEqual(annotTarget('mosaic'), { engineTool: null, shape: null, mosaic: true });
  assert.deepEqual(annotTarget(null), { engineTool: null, shape: null, mosaic: false });
  // 수정 키 고정(Alt 점선은 그대로)
  const s = { x: 1, y: 2, t: 0, shiftKey: true, ctrlKey: false, metaKey: false, altKey: true };
  assert.deepEqual(applyToolMods(s, 'rect').sample, { ...s, shiftKey: false, ctrlKey: true, metaKey: false });
  assert.deepEqual(applyToolMods(s, 'arrow', 'macos').sample, { ...s, shiftKey: true, ctrlKey: false, metaKey: true });
  assert.equal(applyToolMods(s, 'ellipse').tab, true);
  assert.equal(applyToolMods(s, 'pen').sample, s, '펜은 사용자 수정 키 그대로');
});

test('도형 도구는 엔진에서 정말 그 도형이 된다(엔진 수정 0줄)', () => {
  for (const [tool, shape] of [['rect', 'rect'], ['arrow', 'arrow'], ['ellipse', 'ellipse']]) {
    const e = createInkEngine({ docOptions: annotDocOptions(1, { x: 300, y: 200, w: 400, h: 300 }, MON100), settings: annotEngineSettings({}) });
    e.setTool('pen');
    const go = (/** @type {'pointerDown'|'pointerMove'|'pointerUp'} */ fn, x, y, t) => {
      const { sample, tab } = applyToolMods({ x, y, t, button: 0, shiftKey: false, ctrlKey: false, metaKey: false, altKey: false }, tool);
      e.mods.tab = tab;
      e[fn](sample);
    };
    go('pointerDown', 10, 10, 0);
    go('pointerMove', 80, 50, 16);
    go('pointerMove', 160, 90, 32);
    go('pointerUp', 160, 90, 48);
    assert.equal(e.items.length, 1, tool);
    const it = /** @type {any} */ (e.items[0]);
    assert.equal(it.kind, 'shape', tool);
    assert.equal(it.shape, shape, tool);
  }
});

test('0.5초 멈춤 제안은 끈다(Enter = 캡처 복사와 겹침)', () => {
  assert.equal(annotEngineSettings({ 'draw.shapeOnHold': true, 'draw.color': 'B' })['draw.shapeOnHold'], false);
  const e = createInkEngine({ settings: annotEngineSettings({ 'draw.color': 'B', 'draw.width': 8 }) });
  assert.equal(e.config.shapeOnHold, false);
  assert.equal(e.color, 'B', '색·굵기는 판서와 같은 값');
  assert.equal(e.widthPx, 8);
});

test('문서 좌표: 범위 왼쪽 위 = 원점, 논리 px, 확대 없음', () => {
  const rect = { x: 2220, y: 300, w: 600, h: 450 };
  const o = annotDocOptions(7, rect, MON150);
  assert.deepEqual(o.docSize, { w: 400, h: 300 });
  assert.deepEqual(o.physSize, { w: 600, h: 450 });
  assert.equal(o.mode, 'capture');
  const anchor = { x: rect.x, y: rect.y };
  const a = anchorToCss(anchor, MON150);
  assert.deepEqual(a, { x: 200, y: 200 });
  assert.deepEqual(cssToDoc(250, 210, a), { x: 50, y: 10 });
  // 범위를 오른쪽 아래로 30 CSS px 옮겨도 주석은 같은 화면 자리(보기 원점이 그만큼 왼쪽 위로)
  assert.deepEqual(layerView(a, { x: 200, y: 200 }), { s: 1, tx: 0, ty: 0 });
  assert.deepEqual(layerView(a, { x: 230, y: 230 }), { s: 1, tx: -30, ty: -30 });
});

test('모자이크 항목·칸 크기(4~48)·물리 px 바꾸기', () => {
  assert.equal(mosaicItem({ x: 5, y: 5 }, { x: 6, y: 40 }, 10, 'm'), null, '너무 좁으면 없음');
  const m = /** @type {any} */ (mosaicItem({ x: 40, y: 30 }, { x: 10, y: 10 }, 10, 'm1'));
  assert.deepEqual([m.x, m.y, m.w, m.h, m.blockSize, m.kind], [10, 10, 30, 20, 10, 'mosaic']);
  assert.ok(m.bbox, '판정 상자(지우개)');
  assert.equal(clampMosaicBlock(2), 4);
  assert.equal(clampMosaicBlock(60), 48);
  assert.equal(clampMosaicBlock(NaN), MOSAIC_BLOCK_DEFAULT);
  // 150%: 논리 10 px 칸 = 물리 15 px, 결과 그림 기준(범위가 anchor보다 오른쪽 아래로 옮겨졌어도)
  const anchor = { x: 2220, y: 300 };
  assert.deepEqual(mosaicToPhys(m, anchor, { x: 2220, y: 300, w: 600, h: 450 }, 1.5), { x: 15, y: 15, w: 45, h: 30, block: 15 });
  assert.deepEqual(mosaicToPhys(m, anchor, { x: 2250, y: 330, w: 600, h: 450 }, 1.5), { x: -15, y: -15, w: 45, h: 30, block: 15 });
  assert.deepEqual(exportTransform(anchor, { x: 2250, y: 330, w: 1, h: 1 }, 1.5), [1.5, 0, 0, 1.5, -30, -30]);
});

test('휠 누적: 40 px마다 한 단계(맥 휠 한 칸), 방향이 바뀌면 처음부터', () => {
  assert.equal(wheelStep(0, -40).dir, 1, '맥 휠 한 칸 = 한 단계');
  let r = wheelStep(0, -30);
  assert.equal(r.dir, 0);
  r = wheelStep(r.acc, -30);
  assert.equal(r.dir, 1, '위로 = 키우기');
  assert.equal(r.acc, 0);
  r = wheelStep(-40, 60);
  assert.equal(r.dir, -1, '반대 방향 60 = 한 단계 아래');
});

test('주석 층: 물리 px 변환으로 그리고 모자이크는 자리를 비운 뒤 목록으로', () => {
  const stroke = withBBox({ id: 'it-1', kind: 'stroke', tool: 'pen', color: 'R', width: 4, opacity: 1, dashed: false, points: [{ x: 1, y: 1, p: 0.5, t: 0 }, { x: 20, y: 5, p: 0.5, t: 8 }] });
  const mosaic = withBBox({ id: 'it-2', kind: 'mosaic', x: 0, y: 0, w: 10, h: 10, blockSize: 10 });
  const marker = withBBox({ ...stroke, id: 'it-3', tool: 'marker', opacity: 0.38 });
  const r = recorder();
  const out = renderAnnotationLayer(r.ctx, /** @type {any} */ ([stroke, mosaic, marker]), { anchor: { x: 100, y: 50 }, rect: { x: 100, y: 50, w: 60, h: 30 }, scale: 2 });
  assert.equal(out.painted, 2);
  assert.deepEqual(out.mosaics, [{ x: 0, y: 0, w: 20, h: 20, block: 20 }]);
  const tf = r.log.filter((e) => e[0] === 'setTransform');
  assert.deepEqual(tf[1].slice(1, 7), [2, 0, 0, 2, 0, 0], '문서 → 물리 px');
  const clears = r.log.filter((e) => e[0] === 'clearRect').map((e) => e.slice(1, 5));
  assert.deepEqual(clears, [[0, 0, 60, 30], [0, 0, 10, 10]], '처음 전체 비움 + 모자이크 자리');
  // 순서: 펜 → 모자이크 자리 비움 → 형광펜(모자이크 위 획은 남는다)
  const order = r.names().filter((n) => n === 'stroke' || n === 'clearRect');
  assert.deepEqual(order, ['clearRect', 'stroke', 'clearRect', 'stroke']);
  // 형광펜은 multiply 없이 source-over(화면 미리보기와 같은 알파)
  const strokes = r.log.filter((e) => e[0] === 'stroke');
  assert.equal(strokes[1].at(-1).op, 'source-over');
});

test('결과 내보내기: 획이 없으면 층 없이 모자이크만, 있으면 범위 크기 RGBA', () => {
  const mosaic = withBBox({ id: 'm', kind: 'mosaic', x: 0, y: 0, w: 10, h: 10, blockSize: 10 });
  const made = [];
  const createCanvas = (/** @type {number} */ w, /** @type {number} */ h) => {
    const r = recorder();
    const c = { width: w, height: h, getContext: () => r.ctx };
    made.push(c);
    return c;
  };
  const opts = { anchor: { x: 0, y: 0 }, rect: { x: 0, y: 0, w: 40, h: 20 }, scale: 1, createCanvas };
  const only = exportAnnotation(/** @type {any} */ ([mosaic]), opts);
  assert.equal(only.rgba, null);
  assert.equal(made.length, 0, '캔버스를 만들지 않는다');
  assert.deepEqual(only.mosaics, [{ x: 0, y: 0, w: 10, h: 10, block: 10 }]);
  const text = withBBox({ id: 't', kind: 'text', color: 'B', fontFamily: 'IBM Plex Sans KR', fontSize: 24, x: 2, y: 2, w: 30, h: 31, text: '핵심' });
  const both = exportAnnotation(/** @type {any} */ ([text, mosaic]), opts);
  assert.equal(both.rgba?.length, 40 * 20 * 4);
  assert.equal(both.w, 40);
  assert.equal(made[0].width, 0, '다 쓴 캔버스는 비운다');
});

test('모자이크 미리보기 원본: 정지 이미지를 문서 원점에서 시작하게', () => {
  assert.deepEqual(mosaicSourcePlan({ x: 2220, y: 300 }, MON150), { w: 2580, h: 1500, dx: -300, dy: -300, dw: 2880, dh: 1800 });
  // 범위가 왼쪽 모니터에서 시작(원점이 이 모니터 왼쪽 밖) → 이미지를 오른쪽으로
  assert.deepEqual(mosaicSourcePlan({ x: 1800, y: 100 }, MON150), { w: 3000, h: 1700, dx: 120, dy: -100, dw: 2880, dh: 1800 });
  assert.equal(mosaicSourcePlan({ x: 5000, y: 0 }, MON150), null);
});

test('키: 글자 입력 중은 상자, 실행 취소·다시 실행은 설정 키로, 도구 중 방향키는 삼킴', () => {
  const keymap = buildDrawKeymap([{ id: 'd-undo', combo: 'Ctrl+Z' }, { id: 'd-redo', combo: 'Ctrl+Y' }]);
  const k = (/** @type {string} */ code, mods = {}) => ({ code, key: code.replace('Key', '').toLowerCase(), ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods });
  const base = { editing: false, toolActive: true, keymap };
  assert.equal(annotKeyRoute(k('Enter'), { ...base, editing: true }), 'text');
  assert.equal(annotKeyRoute(k('KeyZ', { ctrlKey: true }), base), 'undo');
  assert.equal(annotKeyRoute(k('KeyY', { ctrlKey: true }), base), 'redo');
  assert.equal(annotKeyRoute(k('ArrowLeft'), base), 'swallow');
  assert.equal(annotKeyRoute(k('ArrowLeft'), { ...base, toolActive: false }), 'capture', '도구 해제 = 범위 조정');
  assert.equal(annotKeyRoute(k('Enter'), base), 'capture', 'Enter·Ctrl+S 판정은 Rust');
  // 도구를 고른 동안 첫 Esc = 도구만 해제(주석 유지), 도구가 없으면 Esc = 캡처 취소(Rust)
  assert.equal(annotKeyRoute({ ...k('Escape'), key: 'Escape' }, base), 'release');
  assert.equal(annotKeyRoute({ ...k('Escape'), key: 'Escape' }, { ...base, toolActive: false }), 'capture');
  assert.equal(annotKeyRoute({ ...k('Escape'), key: 'Escape' }, { ...base, editing: true }), 'text', '글자 입력 중 Esc = 글자 확정');
  assert.equal(annotKeyRoute(k('KeyS', { ctrlKey: true }), base), 'capture');
  assert.equal(annotKeyRoute({ ...k('KeyZ', { ctrlKey: true }), isComposing: true }, base), 'capture', '한글 조합 중은 엔진 키 아님');
  // 키를 바꾸면 바로 반영(R-KEY-6)
  const mine = buildDrawKeymap([{ id: 'd-undo', combo: 'Ctrl+Shift+U' }]);
  assert.equal(annotKeyRoute(k('KeyZ', { ctrlKey: true }), { ...base, keymap: mine }), 'capture');
  assert.equal(annotKeyRoute(k('KeyU', { ctrlKey: true, shiftKey: true }), { ...base, keymap: mine }), 'undo');
});

test('모자이크는 엔진 명령(add)으로 쌓여 실행 취소·지우개가 그대로 된다', () => {
  const e = createInkEngine({ docOptions: annotDocOptions(1, { x: 0, y: 0, w: 400, h: 300 }, MON100), settings: annotEngineSettings({}) });
  const m = /** @type {any} */ (mosaicItem({ x: 10, y: 10 }, { x: 60, y: 50 }, 10, e.nextId()));
  e.push({ type: 'add', page: e.doc.activePage, item: m });
  assert.equal(itemCount(e.doc), 1);
  e.runAction('d-undo');
  assert.equal(itemCount(e.doc), 0);
  e.runAction('d-redo');
  assert.equal(itemCount(e.doc), 1);
  e.setTool('eraser');
  e.pointerDown({ x: 30, y: 30, t: 0, button: 0 });
  e.pointerUp({ x: 30, y: 30, t: 10, button: 0 });
  assert.equal(itemCount(e.doc), 0, '지우개가 모자이크 상자를 지운다');
});

test('확정 뒤 도구 한 글자 키(R·O·A·P·H·M·T·E), 크기([·], ⇧+[·]), 색 키, 스포이트 Esc', () => {
  const k = (/** @type {string} */ code, mods = {}) => ({ code, key: code.replace('Key', '').toLowerCase(), ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods });
  // 확정 상태에서 도구 키 8종
  assert.deepEqual(matchCaptureAnnotKey(k('KeyR'), { confirmed: true }), { type: 'tool', id: 'rect' });
  assert.deepEqual(matchCaptureAnnotKey(k('KeyO'), { confirmed: true }), { type: 'tool', id: 'ellipse' });
  assert.deepEqual(matchCaptureAnnotKey(k('KeyA'), { confirmed: true }), { type: 'tool', id: 'arrow' });
  assert.deepEqual(matchCaptureAnnotKey(k('KeyP'), { confirmed: true }), { type: 'tool', id: 'pen' });
  assert.deepEqual(matchCaptureAnnotKey(k('KeyH'), { confirmed: true }), { type: 'tool', id: 'marker' });
  assert.deepEqual(matchCaptureAnnotKey(k('KeyM'), { confirmed: true }), { type: 'tool', id: 'mosaic' });
  assert.deepEqual(matchCaptureAnnotKey(k('KeyT'), { confirmed: true }), { type: 'tool', id: 'text' });
  assert.deepEqual(matchCaptureAnnotKey(k('KeyE'), { confirmed: true }), { type: 'tool', id: 'eraser' });
  // 고르기 중에는 도구 키를 가로채지 않는다
  assert.equal(matchCaptureAnnotKey(k('KeyR'), { confirmed: false }), null);
  // 패널이 열려 있으면 R/K/W 모두 색 키 우선
  assert.deepEqual(matchCaptureAnnotKey(k('KeyR'), { confirmed: true, toolActive: true, panelOpen: true }), { type: 'color', code: 'R' });
  assert.deepEqual(matchCaptureAnnotKey(k('KeyK'), { confirmed: true, toolActive: true, panelOpen: true }), { type: 'color', code: 'K' });
  // 도구 활성 중 비충돌 색 키(B 등) 및 크기 조절
  assert.deepEqual(matchCaptureAnnotKey(k('KeyB'), { confirmed: true, toolActive: true, panelOpen: false }), { type: 'color', code: 'B' });
  assert.deepEqual(matchCaptureAnnotKey(k('BracketLeft'), { confirmed: true, toolActive: true }), { type: 'size', dir: -1, large: false });
  assert.deepEqual(matchCaptureAnnotKey(k('BracketRight', { shiftKey: true }), { confirmed: true, toolActive: true }), { type: 'size', dir: 1, large: true });
  // 스포이트 Esc 취소
  assert.deepEqual(matchCaptureAnnotKey({ ...k('Escape'), key: 'Escape' }, { eyedropper: true }), { type: 'cancelEyedropper' });
  // 조합 중·글자 입력 중·수정키는 무시
  assert.equal(matchCaptureAnnotKey(k('KeyR', { ctrlKey: true }), { confirmed: true }), null);
  assert.deepEqual(matchCaptureAnnotKey(k('KeyR'), { confirmed: true, editing: true }), { type: 'text' });
});

test('글꼴 전달·document.fonts.load() 대기·실패 시 기본 글꼴 폴백 굽기(R2-TXT-3)', async () => {
  const t1 = withBBox({ id: 't1', kind: 'text', color: 'R', fontFamily: 'Paperlogy', fontWeight: 600, fontSize: 28, x: 4, y: 8, w: 40, h: 36, text: '제목' });
  const t2 = withBBox({ id: 't2', kind: 'text', color: 'B', fontFamily: 'BrokenFont', fontWeight: 400, fontSize: 20, x: 4, y: 50, w: 40, h: 26, text: '본문' });

  const fontsSpec = collectTextFonts([t1, t2]);
  assert.equal(fontsSpec.length, 2);
  assert.equal(fontsSpec[0].fontFamily, 'Paperlogy');
  assert.equal(fontsSpec[0].fontWeight, 600);

  /** @type {string[]} */
  const loadedCalls = [];
  const fakeFonts = {
    load: async (/** @type {string} */ desc) => {
      loadedCalls.push(desc);
      if (desc.includes('BrokenFont')) throw new Error('font load fail');
      return [];
    },
    ready: Promise.resolve(),
  };

  const made = [];
  const createCanvas = (/** @type {number} */ w, /** @type {number} */ h) => {
    const r = recorder();
    const c = { width: w, height: h, getContext: () => r.ctx, _r: r };
    made.push(c);
    return c;
  };

  const out = await exportAnnotationAsync(/** @type {any} */ ([t1, t2]), {
    anchor: { x: 0, y: 0 },
    rect: { x: 0, y: 0, w: 100, h: 100 },
    scale: 1,
    createCanvas,
    fonts: fakeFonts,
    timeoutMs: 100,
  });

  assert.equal(loadedCalls.length, 2);
  assert.equal(out.fontFallback, true, 'BrokenFont 실패 시 폴백 플래그');
  assert.deepEqual(out.failedFonts, ['BrokenFont']);
  assert.equal(out.rgba?.length, 100 * 100 * 4);
  const fillCalls = made[0]._r.log.filter((e) => e[0] === 'fillText');
  assert.equal(fillCalls.length, 2);

  // 타임아웃 시에도 멈추지 않고 폴백으로 내보냄
  const slowFonts = {
    load: () => new Promise(() => {}),
  };
  const slowOut = await ensureAnnotationFontsLoaded(/** @type {any} */ ([t1]), { fonts: slowFonts, timeoutMs: 15 });
  assert.equal(slowOut.ok, false);
  assert.deepEqual(slowOut.failed, ['Paperlogy']);
  assert.equal(/** @type {any} */ (slowOut.items[0]).fontFamily, 'Freesentation');
});

test('글 상자 편집 중 초점이 상자 밖이어도 Esc는 글자 확정(commitText), 조합 중·수정키는 제외', () => {
  const k = (/** @type {string} */ code, mods = {}) => ({ code, key: code.replace('Key', '').toLowerCase(), ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods });
  assert.deepEqual(matchCaptureAnnotKey({ ...k('Escape'), key: 'Escape' }, { confirmed: true, toolActive: true, editing: true }), { type: 'commitText' });
  assert.equal(matchCaptureAnnotKey({ ...k('Escape'), key: 'Escape', isComposing: true }, { confirmed: true, toolActive: true, editing: true }), null);
  assert.deepEqual(matchCaptureAnnotKey({ ...k('Escape'), key: 'Escape', ctrlKey: true }, { confirmed: true, toolActive: true, editing: true }), { type: 'text' });
  assert.deepEqual(matchCaptureAnnotKey(k('KeyR'), { confirmed: true, toolActive: true, editing: true }), { type: 'text' });
});

test('세션 단위 주석 도구 동기화(W5): 한쪽 창 도구 선택 시 다른 창도 동기화되고 포인터 드래그를 차단한다', () => {
  assert.equal(normalizeAnnotTool('pen'), 'pen');
  assert.equal(normalizeAnnotTool('unknown'), null);
  assert.equal(normalizeAnnotTool(null), null);

  // 오른쪽 창에서 펜을 골랐을 때 왼쪽 창은 view.annotTool === 'pen'을 받아 동기화된다
  assert.equal(syncAnnotToolFromView(null, { phase: 'confirmed', annotTool: 'pen' }), 'pen');
  assert.equal(syncAnnotToolFromView('pen', { phase: 'confirmed', annotTool: null }), null);
  // hover로 돌아가면 도구 해제
  assert.equal(syncAnnotToolFromView('pen', { phase: 'hover', annotTool: 'pen' }), null);
  // annotTool 필드가 없는 구형 view 호환
  assert.equal(syncAnnotToolFromView('pen', { phase: 'confirmed' }), 'pen');

  // 포인터 차단: 로컬 annotTool 또는 view.annotTool 둘 중 하나라도 있으면 범위 드래그 차단
  assert.equal(shouldBlockCapturePointer('pen', { phase: 'confirmed', annotTool: null }), true);
  assert.equal(shouldBlockCapturePointer(null, { phase: 'confirmed', annotTool: 'pen' }), true);
  assert.equal(shouldBlockCapturePointer(null, { phase: 'confirmed', annotTool: null }), false);
});

test('4K 200% + FHD 100% 혼합 배율 경계 캡처에서 양쪽 모니터 주석 좌표가 물리 좌표와 정확히 일치한다', () => {
  const monLeft = { id: 0, bounds: { x: 0, y: 0, w: 3840, h: 2160 }, scale: 2 };
  const monRight = { id: 1, bounds: { x: 3840, y: 0, w: 1920, h: 1080 }, scale: 1 };
  const rect = { x: 3500, y: 320, w: 600, h: 400 };
  const anchor = { x: rect.x, y: rect.y };

  // 왼쪽 모니터(200%): 물리 (3590, 430) -> CSS (1795, 215), anchor CSS (1750, 160) -> 문서 (45, 55) -> export 2배 -> (90, 110)
  const leftAnchorCss = anchorToCss(anchor, monLeft);
  assert.deepEqual(leftAnchorCss, { x: 1750, y: 160 });
  const leftDoc = cssToDoc(3590 / 2, 430 / 2, leftAnchorCss);
  assert.deepEqual(leftDoc, { x: 45, y: 55 });
  const leftTf = exportTransform(anchor, rect, monLeft.scale);
  assert.deepEqual([leftDoc.x * leftTf[0] + leftTf[4], leftDoc.y * leftTf[3] + leftTf[5]], [90, 110]);

  // 오른쪽 모니터(100%): 물리 (3900, 480) -> CSS (60, 480), anchor CSS (-340, 320) -> 문서 (400, 160) -> export 1배 -> (400, 160)
  const rightAnchorCss = anchorToCss(anchor, monRight);
  assert.deepEqual(rightAnchorCss, { x: -340, y: 320 });
  const rightDoc = cssToDoc(3900 - 3840, 480, rightAnchorCss);
  assert.deepEqual(rightDoc, { x: 400, y: 160 });
  const rightTf = exportTransform(anchor, rect, monRight.scale);
  assert.deepEqual([rightDoc.x * rightTf[0] + rightTf[4], rightDoc.y * rightTf[3] + rightTf[5]], [400, 160]);
});
