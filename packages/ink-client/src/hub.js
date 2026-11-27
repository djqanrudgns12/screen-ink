// 앱 사건 나눔(창마다 하나) — TauriInkClient.subscribe가 쓴다. Tauri에 기대지 않게 `listen`을 받는다(시험용).
//
// 구독할 때마다 `listen`을 새로 걸면 등록(IPC 왕복)이 끝나기 전에 온 사건을 놓친다. 판서 → 캡처 전환에서
// 그 모니터 창의 캡처 화면은 새로 붙는데, 캡처 세션 사건(`overlay`)이 그 사이에 와서 빈 화면이 되었다
// (2026-11-26 맥 실측: 판서 중 F1 → 커서 모니터 캡처 화면이 입력을 못 받음). 그래서
// ① 사건 듣기는 창마다 한 번만 걸고 구독자에게 나눠 주고,
// ② 마지막 캡처 세션은 기억했다가 늦게 붙은 구독자에게 한 번 다시 준다(캡처가 아닌 상태가 오면 지운다).

/** @typedef {(name: string, cb: (e: { payload: any }) => void) => Promise<() => void>} ListenFn */

/** 앱 사건 이름 → 구독자에게 줄 모양. */
export const EVENT_MAP = Object.freeze([
  ['screen-ink://state', (p) => ({ type: 'state', state: p })],
  ['screen-ink://island', (p) => ({ type: 'island', island: p })],
  ['screen-ink://overlay', (p) => ({ type: 'overlay', overlay: p })],
  ['screen-ink://capture', (p) => ({ type: 'capture', view: p })],
  ['screen-ink://capture-command', (p) => ({ type: 'captureCommand', ...p })],
  ['screen-ink://draw', (p) => ({ type: 'draw', draw: p })],
  ['screen-ink://draw-command', (p) => ({ type: 'drawCommand', ...p })],
  ['screen-ink://afterimage', (p) => ({ type: 'afterimage', afterimage: p })],
  ['screen-ink://zoom', (p) => ({ type: 'zoom', zoom: p ?? null })],
  ['screen-ink://capture-commit', (p) => ({ type: 'captureCommit', request: p })],
]);

/**
 * @param {ListenFn} listen
 * @returns {{ islandReady: Promise<boolean>, add: (handler: (e: any) => void) => () => void }}
 */
export function createEventHub(listen) {
  /** @type {Set<(e: any) => void>} */
  const subs = new Set();
  /** @type {any} */
  let lastOverlay = null;
  /** @param {any} ev */
  const dispatch = (ev) => {
    if (ev.type === 'overlay') lastOverlay = ev;
    else if (ev.type === 'state' && ev.state?.mode && ev.state.mode.mode !== 'capture') lastOverlay = null;
    for (const h of [...subs]) h(ev);
  };
  let islandReady = Promise.resolve(false);
  for (const [name, to] of EVENT_MAP) {
    const ready = listen(name, (e) => dispatch(to(e.payload))).then(() => true, () => false);
    if (name === 'screen-ink://island') islandReady = ready;
  }
  return {
    islandReady,
    add(handler) {
      subs.add(handler);
      // 지금 캡처 중이면 마지막 세션을 한 번 다시 준다(붙기 전에 지나간 사건)
      const replay = lastOverlay;
      if (replay) {
        queueMicrotask(() => {
          if (subs.has(handler) && lastOverlay === replay) handler(replay);
        });
      }
      return () => {
        subs.delete(handler);
      };
    },
  };
}
