// 키보드 이벤트를 정규 단축키 조합 문자열로 바꾼다.
// 한글 IME 상태에서도 physical `code`(KeyA, Digit1 등)를 기준으로 영문/숫자를 뽑는다.

const MOD_CODES = new Set([
  'ControlLeft',
  'ControlRight',
  'AltLeft',
  'AltRight',
  'ShiftLeft',
  'ShiftRight',
  'MetaLeft',
  'MetaRight',
]);

const MOD_KEYS = new Set(['Control', 'Alt', 'Shift', 'Meta', 'OS', 'AltGraph']);

const CODE_MAP = {
  Backquote: '`',
  Escape: 'Esc',
  Enter: 'Enter',
  NumpadEnter: 'Enter',
  Tab: 'Tab',
  Space: 'Space',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Insert: 'Insert',
  Home: 'Home',
  End: 'End',
  PageUp: 'PgUp',
  PageDown: 'PgDn',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  PrintScreen: 'PrtScn',
  Pause: 'Pause',
  ScrollLock: 'ScrLk',
  Comma: ',',
  Period: '.',
  Slash: '/',
  Backslash: '\\',
  Minus: '-',
  Equal: '=',
  BracketLeft: '[',
  BracketRight: ']',
  Semicolon: ';',
  Quote: "'",
  NumpadAdd: 'Plus',
  NumpadSubtract: 'Minus',
  NumpadMultiply: '*',
  NumpadDivide: '/',
  NumpadDecimal: '.',
};

const KEY_FALLBACK_MAP = {
  Escape: 'Esc',
  ' ': 'Space',
  Spacebar: 'Space',
  Enter: 'Enter',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Insert: 'Insert',
  Home: 'Home',
  End: 'End',
  PageUp: 'PgUp',
  PageDown: 'PgDn',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  PrintScreen: 'PrtScn',
  Pause: 'Pause',
  ScrollLock: 'ScrLk',
  '`': '`',
};

const FN_KEY_RE = /^F([1-9]|1\d|2[0-4])$/;
const CODE_LETTER_RE = /^Key([A-Z])$/;
const CODE_DIGIT_RE = /^Digit([0-9])$/;
const CODE_NUMPAD_DIGIT_RE = /^Numpad([0-9])$/;

/**
 * 물리 키 `code` 우선, 없을 때만 `key`에서 주 키 이름을 뽑는다.
 * @param {{ key?: string, code?: string }} event
 * @returns {string | null}
 */
function resolvePrimaryKey(event) {
  const code = event.code || '';
  const key = event.key || '';

  if (MOD_CODES.has(code) || (!code && MOD_KEYS.has(key))) {
    return null;
  }

  if (code) {
    if (CODE_MAP[code]) return CODE_MAP[code];
    const letter = CODE_LETTER_RE.exec(code);
    if (letter) return letter[1];
    const digit = CODE_DIGIT_RE.exec(code);
    if (digit) return digit[1];
    const numpad = CODE_NUMPAD_DIGIT_RE.exec(code);
    if (numpad) return numpad[1];
    if (FN_KEY_RE.test(code)) return code;
  }

  if (!key || MOD_KEYS.has(key) || key === 'Process' || key === 'Dead' || key === 'Unidentified') {
    return null;
  }

  if (KEY_FALLBACK_MAP[key]) return KEY_FALLBACK_MAP[key];
  if (FN_KEY_RE.test(key)) return key;
  if (/^[a-zA-Z0-9]$/.test(key)) return key.toUpperCase();
  if (/^[,./\\\-=[\];']$/.test(key)) return key;

  return null;
}

/**
 * KeyboardEvent 모양 객체 → 정규 단축키 문자열. 수식키만 누르면 null.
 * 수식키 순서: win [Ctrl, Alt, Shift, Win] / mac [Ctrl, Option, Shift, Cmd]
 *
 * @param {{ key?: string, code?: string, ctrlKey?: boolean, altKey?: boolean, shiftKey?: boolean, metaKey?: boolean }} event
 * @param {'windows'|'macos'} [platform='windows']
 * @returns {string | null}
 */
export function eventToCombo(event, platform = 'windows') {
  if (!event) return null;
  const primary = resolvePrimaryKey(event);
  if (!primary) return null;

  const mods = [];
  if (event.ctrlKey) mods.push('Ctrl');
  if (event.altKey) mods.push(platform === 'macos' ? 'Option' : 'Alt');
  if (event.shiftKey) mods.push('Shift');
  if (event.metaKey) mods.push(platform === 'macos' ? 'Cmd' : 'Win');

  return mods.length ? `${mods.join('+')}+${primary}` : primary;
}

/**
 * 수식키 없이 Esc만 눌렀는지 판별.
 * @param {{ key?: string, code?: string, ctrlKey?: boolean, altKey?: boolean, shiftKey?: boolean, metaKey?: boolean }} event
 */
export function isEscapeOnly(event) {
  return (
    Boolean(event) &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey &&
    !event.metaKey &&
    (event.code === 'Escape' || event.key === 'Escape')
  );
}

/**
 * 수식키 없이 Backspace만 눌렀는지 판별(단축키 비우기).
 * @param {{ key?: string, code?: string, ctrlKey?: boolean, altKey?: boolean, shiftKey?: boolean, metaKey?: boolean }} event
 */
export function isClearOnly(event) {
  return (
    Boolean(event) &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.shiftKey &&
    !event.metaKey &&
    (event.code === 'Backspace' || event.key === 'Backspace')
  );
}
