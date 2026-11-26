import test from 'node:test';
import assert from 'node:assert/strict';
import { createFakeClient } from './fake.js';
import { decodeAnnotatedCommit, encodeAnnotatedCommit } from './index.js';

test('주석 층 본문 = [머리 길이 u32 LE][머리 JSON][RGBA]', () => {
  const meta = { session: 3, seq: 2, action: 'copy', w: 1, h: 1, mosaics: [{ x: 0, y: 0, w: 4, h: 4, block: 10 }] };
  const bytes = encodeAnnotatedCommit(meta, new Uint8ClampedArray([255, 0, 0, 255]));
  const n = new DataView(bytes.buffer).getUint32(0, true);
  assert.equal(n, JSON.stringify(meta).length, '머리 길이(ASCII JSON)');
  assert.deepEqual([...bytes.subarray(4 + n)], [255, 0, 0, 255], 'RGBA는 머리 뒤에 그대로');
  const back = decodeAnnotatedCommit(bytes);
  assert.deepEqual(back.meta, meta);
  assert.equal(back.rgba.length, 4);
  // 층 없이(모자이크만)
  const only = encodeAnnotatedCommit({ ...meta, w: 0, h: 0 }, null);
  assert.equal(decodeAnnotatedCommit(only).rgba.length, 0);
});

/** 가짜: 캡처를 열고 (주석 개수를 알린 뒤) 결과 단추를 누른다. */
async function scene(items) {
  const c = createFakeClient();
  const seen = [];
  c.subscribe((e) => seen.push(e));
  await c.run('capture.region');
  const session = (await c.getState()).mode.session;
  if (items) await c.captureAnnotations(session, 0, items);
  await c.captureInput({ type: 'up', x: 760, y: 420, button: 0 });
  await c.captureInput({ type: 'command', name: 'copy' });
  return { c, seen, session };
}

test('주석이 없으면 부탁 없이 바로 결과', async () => {
  const { c, seen } = await scene(0);
  assert.equal(seen.filter((e) => e.type === 'captureCommit').length, 0);
  assert.equal((await c.getState()).mode.mode, 'idle');
  assert.equal(seen.filter((e) => e.type === 'island').at(-1).island.text, '복사했어요');
});

test('주석이 있으면 부탁 → 답(순번 맞을 때만) → 결과', async () => {
  const { c, seen, session } = await scene(2);
  const req = seen.find((e) => e.type === 'captureCommit').request;
  assert.equal(req.session, session);
  assert.equal(req.action, 'copy');
  assert.equal((await c.getState()).mode.mode, 'capture', '답을 기다린다');
  await c.captureCommitAnnotated({ session, seq: req.seq + 5, action: 'copy', w: 0, h: 0, mosaics: [] }, null);
  assert.equal((await c.getState()).mode.mode, 'capture', '다른 순번은 버림');
  await c.captureCommitAnnotated({ session, seq: req.seq, action: 'copy', w: req.rect.w, h: req.rect.h, mosaics: [] }, new Uint8Array(req.rect.w * req.rect.h * 4));
  assert.equal((await c.getState()).mode.mode, 'idle');
  assert.equal(c.annotatedLog.length, 1);
  assert.equal(c.annotatedLog[0].bytes, req.rect.w * req.rect.h * 4);
  assert.ok(seen.some((e) => e.type === 'afterimage'), '결과 뒤 잔상');
});

test('부탁 뒤 주석이 모두 사라지면 기다리지 않는다', async () => {
  const { c, session } = await scene(1);
  assert.equal((await c.getState()).mode.mode, 'capture');
  await c.captureAnnotations(session, 0, 0);
  assert.equal((await c.getState()).mode.mode, 'idle');
});
