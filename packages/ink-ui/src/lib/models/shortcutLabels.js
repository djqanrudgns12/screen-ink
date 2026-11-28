// 단축키 칸 → 화면에 보일 이름(지금 쓰는 쉬운 말). Rust 카탈로그의 원래 이름(예: 판서·실시간 확대)은 바꾸지 않고 화면에서만 바꾼다.
// 단축키 탭·설정 찾기가 같은 표를 쓴다(찾기 결과에 옛 이름이 보이던 것, 2026-11-28 Windows 화면 확인 #4·#5·#8).

export const SLOT_I18N_KEYS = {
  region: 'shortcuts.slot.region',
  window: 'shortcuts.slot.window',
  record: 'shortcuts.slot.record',
  draw: 'shortcuts.slot.draw',
  zoom: 'shortcuts.slot.zoom',
  board: 'shortcuts.slot.board',
  last: 'shortcuts.slot.last',
  full: 'shortcuts.slot.full',
  active: 'shortcuts.slot.active',
  delay: 'shortcuts.slot.delay',
  restore: 'shortcuts.slot.restore',
  pause: 'shortcuts.slot.pause',
  topmost: 'shortcuts.slot.topmost',
  pin: 'shortcuts.slot.pin',
  pinToggle: 'shortcuts.slot.pinToggle',
  timer: 'shortcuts.slot.timer',
  'c-detect': 'shortcuts.slot.cDetect',
  'c-copy': 'shortcuts.slot.cCopy',
  'c-save': 'shortcuts.slot.cSave',
  'c-quick': 'shortcuts.slot.cQuick',
  'c-cancel': 'shortcuts.slot.cCancel',
  'c-last': 'shortcuts.slot.cLast',
  'c-prev': 'shortcuts.slot.cPrev',
  'c-next': 'shortcuts.slot.cNext',
  'c-full': 'shortcuts.slot.cFull',
  'c-cursor': 'shortcuts.slot.cCursor',
  'c-color': 'shortcuts.slot.cColor',
  'd-pen': 'shortcuts.slot.dPen',
  'd-marker': 'shortcuts.slot.dMarker',
  'd-text': 'shortcuts.slot.dText',
  'd-select': 'shortcuts.slot.dSelect',
  'd-eraser': 'shortcuts.slot.dEraser',
  'd-clear': 'shortcuts.slot.dClear',
  'd-red': 'shortcuts.slot.dRed',
  'd-yellow': 'shortcuts.slot.dYellow',
  'd-green': 'shortcuts.slot.dGreen',
  'd-orange': 'shortcuts.slot.dOrange',
  'd-blue': 'shortcuts.slot.dBlue',
  'd-lime': 'shortcuts.slot.dLime',
  'd-white': 'shortcuts.slot.dWhite',
  'd-black': 'shortcuts.slot.dBlack',
  'd-undo': 'shortcuts.slot.dUndo',
  'd-redo': 'shortcuts.slot.dRedo',
  'd-copy': 'shortcuts.slot.dCopy',
  'd-save': 'shortcuts.slot.dSave',
  'd-restore': 'shortcuts.slot.dRestore',
  'd-center': 'shortcuts.slot.dCenter',
  'd-zin': 'shortcuts.slot.dZin',
  'd-zout': 'shortcuts.slot.dZout',
  'd-wup': 'shortcuts.slot.dWup',
  'd-wdown': 'shortcuts.slot.dWdown',
  'd-help': 'shortcuts.slot.dHelp',
  'd-exit': 'shortcuts.slot.dExit',
  'd-spot': 'shortcuts.slot.dSpot',
  'd-check': 'shortcuts.slot.dCheck',
  'd-number': 'shortcuts.slot.dNumber',
  'z-exit': 'shortcuts.slot.zExit',
};

/**
 * @param {{ id: string, label: string }} row
 * @param {(key: string) => string} t
 */
export function slotLabel(row, t) {
  const key = /** @type {Record<string, string>} */ (SLOT_I18N_KEYS)[row.id];
  return key ? t(key) : row.label;
}

/**
 * 찾기용: 단축키 목록의 이름을 화면 이름으로 바꾼 사본.
 * @template {{ id: string, label: string }} T
 * @param {T[]} rows
 * @param {(key: string) => string} t
 * @returns {T[]}
 */
export function withSlotLabels(rows, t) {
  return rows.map((r) => ({ ...r, label: slotLabel(r, t) }));
}
