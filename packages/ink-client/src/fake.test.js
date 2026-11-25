import test from 'node:test';
import assert from 'node:assert/strict';
import { createFakeClient } from './fake.js';
import { keycaps } from './index.js';

test('확정본 59칸이 행으로 나오고 Snipaste 겹침 4개를 요약한다', async () => {
  const c = createFakeClient();
  const s = await c.getState();
  assert.equal(s.shortcuts.length, 59);
  assert.equal(s.conflicts.count, 4);
  assert.equal(s.conflicts.app, 'Snipaste');
});

test('맥에서는 창 항상 위 칸이 빠진다', async () => {
  const s = await createFakeClient({ platform: 'macos' }).getState();
  assert.equal(s.shortcuts.length, 58);
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

test('키캡은 Windows에서 Shift, 맥에서 기호', () => {
  assert.deepEqual(keycaps('Alt+Shift+D'), ['Alt', 'Shift', 'D']);
  assert.deepEqual(keycaps('Option+Shift+1', 'macos'), ['⌥', '⇧', '1']);
});
