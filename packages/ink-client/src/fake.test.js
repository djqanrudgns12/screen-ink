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

  // 4초 물러남(idleDim)·커서 회피(avoid) 전달 + 칠판 색 점
  await c.drawIsland({ idleDim: true, avoid: true });
  assert.equal(seen.at(-1).island.idleDim, true);
  assert.equal(seen.at(-1).island.avoid, true);
  await c.drawIsland({ background: 'black', color: 'W', idleDim: false, avoid: false });
  assert.deepEqual(
    [seen.at(-1).island.variant, seen.at(-1).island.penColor, seen.at(-1).island.text, seen.at(-1).island.idleDim, seen.at(-1).island.avoid],
    ['board', 'W', '칠판 · 검은색', false, false],
  );
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

test('이어받기 흉내: 결과 섬 [판서로]는 고른 범위를 꽉 차게, 확대 중 판서는 그 배율 그대로', async () => {
  const c = createFakeClient();
  const seen = [];
  c.subscribe((e) => seen.push(e));

  // 결과 섬 [판서로] — 2행(720×40)을 화면 88%에: min(0.88·1920/720, 0.88·1080/40) = 2.346… → 2.34
  await c.run('capture.region');
  await c.captureInput({ type: 'move', x: 800, y: 500 });
  await c.captureInput({ type: 'command', name: 'copy' });
  const result = seen.at(-1).island;
  assert.deepEqual(result.secondaryButton, { label: '판서로', action: 'capture.toDraw' });
  assert.equal(result.secondaryButtonKey, 'Alt+1');
  await c.run(result.secondaryButton.action);
  const draw = seen.filter((e) => e.type === 'draw').at(-1).draw;
  assert.equal(draw.initialView.s, 2.34);
  assert.equal((await c.getState()).mode.mode, 'draw');
  assert.equal(seen.filter((e) => e.type === 'island').at(-1).island.value, '×2.34');
  // 한 번 쓰면 없다
  await c.drawFinish({ session: draw.session, keep: false });
  await c.run('capture.toDraw');
  assert.equal(seen.at(-1).island.text, '판서로 넘길 캡처가 없어요');

  // 확대 중 판서 = 가운데 확대 그대로(휠 여덟 칸 = 두 배 → ×4)
  await c.run('zoom');
  await c.zoomInput({ type: 'wheel', dy: -8 });
  await c.run('draw');
  const fromZoom = seen.filter((e) => e.type === 'draw').at(-1).draw;
  assert.deepEqual(fromZoom.initialView, { s: 4, tx: -2880, ty: -1620 });
  assert.equal(seen.filter((e) => e.type === 'zoom').at(-1).zoom, null, '확대는 끝난다');
  await c.drawFinish({ session: fromZoom.session, keep: false });

  // 키로 연 판서는 1×
  await c.run('draw');
  assert.deepEqual(seen.filter((e) => e.type === 'draw').at(-1).draw.initialView, { s: 1, tx: 0, ty: 0 });
});

test('온보딩 단계와 맥 권한 흉내: perm.refresh는 바뀔 때만 알리고 settings.reveal은 무시한다', async () => {
  const c = createFakeClient({
    platform: 'macos',
    permissions: { screen: false, accessibility: false, screenRevoked: true, accessibilityRevoked: false },
  });
  const seen = [];
  c.subscribe((e) => seen.push(e));

  assert.equal((await c.updateSetting('onboarding.step', 2)).ok, true);
  assert.equal((await c.updateSetting('onboarding.step', 9)).ok, false);
  assert.equal((await c.getState()).settings['onboarding.step'], 2);

  await c.run('perm.refresh');
  assert.equal(seen.filter((e) => e.type === 'state').length, 1, '권한 변화가 없으면 state를 다시 보내지 않는다');

  await c.run('perm.accessibility');
  await c.run('perm.refresh');
  let s = await c.getState();
  assert.equal(s.permissions.accessibility, true);
  assert.equal(s.permissions.screen, false);

  await c.run('perm.screen');
  await c.run('perm.refresh');
  s = await c.getState();
  assert.equal(s.permissions.screen, true);
  assert.equal(s.permissions.screenRevoked, false);

  // screenNeedsRelaunch면 app.relaunch로 켜진다
  const c2 = createFakeClient({
    platform: 'macos',
    permissions: { screen: false, accessibility: true },
    screenNeedsRelaunch: true,
  });
  await c2.run('perm.screen');
  await c2.run('perm.refresh');
  assert.equal((await c2.getState()).permissions.screen, false);
  await c2.run('app.relaunch');
  assert.equal((await c2.getState()).permissions.screen, true);

  // settings.reveal은 섬 알림 없이 조용히 무시된다
  const beforeReveal = seen.length;
  await c.run('settings.reveal');
  assert.equal(seen.length, beforeReveal);
});

test('맥 미리보기 OS 분기: 추천 키·예외 앱·경로·가져오기·섬 표기·시스템 글꼴', async () => {
  const mac = createFakeClient({ platform: 'macos', frameUrl: 'frame.png' });
  const s = await mac.getState();
  assert.equal(s.settings['output.folder'], '~/Pictures/Shookshok');
  assert.deepEqual(s.settings['capture.excluded'], [{ exe: 'LICEcap', mode: 'exclude', auto: false }]);
  assert.equal(s.location.path, '~/Library/Application Support/Shookshok/settings.json');

  const bad = await mac.setShortcut('active', 'Option+1');
  assert.equal(bad.ok, false);
  assert.ok(bad.suggestions.length > 0);
  assert.ok(bad.suggestions.every((k) => k.includes('Cmd+') || k.includes('Option+')));

  const preview = await mac.previewImport();
  assert.equal(preview.path, '~/Desktop/Shookshok-settings-2026-11-25.json');
  const activeChange = preview.changes.find((c) => c.path === 'shortcuts.active');
  assert.equal(activeChange?.after, 'Cmd+Option+F2');

  const seen = [];
  mac.subscribe((e) => seen.push(e));

  await mac.run('zoom');
  const zoomIsl = seen.filter((e) => e.type === 'island').at(-1).island;
  assert.ok(zoomIsl.hints.some((h) => h.key === '⌥1'));

  await mac.setHotkeysEnabled(false);
  assert.equal(seen.at(-1).island.detail, '⌥`');

  await mac.run('capture.region');
  await mac.captureInput({ type: 'command', name: 'copy' });
  const copyIsl = seen.at(-1).island;
  assert.equal(copyIsl.detail, '한글·PPT에 ⌘V');
  assert.equal(copyIsl.buttonKey, 'Cmd+S');

  await mac.run('folder.pick');
  assert.equal((await mac.getState()).settings['output.folder'], '~/Desktop/수업자료/캡처');

  await mac.resetAll();
  assert.equal((await mac.getState()).settings['output.folder'], '~/Pictures/Shookshok');

  const fonts = await mac.systemFonts();
  assert.ok(Array.isArray(fonts) && fonts.includes('Apple SD Gothic Neo'));
});

test('캡처 주석 되살리기(R2-UNDO-1)와 문제 보고용 기록 묶기(R2-DIAG-1)', async () => {
  const c = createFakeClient({ platform: 'macos', frameUrl: 'frame.png' });
  const seen = [];
  c.subscribe((e) => seen.push(e));

  await c.run('capture.region');
  const s1 = seen.find((e) => e.type === 'overlay').overlay.session;
  await c.captureInput({ type: 'down', x: 100, y: 100, button: 0 });
  await c.captureInput({ type: 'move', x: 400, y: 300 });
  await c.captureInput({ type: 'up', x: 400, y: 300, button: 0 });
  const doc = { monitorId: 0, anchor: { x: 100, y: 100 }, document: { pages: [{ id: 'page-1', items: [{ id: 's1' }] }] } };
  await c.captureAnnotations(s1, 0, 1, doc);

  await c.captureInput({ type: 'command', name: 'cancel' });
  assert.equal((await c.getState()).mode.mode, 'idle');
  const undoIsland = seen.filter((e) => e.type === 'island').at(-1).island;
  assert.equal(undoIsland.text, '주석을 지웠어요');
  assert.equal(undoIsland.button?.action, 'capture.restore');
  assert.equal(undoIsland.durationMs, 5000);

  await c.run('capture.restore');
  assert.equal((await c.getState()).mode.mode, 'capture');
  const restoredOv = seen.filter((e) => e.type === 'overlay').at(-1).overlay;
  assert.deepEqual(restoredOv.restoredAnnotations, [doc]);
  assert.equal(seen.filter((e) => e.type === 'capture').at(-1).view.phase, 'confirmed');

  // 기록 묶기: 한국어/영어 이름·단축키 포함 여부·저장 섬
  const rKo = await c.exportDiagnostics({ includeShortcuts: true });
  assert.equal(rKo.ok, true);
  assert.match(rKo.name, /^Shookshok-기록-\d{4}-\d{2}-\d{2}\.zip$/);
  const diagIsl = seen.filter((e) => e.type === 'island').at(-1).island;
  assert.equal(diagIsl.text, '다운로드 폴더에 저장했어요');
  assert.equal(diagIsl.button?.action, 'diag.openFolder');

  await c.updateSetting('general.lang', 'en');
  const rEn = await c.exportDiagnostics({ includeShortcuts: false });
  assert.equal(rEn.ok, true);
  assert.match(rEn.name, /^Shookshok-logs-\d{4}-\d{2}-\d{2}\.zip$/);
  assert.deepEqual(c.diagLog.map((d) => d.includeShortcuts), [true, false]);
});

test('최근 다섯 장에서 고른 그림을 열고 찍은 지 30분이 된 항목만 비운다', async (t) => {
  t.mock.timers.enable({ apis: ['Date', 'setTimeout'], now: 60_000 });
  const c = createFakeClient();
  const events = [];
  c.subscribe((event) => events.push(event));
  assert.deepEqual((await c.getState()).recentCaptures, []);
  for (let i = 0; i < 6; i++) {
    await c.run('capture.region');
    await c.captureInput({ type: 'move', x: 700 + i * 20, y: 400 });
    await c.captureInput({ type: 'command', name: 'copy' });
    t.mock.timers.tick(1000);
  }
  const entries = (await c.getState()).recentCaptures;
  assert.deepEqual(entries.map((e) => e.id), [6, 5, 4, 3, 2]);
  assert.equal(entries[4].capturedAt, 61);
  assert.deepEqual(Object.keys(entries[4]).sort(), ['capturedAt', 'id', 'thumbUrl']);
  await c.run('capture.reopen', 2);
  const selected = events.findLast((e) => e.type === 'draw').draw;
  const selectedThumb = decodeURIComponent(entries[4].thumbUrl).match(/viewBox="[^"]+"/)[0];
  assert.ok(decodeURIComponent(selected.monitor.frameUrl).includes(selectedThumb));
  assert.equal(selected.restoreOnOpen, false);
  t.mock.timers.tick(30 * 60_000 - 5000);
  assert.deepEqual(events.findLast((e) => e.type === 'state').state.recentCaptures.map((e) => e.id), [6, 5, 4, 3]);
  await c.run('capture.reopen', 2);
  assert.equal((await c.getState()).mode.session, selected.session);
  c.simulateLock();
  assert.deepEqual((await c.getState()).recentCaptures, []);
});
