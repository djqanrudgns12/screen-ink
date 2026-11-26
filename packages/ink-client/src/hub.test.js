import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEventHub } from './hub.js';

/** 가짜 listen: 이름 → 콜백, `fire`로 사건을 낸다. 실제처럼 등록은 한 번씩만. */
function fakeListen() {
  /** @type {Map<string, (e: any) => void>} */
  const cbs = new Map();
  let calls = 0;
  const listen = (name, cb) => {
    calls += 1;
    cbs.set(name, cb);
    return Promise.resolve(() => cbs.delete(name));
  };
  const fire = (name, payload) => cbs.get(name)?.({ payload });
  return { listen, fire, calls: () => calls };
}

const tick = () => new Promise((r) => queueMicrotask(() => r(undefined)));

test('듣기는 창마다 한 번 — 구독자가 늘어도 다시 걸지 않는다', () => {
  const f = fakeListen();
  const hub = createEventHub(f.listen);
  const n = f.calls();
  hub.add(() => {});
  hub.add(() => {});
  assert.equal(f.calls(), n);
});

test('판서→캡처 전환: 세션 사건 뒤에 붙은 캡처 화면도 그 세션을 받는다', async () => {
  const f = fakeListen();
  const hub = createEventHub(f.listen);
  /** @type {any[]} */
  const app = [];
  hub.add((e) => app.push(e.type));
  // Rust 순서: 판서 끝(쉼) → 캡처 세션 → 캡처 상태
  f.fire('screen-ink://state', { mode: { mode: 'idle' } });
  f.fire('screen-ink://overlay', { session: 7 });
  /** @type {any[]} */
  const late = [];
  hub.add((e) => late.push(e));
  f.fire('screen-ink://state', { mode: { mode: 'capture', session: 7 } });
  await tick();
  assert.ok(late.some((e) => e.type === 'overlay' && e.overlay.session === 7), '늦게 붙어도 세션 7');
  assert.deepEqual(app, ['state', 'overlay', 'state'], '먼저 붙은 구독자는 한 번씩만');
});

test('캡처가 끝나면 지난 세션을 다시 주지 않는다', async () => {
  const f = fakeListen();
  const hub = createEventHub(f.listen);
  f.fire('screen-ink://overlay', { session: 3 });
  f.fire('screen-ink://state', { mode: { mode: 'idle' } });
  /** @type {any[]} */
  const got = [];
  hub.add((e) => got.push(e));
  await tick();
  assert.equal(got.length, 0);
});

test('구독을 끊으면 받지 않는다', () => {
  const f = fakeListen();
  const hub = createEventHub(f.listen);
  let n = 0;
  const off = hub.add(() => (n += 1));
  f.fire('screen-ink://island', { kind: 'result' });
  off();
  f.fire('screen-ink://island', { kind: 'result' });
  assert.equal(n, 1);
});
