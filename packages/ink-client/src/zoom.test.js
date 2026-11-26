import test from 'node:test';
import assert from 'node:assert/strict';
import { createFakeClient } from './fake.js';

/** 이벤트를 모은다 */
function record(c) {
  const events = [];
  c.subscribe((e) => events.push(e));
  return events;
}

test('실시간 확대: 시작하면 liveZoom 모드·확대 섬·HUD 상태, 끝내면 null', async () => {
  const c = createFakeClient();
  const ev = record(c);
  await c.run('zoom');
  const s = await c.getState();
  assert.equal(s.mode.mode, 'liveZoom');
  const island = ev.filter((e) => e.type === 'island').at(-1).island;
  assert.deepEqual([island.kind, island.variant, island.text, island.value], ['mode', 'zoom', '실시간 확대', '×2.0']);
  assert.deepEqual(island.hints.map((h) => h.key), ['휠', 'Alt 1', 'Esc']);
  const zoom = ev.filter((e) => e.type === 'zoom').at(-1).zoom;
  assert.equal(zoom.monitorLabel, '모니터 1');
  assert.deepEqual(zoom.viewport, { x: 0.25, y: 0.25, w: 0.5, h: 0.5 });
  assert.ok(zoom.cursorCss);

  await c.zoomInput({ type: 'key', slot: 'z-exit' });
  assert.equal((await c.getState()).mode.mode, 'idle');
  assert.equal(ev.filter((e) => e.type === 'zoom').at(-1).zoom, null);
});

test('실시간 확대: 휠·확대 키는 단계를 오르내리고 끝 단계에서 멈춘다', async () => {
  const c = createFakeClient();
  const ev = record(c);
  const scale = () => ev.filter((e) => e.type === 'zoom').at(-1).zoom.scale;
  await c.run('zoom');
  await c.zoomInput({ type: 'wheel', dy: -1 });
  assert.equal(scale(), 2.5);
  await c.zoomInput({ type: 'key', slot: 'z-out' });
  await c.zoomInput({ type: 'key', slot: 'z-out' });
  assert.equal(scale(), 1.5);
  for (let i = 0; i < 5; i++) await c.zoomInput({ type: 'wheel', dy: 1 });
  assert.equal(scale(), 1.25);
  assert.equal(ev.filter((e) => e.type === 'island').at(-1).island.value, '×1.25');
  for (let i = 0; i < 9; i++) await c.zoomInput({ type: 'key', slot: 'z-in' });
  assert.equal(scale(), 8);
  await c.zoomInput({ type: 'exit' });
  // 끝난 뒤 입력은 무시
  await c.zoomInput({ type: 'wheel', dy: -1 });
  assert.equal((await c.getState()).mode.mode, 'idle');
});

test('실시간 확대 중 캡처를 시작하면 확대를 끝내고, 위치 지도 설정을 따른다', async () => {
  const c = createFakeClient();
  const ev = record(c);
  await c.updateSetting('zoom.minimap', false);
  await c.run('zoom');
  assert.equal(ev.filter((e) => e.type === 'zoom').at(-1).zoom.showMinimap, false);
  await c.run('capture.region');
  assert.equal((await c.getState()).mode.mode, 'capture');
  assert.equal(ev.filter((e) => e.type === 'zoom').at(-1).zoom, null);
  // 캡처 중 확대 = 캡처를 끝내고 확대
  await c.run('zoom');
  assert.equal((await c.getState()).mode.mode, 'liveZoom');
});
