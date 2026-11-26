import test from 'node:test';
import assert from 'node:assert/strict';
import { HALO_SIZE, applyZoomEvent, haloBox, isLiveZoomMode, showMinimap } from './zoomHud.js';

const zoom = {
  session: 3,
  monitorId: 0,
  monitorLabel: '모니터 1',
  scale: 2,
  scaleLabel: '×2.0',
  viewport: { x: 0, y: 0.03, w: 0.5, h: 0.5 },
  cursorCss: { x: 801, y: 541 },
  showMinimap: true,
};

test('후광은 44px, 중심이 커서 끝이고 커서가 확대 화면 밖이면 없다', () => {
  assert.equal(HALO_SIZE, 44);
  assert.deepEqual(haloBox({ x: 801, y: 541 }), { x: 779, y: 519, size: 44 });
  assert.equal(haloBox(undefined), null);
  assert.equal(haloBox({ x: Number.NaN, y: 1 }), null);
});

test('zoom 이벤트가 값을 정하고 null이면 거두며, 모드가 확대가 아니면 상태 이벤트로도 거둔다', () => {
  let s = applyZoomEvent(null, { type: 'zoom', zoom });
  assert.equal(s, zoom);
  s = applyZoomEvent(s, { type: 'state', state: { mode: { mode: 'liveZoom' } } });
  assert.equal(s, zoom, '확대 중 상태 이벤트는 그대로');
  s = applyZoomEvent(s, { type: 'island', island: {} });
  assert.equal(s, zoom);
  assert.equal(applyZoomEvent(s, { type: 'zoom', zoom: null }), null);
  assert.equal(applyZoomEvent(s, { type: 'state', state: { mode: { mode: 'idle' } } }), null);
  assert.equal(applyZoomEvent(s, undefined), zoom);
});

test('모드 판정과 위치 지도 설정', () => {
  assert.equal(isLiveZoomMode({ mode: 'liveZoom' }), true);
  assert.equal(isLiveZoomMode({ mode: 'capture' }), false);
  assert.equal(isLiveZoomMode(undefined), false);
  assert.equal(showMinimap(zoom), true);
  assert.equal(showMinimap({ ...zoom, showMinimap: false }), false);
  assert.equal(showMinimap(null), false);
});
