import test from 'node:test';
import assert from 'node:assert/strict';
import { createFakeClient } from './fake.js';
import { keycaps } from './index.js';

test('확정본 57칸이 행으로 나오고 Snipaste 겹침 4개를 요약한다', async () => {
  const c = createFakeClient();
  const s = await c.getState();
  assert.equal(s.shortcuts.length, 57);
  assert.equal(s.conflicts.count, 4);
  assert.equal(s.conflicts.app, 'Snipaste');
});

test('맥에서는 창 항상 위 칸이 빠진다', async () => {
  const s = await createFakeClient({ platform: 'macos' }).getState();
  assert.equal(s.shortcuts.length, 56);
  assert.ok(!s.shortcuts.some((r) => r.id === 'topmost'));
});

test('겹치는 키는 옛 키를 지키고 줄마다 추천을 준다, 성공은 되돌릴 수 있다', async () => {
  const c = createFakeClient();
  const bad = await c.setShortcut('active', 'Alt+1');
  assert.equal(bad.ok, false);
  assert.equal(bad.suggestions.length, 3);
  assert.equal((await c.getState()).shortcuts.find((r) => r.id === 'active').combo, 'Shift+F2');
  const good = await c.setShortcut('active', 'Ctrl+Alt+F2');
  assert.equal(good.ok, true);
  await c.undo();
  assert.equal((await c.getState()).shortcuts.find((r) => r.id === 'active').combo, 'Shift+F2');
});

test('가져오기는 미리 보기 뒤 반영, 되돌리기 한 번에 원래대로', async () => {
  const c = createFakeClient();
  const p = await c.previewImport();
  assert.equal(p.ok, true);
  assert.equal(p.changes.length, 3);
  assert.equal((await c.getState()).settings['draw.width'], 4, '미리 보기는 바꾸지 않는다');
  await c.applyImport(p.path);
  assert.equal((await c.getState()).settings['draw.width'], 8);
  await c.undo();
  const s = await c.getState();
  assert.equal(s.settings['draw.width'], 4);
  assert.equal(s.shortcuts.find((r) => r.id === 'active').combo, 'Shift+F2');
  assert.equal(s.location.portable, false);
});

test('캡처 진입은 오버레이 세션·모드 섬, 취소는 끝 섬과 idle', async () => {
  const c = createFakeClient({ frameUrl: 'frame.png' });
  const seen = [];
  c.subscribe((e) => seen.push(e));
  await c.run('capture.region');
  const ov = seen.find((e) => e.type === 'overlay').overlay;
  assert.equal(ov.kind, 'region');
  assert.equal(ov.monitors[0].frameUrl, 'frame.png');
  assert.equal((await c.getState()).mode.mode, 'capture');
  assert.equal(seen.find((e) => e.type === 'island').island.kind, 'mode');
  await c.overlayReady(ov.session);
  assert.deepEqual(c.readyLog, [ov.session]);
  await c.run('capture.cancel');
  assert.equal((await c.getState()).mode.mode, 'idle');
  assert.equal(seen.at(-1).island.kind, 'end');
});

test('고르기 흉내: 휠은 단계, 클릭은 확정, Enter는 결과 섬', async () => {
  const c = createFakeClient();
  const seen = [];
  c.subscribe((e) => seen.push(e));
  await c.run('capture.region');
  await c.captureInput({ type: 'move', x: 800, y: 500 });
  let v = seen.filter((e) => e.type === 'capture').at(-1).view;
  assert.deepEqual(v.path, ['화면', '창', '표', '2행']);
  assert.equal(v.rect.h, 40);
  await c.captureInput({ type: 'wheel', dy: -1 });
  v = seen.filter((e) => e.type === 'capture').at(-1).view;
  assert.equal(v.level, 2);
  await c.captureInput({ type: 'down', x: 800, y: 500, button: 0 });
  await c.captureInput({ type: 'up', x: 800, y: 500, button: 0 });
  assert.equal(seen.filter((e) => e.type === 'capture').at(-1).view.phase, 'confirmed');
  await c.captureInput({ type: 'key', key: 'Enter', code: 'Enter', ctrl: false, shift: false, alt: false });
  assert.equal(seen.at(-1).island.kind, 'result');
  assert.equal((await c.getState()).mode.mode, 'idle');
});

test('선택 툴바 단추(command)는 키와 같은 결과', async () => {
  const c = createFakeClient();
  const seen = [];
  c.subscribe((e) => seen.push(e));
  await c.run('capture.region');
  await c.captureInput({ type: 'move', x: 800, y: 500 });
  await c.captureInput({ type: 'down', x: 800, y: 500, button: 0 });
  await c.captureInput({ type: 'up', x: 800, y: 500, button: 0 });
  await c.captureInput({ type: 'command', name: 'copy' });
  assert.equal(seen.at(-1).island.text, '복사했어요');
  await c.run('capture.region');
  await c.captureInput({ type: 'command', name: 'save' });
  assert.equal(seen.at(-1).island.text, '저장했어요');
  await c.run('capture.region');
  await c.captureInput({ type: 'command', name: 'cancel' });
  assert.equal((await c.getState()).mode.mode, 'idle');
});

test('툴바 [고정]은 핀을 만들고 트레이 개수가 늘고, 모두 닫기로 0', async () => {
  const c = createFakeClient();
  await c.run('capture.region');
  await c.captureInput({ type: 'move', x: 800, y: 500 });
  await c.captureInput({ type: 'command', name: 'pin' });
  let s = await c.getState();
  assert.equal(s.mode.mode, 'idle');
  assert.equal(s.pins, 1);
  const info = await c.pinInfo(1);
  assert.equal(info.w, 720);
  assert.equal(await c.pinZoom(1, -1, 0.5, 0.5), 125);
  await c.run('pins.closeAll');
  s = await c.getState();
  assert.equal(s.pins, 0);
});

test('키캡은 Windows에서 Shift, 맥에서 기호', () => {
  assert.deepEqual(keycaps('Alt+Shift+D'), ['Alt', 'Shift', 'D']);
  assert.deepEqual(keycaps('Option+Shift+1', 'macos'), ['⌥', '⇧', '1']);
});

/** 항목 n개 InkDocument(모양만). */
const inkDoc = (n) => ({ schemaVersion: 1, pages: [{ id: 'page-1', items: Array.from({ length: n }, (_, i) => ({ id: `it-${i}`, kind: 'stroke' })) }] });

test('판서 진입: 세션이 상태보다 먼저 오고 모드 섬은 판서 · 펜 4px', async () => {
  const c = createFakeClient({ frameUrl: 'frame.png' });
  const seen = [];
  c.subscribe((e) => seen.push(e));
  await c.run('draw');
  const drawAt = seen.findIndex((e) => e.type === 'draw');
  const stateAt = seen.findIndex((e) => e.type === 'state');
  assert.ok(drawAt >= 0 && drawAt < stateAt, '세션 먼저, 상태 나중');
  const ds = seen[drawAt].draw;
  assert.equal(ds.board, false);
  assert.equal(ds.monitor.frameUrl, 'frame.png');
  assert.deepEqual(ds.initialView, { s: 1, tx: 0, ty: 0 });
  assert.equal(ds.keptCompatible, 'none');
  const s = await c.getState();
  assert.deepEqual([s.mode.mode, s.mode.session, s.mode.board, s.mode.monitorId], ['draw', ds.session, false, 0]);
  const isl = seen.at(-1).island;
  assert.deepEqual([isl.kind, isl.variant, isl.text, isl.penColor, isl.penWidth], ['mode', 'draw', '판서 · 펜', 'R', '4px']);
  await c.drawReady(ds.session, 12);
  await c.drawReady(ds.session + 1, 12);
  assert.equal(c.drawLog.filter((l) => l.type === 'ready').length, 1);
});

test('칠판은 배경 주소 없이 흰 바탕과 쪽 표시', async () => {
  const c = createFakeClient({ frameUrl: 'frame.png' });
  const seen = [];
  c.subscribe((e) => seen.push(e));
  await c.run('board');
  const ds = seen.find((e) => e.type === 'draw').draw;
  assert.equal(ds.board, true);
  assert.equal(ds.monitor.frameUrl, undefined);
  assert.deepEqual([seen.at(-1).island.text, seen.at(-1).island.value], ['칠판 · 흰색', '1 / 1']);
});

test('끝내기: 항목이 있으면 보관 + 섬, 다음 판서에서 같은 모니터로 복원', async () => {
  const c = createFakeClient();
  const seen = [];
  c.subscribe((e) => seen.push(e));
  await c.run('draw');
  let s = await c.getState();
  await c.drawFinish({ session: s.mode.session, keep: true, kept: { document: inkDoc(2) } });
  assert.equal((await c.getState()).mode.mode, 'idle');
  assert.equal(seen.at(-1).island.text, '판서를 보관했어요 · Alt+Shift+1');
  assert.equal(c.keptInk.document.pages[0].items.length, 2);

  await c.run('draw');
  const ds = seen.filter((e) => e.type === 'draw').at(-1).draw;
  assert.deepEqual([ds.hasKept, ds.keptCompatible], [true, 'same']);
  assert.equal(seen.at(-1).island.detail, 'Ctrl+Shift+R 이전 판서');
  const before = seen.length;
  const r = await c.drawRestore(ds.session);
  assert.equal(r.status, 'same');
  assert.equal(r.kept.document.pages[0].items.length, 2);
  assert.equal(seen.length, before, '불러왔어요 문구는 엔진 몫(restoreKept → drawIsland)');
  // 빈 판서로 끝내면 덮지 않고, 보관 섬도 없다
  await c.drawFinish({ session: ds.session, keep: true, kept: { document: inkDoc(0) } });
  assert.equal(seen.at(-1).island.kind, 'end');
  assert.equal(c.keptInk.document.pages[0].items.length, 2);
  // 설정을 끄면 보관하지 않고 있던 것도 지운다
  await c.updateSetting('draw.keep', false);
  await c.run('draw');
  s = await c.getState();
  await c.drawFinish({ session: s.mode.session, keep: true, kept: { document: inkDoc(3) } });
  assert.equal(c.keptInk, null);
});

test('먼저 끝난 세션(잠금·다른 모드 키)의 늦은 끝은 한 번만 조용히 보관', async () => {
  const c = createFakeClient();
  const seen = [];
  c.subscribe((e) => seen.push(e));
  await c.run('draw');
  const session = (await c.getState()).mode.session;
  c.simulateLock();
  assert.equal((await c.getState()).mode.mode, 'idle');
  await c.drawFinish({ session, keep: true, kept: { document: inkDoc(1) } });
  assert.equal(c.keptInk.document.pages[0].items.length, 1);
  assert.equal(seen.at(-1).island.kind, 'end', '잠금이면 보관 섬 없음');
  await c.drawFinish({ session, keep: true, kept: { document: inkDoc(5) } });
  assert.equal(c.keptInk.document.pages[0].items.length, 1, '두 번째 끝은 무시');
  // 판서 중 캡처 = 판서를 끝내고 캡처
  await c.run('draw');
  const drawing = (await c.getState()).mode.session;
  await c.run('capture.region');
  assert.equal((await c.getState()).mode.mode, 'capture');
  await c.drawFinish({ session: drawing, keep: true, kept: { document: inkDoc(4) } });
  assert.equal(c.keptInk.document.pages[0].items.length, 4);
});

test('직전 판서 열기: 열자마자 복원, 판서 중이면 같은 세션을 복원 표시로', async () => {
  const c = createFakeClient();
  const seen = [];
  c.subscribe((e) => seen.push(e));
  await c.run('draw.restore');
  const first = seen.filter((e) => e.type === 'draw').at(-1).draw;
  assert.equal(first.restoreOnOpen, true);
  assert.equal((await c.drawRestore(first.session)).status, 'none');
  assert.equal(seen.at(-1).island.text, '불러올 판서가 없어요');
  await c.run('draw.restore');
  const again = seen.filter((e) => e.type === 'draw').at(-1).draw;
  assert.deepEqual([again.session, again.restoreOnOpen], [first.session, true]);
});

test('내보내기는 늘 모니터 전체 원본 바이트 — 길이·크기를 검사하고 복사·저장 섬', async () => {
  const c = createFakeClient();
  const seen = [];
  c.subscribe((e) => seen.push(e));
  const full = 1920 * 1080 * 4;
  assert.equal((await c.drawExport('copy', new Uint8Array(full), { w: 1920, h: 1080 })).ok, false, '판서 중이 아니면 거절');
  await c.run('draw');
  const session = (await c.getState()).mode.session;
  assert.deepEqual(await c.drawExport('copy', new Uint8ClampedArray(full), { w: 1920, h: 1080, session }), { ok: true });
  assert.deepEqual([seen.at(-1).island.text, seen.at(-1).island.value], ['복사했어요', '1920×1080']);
  const saved = await c.drawExport('save', new ArrayBuffer(full), { w: 1920, h: 1080 });
  assert.equal(saved.ok, true);
  assert.match(saved.name, /\.png$/);
  assert.equal((await c.drawExport('save', new Uint8Array(full - 1), { w: 1920, h: 1080 })).ok, false, '길이가 틀리면 거절');
  assert.deepEqual(await c.drawExport('copy', new Uint8Array(32), { w: 4, h: 2 }), { ok: false, error: 'not monitor size' }, '확대 중 보이는 부분만은 거절');
  assert.equal((await c.getState()).mode.mode, 'draw', '내보내기는 판서를 끝내지 않는다');
});

test('판서 섬 조각과 안내, 펜 지연 기록', async () => {
  const c = createFakeClient();
  const seen = [];
  c.subscribe((e) => seen.push(e));
  await c.drawIsland({ zoom: 2 });
  assert.equal(seen.length, 0, '판서 중이 아니면 무시');
  await c.run('draw');
  await c.drawIsland({ color: 'G', zoom: 1.75, notice: '수평으로 맞췄어요' });
  const [mode, notice] = seen.slice(-2).map((e) => e.island);
  assert.deepEqual([mode.kind, mode.penColor, mode.value], ['mode', 'G', '×1.75']);
  assert.deepEqual([notice.kind, notice.text], ['info', '수평으로 맞췄어요']);
  await c.drawIsland({ tool: 'eraser', zoom: 2 });
  assert.deepEqual([seen.at(-1).island.text, seen.at(-1).island.penColor, seen.at(-1).island.value], ['판서 · 지우개', 'G', '×2.0']);
  await c.drawPenMetrics({ n: 120, p50: 7.5, p95: 13.25, max: 40 });
  assert.equal(c.drawLog.at(-1).line, 'pen latency n=120 p50=7.5 p95=13.3 max=40.0 ms');

  // 멈춤 도형 제안 → 섬 [바꾸기] → 오버레이 엔진 명령
  await c.drawIsland({ suggestion: '원으로 바꿀까요? · Enter' });
  assert.deepEqual([seen.at(-1).island.text, seen.at(-1).island.button.action], ['원으로 바꿀까요? · Enter', 'draw.acceptSuggestion']);
  await c.run('draw.acceptSuggestion');
  const cmd = seen.at(-1);
  assert.deepEqual([cmd.type, cmd.command, cmd.session], ['drawCommand', 'acceptSuggestion', (await c.getState()).mode.session]);
  await c.drawIsland({ notice: '판서를 모두 지웠어요', noticeAction: 'undo' });
  assert.equal(seen.at(-1).island.button.action, 'draw.undo');
  assert.equal(seen.at(-2).island.button, undefined, '제안 없이 오면 걷힌다');
});

test('결과 섬 HEX 칩과 주황 잔상 0.4초 이벤트를 흉내 낸다', async () => {
  const c = createFakeClient();
  const seen = [];
  c.subscribe((e) => seen.push(e));

  await c.run('capture.region');
  await c.captureInput({ type: 'move', x: 800, y: 500 });
  await c.captureInput({ type: 'command', name: 'copy' });

  const afterimage = seen.find((e) => e.type === 'afterimage')?.afterimage;
  assert.ok(afterimage, '복사 확정 직후 주황 잔상 이벤트가 온다');
  assert.equal(afterimage.durationMs, 400);
  assert.equal(afterimage.rect.w, 720);

  const resultIsland = seen.at(-1).island;
  assert.deepEqual(resultIsland.chip, { label: '#FFFFFF 복사', action: 'capture.copyHex' });

  await c.run(resultIsland.chip.action);
  assert.equal(seen.at(-1).island.kind, 'result');
  assert.equal(seen.at(-1).island.text, '#FFFFFF 복사했어요');

  // 취소에는 잔상이 뜨지 않는다
  seen.length = 0;
  await c.run('capture.region');
  await c.captureInput({ type: 'command', name: 'cancel' });
  assert.equal(seen.some((e) => e.type === 'afterimage'), false);
});
