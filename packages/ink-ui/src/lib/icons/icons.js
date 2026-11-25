// F01 선 아이콘 24종 + 보조 아이콘 3종(lock, mic-off, close)
// 규칙(10 §4): 24px 격자, 둥근 끝·이음, 작은 점은 <circle r="0.85"> 채움(h.01 금지), 주황 채움 점은 모드 아이콘에만.

/**
 * @typedef {{ tag: 'path'|'rect'|'circle', attrs: Record<string, string|number>, role?: 'stroke'|'dot'|'accent'|'danger' }} IconShape
 * @typedef {{ label: string, mode?: boolean, shapes: IconShape[] }} IconDef
 */

/** @type {readonly string[]} F01의 24종 순서 그대로 */
export const ICON_NAMES = [
  'region',
  'window',
  'record',
  'draw',
  'zoom',
  'board',
  'pin',
  'restore',
  'folder',
  'shortcuts',
  'settings',
  'info',
  'copy',
  'save',
  'monitor',
  'mic',
  'magnet',
  'dark',
  'delay',
  'eraser',
  'undo',
  'check',
  'warning',
  'quit',
];

/** @type {Record<string, IconDef>} */
export const ICONS = {
  // 모드 아이콘 6종(주황 채움 점 · 녹화는 빨강)
  region: {
    label: '범위 캡처',
    mode: true,
    shapes: [
      { tag: 'path', attrs: { d: 'M4 8V6a2 2 0 0 1 2-2h2M16 4h2a2 2 0 0 1 2 2v2M20 16v2a2 2 0 0 1-2 2h-2M8 20H6a2 2 0 0 1-2-2v-2' } },
      { tag: 'path', role: 'accent', attrs: { d: 'M12 8.8c1.4 1.8 2.2 2.9 2.2 3.9a2.2 2.2 0 0 1-4.4 0c0-1 .8-2.1 2.2-3.9z' } },
    ],
  },
  window: {
    label: '창 캡처',
    mode: true,
    shapes: [
      { tag: 'rect', attrs: { x: 3.5, y: 5, width: 17, height: 14, rx: 2 } },
      { tag: 'path', attrs: { d: 'M3.5 9h17' } },
      { tag: 'circle', role: 'accent', attrs: { cx: 6.2, cy: 7, r: 0.95 } },
    ],
  },
  record: {
    label: '화면 녹화',
    mode: true,
    shapes: [
      { tag: 'circle', attrs: { cx: 12, cy: 12, r: 8 } },
      { tag: 'circle', role: 'danger', attrs: { cx: 12, cy: 12, r: 3.2 } },
    ],
  },
  draw: {
    label: '판서',
    mode: true,
    shapes: [
      { tag: 'path', attrs: { d: 'M15 4.5l4.5 4.5L9.5 19H5v-4.5z' } },
      { tag: 'path', attrs: { d: 'M13 6.5l4.5 4.5' } },
      { tag: 'circle', role: 'accent', attrs: { cx: 4.8, cy: 20.4, r: 1.5 } },
    ],
  },
  zoom: {
    label: '실시간 확대',
    mode: true,
    shapes: [
      { tag: 'circle', attrs: { cx: 10.5, cy: 10.5, r: 6.5 } },
      { tag: 'path', attrs: { d: 'M15.5 15.5L20 20' } },
      { tag: 'circle', role: 'accent', attrs: { cx: 10.5, cy: 10.5, r: 1.8 } },
    ],
  },
  board: {
    label: '칠판',
    mode: true,
    shapes: [
      { tag: 'rect', attrs: { x: 3.5, y: 4, width: 17, height: 11.5, rx: 1.5 } },
      { tag: 'path', attrs: { d: 'M8.5 20l2-4.5M15.5 20l-2-4.5' } },
      { tag: 'circle', role: 'accent', attrs: { cx: 16.4, cy: 12, r: 1.4 } },
    ],
  },

  // 일반 아이콘 18종(무채색 선)
  pin: {
    label: '창 항상 위',
    shapes: [
      { tag: 'path', attrs: { d: 'M9.5 3.5h5M10.5 3.5v4.6L7.5 12h9l-3-3.9V3.5' } },
      { tag: 'path', attrs: { d: 'M12 12v8' } },
    ],
  },
  restore: {
    label: '직전 판서',
    shapes: [
      { tag: 'path', attrs: { d: 'M4 12a8 8 0 1 0 2.3-5.6L4 8.5' } },
      { tag: 'path', attrs: { d: 'M4 4v4.5h4.5M12 8v4l2.5 1.5' } },
    ],
  },
  folder: {
    label: '저장 폴더',
    shapes: [
      { tag: 'path', attrs: { d: 'M3.5 7.5a2 2 0 0 1 2-2h4l2 2h7a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z' } },
    ],
  },
  shortcuts: {
    label: '단축키',
    shapes: [
      { tag: 'rect', attrs: { x: 2.5, y: 6, width: 19, height: 12, rx: 2 } },
      { tag: 'circle', role: 'dot', attrs: { cx: 6.5, cy: 10, r: 0.85 } },
      { tag: 'circle', role: 'dot', attrs: { cx: 10, cy: 10, r: 0.85 } },
      { tag: 'circle', role: 'dot', attrs: { cx: 14, cy: 10, r: 0.85 } },
      { tag: 'circle', role: 'dot', attrs: { cx: 17.5, cy: 10, r: 0.85 } },
      { tag: 'path', attrs: { d: 'M7.5 14h9' } },
    ],
  },
  settings: {
    label: '설정',
    shapes: [
      { tag: 'path', attrs: { d: 'M4 7h9M17 7h3M4 17h3M11 17h9' } },
      { tag: 'circle', attrs: { cx: 15, cy: 7, r: 2 } },
      { tag: 'circle', attrs: { cx: 9, cy: 17, r: 2 } },
    ],
  },
  info: {
    label: '정보',
    shapes: [
      { tag: 'circle', attrs: { cx: 12, cy: 12, r: 8.5 } },
      { tag: 'path', attrs: { d: 'M12 11v5' } },
      { tag: 'circle', role: 'dot', attrs: { cx: 12, cy: 8, r: 0.85 } },
    ],
  },
  copy: {
    label: '복사',
    shapes: [
      { tag: 'rect', attrs: { x: 8, y: 8, width: 12, height: 12, rx: 2 } },
      { tag: 'path', attrs: { d: 'M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2' } },
    ],
  },
  save: {
    label: '저장',
    shapes: [
      { tag: 'path', attrs: { d: 'M12 4v11M7.5 10.5L12 15l4.5-4.5M5 19.5h14' } },
    ],
  },
  monitor: {
    label: '모니터',
    shapes: [
      { tag: 'rect', attrs: { x: 3, y: 4, width: 18, height: 12, rx: 2 } },
      { tag: 'path', attrs: { d: 'M9 20h6M12 16v4' } },
    ],
  },
  mic: {
    label: '마이크',
    shapes: [
      { tag: 'path', attrs: { d: 'M12 3.5a3 3 0 0 1 3 3v5a3 3 0 0 1-6 0v-5a3 3 0 0 1 3-3z' } },
      { tag: 'path', attrs: { d: 'M6.5 11a5.5 5.5 0 0 0 11 0M12 16.5v4' } },
    ],
  },
  magnet: {
    label: '모서리 자석',
    shapes: [
      { tag: 'path', attrs: { d: 'M6 4h4v7a2 2 0 0 0 4 0V4h4v7a6 6 0 0 1-12 0z' } },
      { tag: 'path', attrs: { d: 'M6 8h4M14 8h4' } },
    ],
  },
  dark: {
    label: '다크 모드',
    shapes: [
      { tag: 'path', attrs: { d: 'M19.5 14.5A8 8 0 1 1 9.5 4.5a6.5 6.5 0 0 0 10 10z' } },
    ],
  },
  delay: {
    label: '지연 캡처',
    shapes: [
      { tag: 'circle', attrs: { cx: 12, cy: 13, r: 7.5 } },
      { tag: 'path', attrs: { d: 'M12 9v4l2.5 1.5M9.5 3h5' } },
    ],
  },
  eraser: {
    label: '지우개',
    shapes: [
      { tag: 'path', attrs: { d: 'M7 19.5l-3.5-3.5a1.5 1.5 0 0 1 0-2.1L13 4.4l7 7-8.1 8.1z' } },
      { tag: 'path', attrs: { d: 'M13 19.5h7M9 9l6.5 6.5' } },
    ],
  },
  undo: {
    label: '되돌리기',
    shapes: [
      { tag: 'path', attrs: { d: 'M9 6.5L4.5 11 9 15.5' } },
      { tag: 'path', attrs: { d: 'M5 11h9a5 5 0 0 1 0 10h-3' } },
    ],
  },
  check: {
    label: '확인',
    shapes: [
      { tag: 'path', attrs: { d: 'M5 12.5l4.5 4.5L19 7.5' } },
    ],
  },
  warning: {
    label: '주의',
    shapes: [
      { tag: 'path', attrs: { d: 'M12 4.5l8.5 15h-17z' } },
      { tag: 'path', attrs: { d: 'M12 10.5v4' } },
      { tag: 'circle', role: 'dot', attrs: { cx: 12, cy: 17, r: 0.85 } },
    ],
  },
  quit: {
    label: '끝내기',
    shapes: [
      { tag: 'path', attrs: { d: 'M12 3.5v7' } },
      { tag: 'path', attrs: { d: 'M17.5 6.5a8 8 0 1 1-11 0' } },
    ],
  },

  // 화면 보조 아이콘(섬 보안·마이크 꺼짐·창 닫기)
  lock: {
    label: '보안 화면',
    shapes: [
      { tag: 'rect', attrs: { x: 5, y: 10.5, width: 14, height: 9.5, rx: 2 } },
      { tag: 'path', attrs: { d: 'M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5' } },
    ],
  },
  'mic-off': {
    label: '마이크 꺼짐',
    shapes: [
      { tag: 'path', attrs: { d: 'M12 3.5a3 3 0 0 1 3 3v5a3 3 0 0 1-6 0v-5a3 3 0 0 1 3-3z' } },
      { tag: 'path', attrs: { d: 'M6.5 11a5.5 5.5 0 0 0 11 0M12 16.5v4M4 4l16 16' } },
    ],
  },
  close: {
    label: '닫기',
    shapes: [
      { tag: 'path', attrs: { d: 'M6 6l12 12M18 6l-12 12' } },
    ],
  },
};
