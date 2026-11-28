// 트레이 팝오버 순수 모델(R2-UI-TRAY · §6.5, D3)
// 상태 요약(녹화·권한·단축키 꺼짐·겹침·준비됨), 4칸 동작 타일, 키보드 이동(↑↓·Home·End·Enter·Esc·첫 글자/초성).

export const TRAY_POPOVER_WIDTH_PX = 252;
export const TRAY_ROW_HEIGHT_PX = 28;

const HANGUL_CHOSEONG = [
  'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ',
  'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

/**
 * 상단 2×2 동작 타일 4칸 명세(영역·창·전체·녹화).
 * 아이콘 이름은 icons.js 기준('region'·'window'·'monitor'·'record').
 */
export const TRAY_TILES = [
  { id: 'region', icon: 'region', labelKey: 'tray.tile.region', action: 'capture.region', slotId: 'region' },
  { id: 'window', icon: 'window', labelKey: 'tray.tile.window', action: 'capture.window', slotId: 'window' },
  { id: 'full', icon: 'monitor', labelKey: 'tray.tile.full', action: 'capture.full', slotId: 'full' },
  { id: 'record', icon: 'record', labelKey: 'tray.tile.record', action: 'record.toggle', slotId: 'record' },
];

/**
 * 녹화 경과 시간(ms)을 MM:SS 문자열로 바꾼다.
 * @param {number | undefined | null} ms
 * @returns {string}
 */
export function formatElapsed(ms) {
  const totalSec = Math.max(0, Math.floor((ms || 0) / 1000));
  const m = String(Math.floor(totalSec / 60)).padStart(2, '0');
  const s = String(totalSec % 60).padStart(2, '0');
  return `${m}:${s}`;
}

/**
 * 트레이 팝오버 상단 한 줄 상태를 우선순위에 따라 결정한다.
 * 우선순위: 녹화 중 > 권한 빠짐(macOS) > 단축키 꺼짐 > 키 겹침 > 준비됨.
 * @param {any} state
 * @returns {{
 *   kind: 'recording' | 'permission' | 'paused' | 'conflict' | 'ready',
 *   tone: 'danger' | 'attn' | 'off' | 'ok',
 *   clickable: boolean,
 *   action?: 'firstRun.open' | 'settings.open',
 *   elapsedMs?: number,
 *   count?: number,
 * }}
 */
export function resolveTrayStatus(state) {
  if (state?.recording?.active) {
    return {
      kind: 'recording',
      tone: 'danger',
      clickable: false,
      elapsedMs: state.recording.elapsedMs ?? 0,
    };
  }
  const perms = state?.permissions;
  if (state?.platform === 'macos' && perms && (!perms.screen || !perms.accessibility)) {
    return {
      kind: 'permission',
      tone: 'attn',
      clickable: true,
      action: 'firstRun.open',
    };
  }
  if (state?.hotkeysEnabled === false) {
    return {
      kind: 'paused',
      tone: 'off',
      clickable: false,
    };
  }
  const count = state?.conflicts?.count ?? 0;
  if (count > 0) {
    return {
      kind: 'conflict',
      tone: 'attn',
      clickable: true,
      action: 'settings.open',
      count,
    };
  }
  return {
    kind: 'ready',
    tone: 'ok',
    clickable: false,
  };
}

/**
 * 한글 완성형 음절의 초성을 반환한다. 완성형 음절이 아니면 null.
 * @param {string} ch
 * @returns {string | null}
 */
export function getHangulChoseong(ch) {
  if (!ch) return null;
  const code = ch.codePointAt(0) ?? 0;
  if (code < 0xac00 || code > 0xd7a3) return null;
  const idx = Math.floor((code - 0xac00) / (21 * 28));
  return HANGUL_CHOSEONG[idx] ?? null;
}

/**
 * 레이블에서 첫 글자(선행 공백 제거)를 소문자로 꺼낸다.
 * @param {string} text
 * @returns {string}
 */
export function getInitialChar(text) {
  const trimmed = String(text ?? '').trimStart();
  if (!trimmed) return '';
  return Array.from(trimmed)[0].toLowerCase();
}

/**
 * 첫 글자(영어 대소문자 무시 · 한글 음절 또는 초성 자모) 일치 항목을 현재 위치 다음부터 순환 탐색한다.
 * @param {{ label: string, disabled?: boolean }[]} items
 * @param {number} currentIndex
 * @param {string} key
 * @returns {number}
 */
export function findTypeaheadIndex(items, currentIndex, key) {
  if (!Array.isArray(items) || items.length === 0) return -1;
  const q = String(key ?? '').trim().toLowerCase();
  if (!q || Array.from(q).length !== 1) return -1;

  const isChoseongQuery = HANGUL_CHOSEONG.includes(q);
  const start = currentIndex >= 0 && currentIndex < items.length ? currentIndex + 1 : 0;

  for (let offset = 0; offset < items.length; offset += 1) {
    const idx = (start + offset) % items.length;
    const item = items[idx];
    if (!item || item.disabled) continue;
    const first = getInitialChar(item.label);
    if (!first) continue;
    if (first === q) return idx;
    if (isChoseongQuery && getHangulChoseong(first) === q) return idx;
  }
  return -1;
}

/**
 * 활성 항목 인덱스를 주어진 방향으로 순환 이동한다.
 * @param {{ label: string, disabled?: boolean }[]} items
 * @param {number} currentIndex
 * @param {1 | -1} delta
 * @returns {number}
 */
function stepEnabledIndex(items, currentIndex, delta) {
  if (!Array.isArray(items) || items.length === 0) return -1;
  const start = currentIndex < 0
    ? (delta > 0 ? 0 : items.length - 1)
    : (currentIndex + delta + items.length) % items.length;

  for (let offset = 0; offset < items.length; offset += 1) {
    const idx = (start + offset * delta + items.length * 2) % items.length;
    if (items[idx] && !items[idx].disabled) return idx;
  }
  return -1;
}

/** 두벌식 자판: 물리 키(`KeyboardEvent.code`) → 초성 자모(Shift면 된소리). */
const DUBEOLSIK_CHOSEONG = {
  KeyR: ['ㄱ', 'ㄲ'], KeyS: ['ㄴ', 'ㄴ'], KeyE: ['ㄷ', 'ㄸ'], KeyF: ['ㄹ', 'ㄹ'], KeyA: ['ㅁ', 'ㅁ'],
  KeyQ: ['ㅂ', 'ㅃ'], KeyT: ['ㅅ', 'ㅆ'], KeyD: ['ㅇ', 'ㅇ'], KeyW: ['ㅈ', 'ㅉ'], KeyC: ['ㅊ', 'ㅊ'],
  KeyZ: ['ㅋ', 'ㅋ'], KeyX: ['ㅌ', 'ㅌ'], KeyV: ['ㅍ', 'ㅍ'], KeyG: ['ㅎ', 'ㅎ'],
};

/**
 * 한글 입력기가 켜진 채 메뉴에서 자음 키를 누르면 Windows는 글자 대신 `Process`(조합 중)를 준다. 메뉴는 글 쓰는 칸이
 * 아니라 조합이 끝나지 않으므로, 물리 키를 두벌식 초성으로 바꿔 첫 글자 이동에 쓴다(2026-11-28 Windows 재확인 #1).
 * @param {{ key?: string, code?: string, isComposing?: boolean, shiftKey?: boolean }} event
 * @returns {string | null}
 */
export function imeChoseongOf(event) {
  const composing = event?.isComposing || event?.key === 'Process' || event?.key === 'Unidentified';
  if (!composing) return null;
  const pair = /** @type {Record<string, string[]>} */ (DUBEOLSIK_CHOSEONG)[event?.code ?? ''];
  return pair ? pair[event?.shiftKey ? 1 : 0] : null;
}

/**
 * 메뉴 키보드 이벤트에 대한 동작(↑↓·Home·End·Enter·Esc·첫 글자 이동)을 계산한다.
 * @param {{ key?: string, code?: string, isComposing?: boolean, shiftKey?: boolean, ctrlKey?: boolean, metaKey?: boolean, altKey?: boolean }} event
 * @param {{ label: string, disabled?: boolean }[]} items
 * @param {number} currentIndex
 * @returns {{ type: 'close' } | { type: 'focus', index: number } | { type: 'activate', index: number } | { type: 'noop' }}
 */
export function resolveMenuKeyAction(event, items, currentIndex) {
  if (!event || event.ctrlKey || event.metaKey || event.altKey) {
    return { type: 'noop' };
  }
  const jamo = imeChoseongOf(event);
  if (jamo) {
    if (!Array.isArray(items) || items.length === 0) return { type: 'noop' };
    const idx = findTypeaheadIndex(items, currentIndex, jamo);
    return idx >= 0 ? { type: 'focus', index: idx } : { type: 'noop' };
  }
  if (event.isComposing) {
    return { type: 'noop' };
  }
  const key = event.key ?? '';
  if (key === 'Escape') {
    return { type: 'close' };
  }
  if (!Array.isArray(items) || items.length === 0) {
    return { type: 'noop' };
  }
  if (key === 'ArrowDown') {
    const idx = stepEnabledIndex(items, currentIndex, 1);
    return idx >= 0 ? { type: 'focus', index: idx } : { type: 'noop' };
  }
  if (key === 'ArrowUp') {
    const idx = stepEnabledIndex(items, currentIndex, -1);
    return idx >= 0 ? { type: 'focus', index: idx } : { type: 'noop' };
  }
  if (key === 'Home') {
    const idx = items.findIndex((it) => it && !it.disabled);
    return idx >= 0 ? { type: 'focus', index: idx } : { type: 'noop' };
  }
  if (key === 'End') {
    for (let i = items.length - 1; i >= 0; i -= 1) {
      if (items[i] && !items[i].disabled) return { type: 'focus', index: i };
    }
    return { type: 'noop' };
  }
  if (key === 'Enter' || key === ' ') {
    if (currentIndex >= 0 && currentIndex < items.length && items[currentIndex] && !items[currentIndex].disabled) {
      return { type: 'activate', index: currentIndex };
    }
    return { type: 'noop' };
  }
  if (Array.from(key).length === 1 && !/\s/.test(key)) {
    const idx = findTypeaheadIndex(items, currentIndex, key);
    return idx >= 0 ? { type: 'focus', index: idx } : { type: 'noop' };
  }
  return { type: 'noop' };
}
