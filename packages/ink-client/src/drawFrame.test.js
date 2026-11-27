import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readDrawFrame } from './drawFrame.js';

test('세션이 끝난 뒤 비트맵이 도착해도 공유 버퍼와 비트맵을 한 번씩 놓는다', async (t) => {
  const webview = new EventTarget();
  const released = [];
  webview.releaseBuffer = (buffer) => released.push(buffer);
  const decoding = Promise.withResolvers();
  const close = t.mock.fn();
  // 이 파일의 독립된 노드 프로세스에 웹뷰 이벤트와 비트맵 해석 완료 시점을 재현한다.
  globalThis.chrome = { webview };
  globalThis.ImageData = class {};
  globalThis.createImageBitmap = () => decoding.promise;
  t.after(() => {
    delete globalThis.chrome;
    delete globalThis.ImageData;
    delete globalThis.createImageBitmap;
  });

  const controller = new AbortController();
  const pending = readDrawFrame(2, controller.signal, async () => {});
  const buffer = new ArrayBuffer(4);
  const getBuffer = t.mock.fn(() => buffer);
  for (const session of [1, 2]) {
    const event = Object.assign(new Event('sharedbufferreceived'), { additionalData: { session, width: 1, height: 1 }, getBuffer });
    webview.dispatchEvent(event);
  }
  controller.abort();
  assert.equal(await pending, null);
  assert.equal(getBuffer.mock.callCount(), 1);
  decoding.resolve({ close });
  await decoding.promise;
  assert.deepEqual(released, [buffer]);
  assert.equal(close.mock.callCount(), 1);
});
