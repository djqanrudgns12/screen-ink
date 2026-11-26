// 슉쇽 맞춤 아이콘 세트(R2-ICON, D1) — 24×24 격자, 둥근 끝·이음, 획 20px 1.60 / 16px 1.75 광학 보정.
// 원본 35종(10-custom-shookshok) + 누락 7종(eyedropper·font·slider·check-circle·warning-triangle·external-link·help-circle) + 채움 변형 8종(*-fill).

/**
 * @typedef {{
 *   tag: 'path' | 'rect' | 'circle' | 'ellipse',
 *   attrs: Record<string, string | number>,
 *   role?: 'stroke' | 'dot' | 'fill' | 'accent' | 'danger',
 * }} IconShape
 * @typedef {{ label: string, labelKey: string, mode?: boolean, shapes: IconShape[] }} IconDef
 */

/** @type {readonly string[]} 기존 F01 24종 순서 유지 + 신규 아이콘 추가 */
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
  // 보조 · 주석 · UI 확장 아이콘
  'lock',
  'mic-off',
  'close',
  'rect',
  'ellipse',
  'arrow',
  'pen',
  'marker',
  'mosaic',
  'text',
  'redo',
  'eyedropper',
  'font',
  'slider',
  'check-circle',
  'warning-triangle',
  'external-link',
  'help-circle',
  'window-top',
  'line',
  'trash',
];

/** "지금 고른 도구"용 채움 변형 9종 */
export const FILL_ICON_NAMES = [
  'rect-fill',
  'ellipse-fill',
  'arrow-fill',
  'line-fill',
  'pen-fill',
  'marker-fill',
  'mosaic-fill',
  'text-fill',
  'eraser-fill',
];

/** @type {Record<string, IconDef>} */
export const ICONS = {
  // ─────────────── 모드 · 동작 · 트레이 24종 ───────────────
  region: {
    label: '범위 캡처',
    labelKey: 'icon.region',
    mode: true,
    shapes: [
      { tag: 'path', attrs: { d: 'M4 8.5V6a2 2 0 0 1 2-2h2.5M15.5 4H18a2 2 0 0 1 2 2v2.5M20 15.5V18a2 2 0 0 1-2 2h-2.5M8.5 20H6a2 2 0 0 1-2-2v-2.5' } },
      { tag: 'path', attrs: { d: 'M12 9.25v5.5M9.25 12h5.5' } },
    ],
  },
  window: {
    label: '창 캡처',
    labelKey: 'icon.window',
    mode: true,
    shapes: [
      { tag: 'rect', attrs: { x: 3.5, y: 5, width: 17, height: 14, rx: 2.2 } },
      { tag: 'path', attrs: { d: 'M3.5 9.25h17' } },
      { tag: 'circle', role: 'dot', attrs: { cx: 6.3, cy: 7.1, r: 0.85 } },
      { tag: 'circle', role: 'dot', attrs: { cx: 9.1, cy: 7.1, r: 0.85 } },
    ],
  },
  record: {
    label: '화면 녹화',
    labelKey: 'icon.record',
    mode: true,
    shapes: [
      { tag: 'circle', attrs: { cx: 12, cy: 12, r: 8 } },
      { tag: 'circle', role: 'danger', attrs: { cx: 12, cy: 12, r: 3.4 } },
    ],
  },
  draw: {
    label: '그리기',
    labelKey: 'icon.draw',
    mode: true,
    shapes: [
      { tag: 'path', attrs: { d: 'M15 4.2l4.8 4.8L10 18.8H5.2V14z' } },
      { tag: 'path', attrs: { d: 'M13 6.2l4.8 4.8' } },
      { tag: 'path', attrs: { d: 'M4.5 21c2.4 0 3.6-1.4 6-1.4s3.4 1.4 5.5 1.4c1.5 0 2.5-.5 3.5-1.2' } },
    ],
  },
  zoom: {
    label: '확대',
    labelKey: 'icon.zoom',
    mode: true,
    shapes: [
      { tag: 'circle', attrs: { cx: 10.5, cy: 10.5, r: 6.5 } },
      { tag: 'path', attrs: { d: 'M15.5 15.5L20 20' } },
      { tag: 'path', attrs: { d: 'M10.5 8v5M8 10.5h5' } },
    ],
  },
  board: {
    label: '칠판',
    labelKey: 'icon.board',
    mode: true,
    shapes: [
      { tag: 'rect', attrs: { x: 3.5, y: 4.5, width: 17, height: 11, rx: 1.8 } },
      { tag: 'path', attrs: { d: 'M8 20l2-4.5M16 20l-2-4.5M12 15.5V20' } },
    ],
  },
  pin: {
    label: '화면에 고정',
    labelKey: 'icon.pin',
    shapes: [
      { tag: 'path', attrs: { d: 'M14.5 4l5.5 5.5-2 2-1.5-.5-3.5 3.5.5 2-1.5 1.5-4.5-4.5 1.5-1.5 2 .5 3.5-3.5-.5-1.5z' } },
      { tag: 'path', attrs: { d: 'M8.5 15.5L4 20' } },
    ],
  },
  restore: {
    label: '방금 그리기',
    labelKey: 'icon.restore',
    shapes: [
      { tag: 'path', attrs: { d: 'M4 12a8 8 0 1 0 2.4-5.7L4 8.5' } },
      { tag: 'path', attrs: { d: 'M4 4.2v4.3h4.3' } },
      { tag: 'path', attrs: { d: 'M12 8v4l2.8 1.6' } },
    ],
  },
  folder: {
    label: '저장 폴더',
    labelKey: 'icon.folder',
    shapes: [
      { tag: 'path', attrs: { d: 'M3.5 7.5a2 2 0 0 1 2-2h4l2 2h7a2 2 0 0 1 2 2v7.5a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z' } },
    ],
  },
  shortcuts: {
    label: '단축키',
    labelKey: 'icon.shortcuts',
    shapes: [
      { tag: 'rect', attrs: { x: 2.75, y: 6, width: 18.5, height: 12, rx: 2.2 } },
      { tag: 'circle', role: 'dot', attrs: { cx: 6.5, cy: 10, r: 0.85 } },
      { tag: 'circle', role: 'dot', attrs: { cx: 10.1, cy: 10, r: 0.85 } },
      { tag: 'circle', role: 'dot', attrs: { cx: 13.9, cy: 10, r: 0.85 } },
      { tag: 'circle', role: 'dot', attrs: { cx: 17.5, cy: 10, r: 0.85 } },
      { tag: 'path', attrs: { d: 'M7.5 14.2h9' } },
    ],
  },
  settings: {
    label: '설정',
    labelKey: 'icon.settings',
    shapes: [
      { tag: 'path', attrs: { d: 'M4 7.5h8.5M16.5 7.5H20M4 16.5h3.5M11.5 16.5H20' } },
      { tag: 'circle', attrs: { cx: 14.5, cy: 7.5, r: 2 } },
      { tag: 'circle', attrs: { cx: 9.5, cy: 16.5, r: 2 } },
    ],
  },
  info: {
    label: '정보',
    labelKey: 'icon.info',
    shapes: [
      { tag: 'circle', attrs: { cx: 12, cy: 12, r: 8.25 } },
      { tag: 'path', attrs: { d: 'M12 11.2v4.8' } },
      { tag: 'circle', role: 'dot', attrs: { cx: 12, cy: 8.2, r: 0.9 } },
    ],
  },
  copy: {
    label: '복사',
    labelKey: 'icon.copy',
    shapes: [
      { tag: 'rect', attrs: { x: 8.5, y: 8.5, width: 11.5, height: 11.5, rx: 2 } },
      { tag: 'path', attrs: { d: 'M15.5 8.5V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7.5a2 2 0 0 0 2 2h2.5' } },
    ],
  },
  save: {
    label: '저장',
    labelKey: 'icon.save',
    shapes: [
      { tag: 'path', attrs: { d: 'M12 4v10.5' } },
      { tag: 'path', attrs: { d: 'M8 10.5l4 4 4-4' } },
      { tag: 'path', attrs: { d: 'M4.5 15.5v2.5a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2v-2.5' } },
    ],
  },
  monitor: {
    label: '모니터',
    labelKey: 'icon.monitor',
    shapes: [
      { tag: 'rect', attrs: { x: 3, y: 4.25, width: 18, height: 12, rx: 2 } },
      { tag: 'path', attrs: { d: 'M8.5 19.75h7M12 16.25v3.5' } },
    ],
  },
  mic: {
    label: '마이크',
    labelKey: 'icon.mic',
    shapes: [
      { tag: 'rect', attrs: { x: 9, y: 3.5, width: 6, height: 10, rx: 3 } },
      { tag: 'path', attrs: { d: 'M6.25 10.75a5.75 5.75 0 0 0 11.5 0M12 16.5v4M9 20.5h6' } },
    ],
  },
  magnet: {
    label: '자동 맞춤',
    labelKey: 'icon.magnet',
    shapes: [
      { tag: 'path', attrs: { d: 'M6 4.5h3.8v6.75a2.2 2.2 0 0 0 4.4 0V4.5H18v6.75a6 6 0 0 1-12 0z' } },
      { tag: 'path', attrs: { d: 'M6 8h3.8M14.2 8H18' } },
    ],
  },
  dark: {
    label: '어둡게',
    labelKey: 'icon.dark',
    shapes: [
      { tag: 'path', attrs: { d: 'M19.5 14.2A7.75 7.75 0 1 1 9.8 4.5a6.25 6.25 0 0 0 9.7 9.7z' } },
    ],
  },
  delay: {
    label: '지연 캡처',
    labelKey: 'icon.delay',
    shapes: [
      { tag: 'circle', attrs: { cx: 12, cy: 13, r: 7.25 } },
      { tag: 'path', attrs: { d: 'M12 9.5v3.5l2.5 1.5M9.5 3.5h5M12 3.5v2.2' } },
    ],
  },
  eraser: {
    label: '지우개',
    labelKey: 'icon.eraser',
    shapes: [
      { tag: 'path', attrs: { d: 'M8 19.5l-4-4a1.5 1.5 0 0 1 0-2.1l9.4-9.4a1.5 1.5 0 0 1 2.1 0l4.5 4.5a1.5 1.5 0 0 1 0 2.1L12 19.5z' } },
      { tag: 'path', attrs: { d: 'M9.5 8.5l6 6' } },
      { tag: 'path', attrs: { d: 'M11.5 19.5h8.5' } },
    ],
  },
  undo: {
    label: '되돌리기',
    labelKey: 'icon.undo',
    shapes: [
      { tag: 'path', attrs: { d: 'M8.5 6L4 10.5 8.5 15' } },
      { tag: 'path', attrs: { d: 'M4.5 10.5H14a5.25 5.25 0 0 1 0 10.5h-2.5' } },
    ],
  },
  check: {
    label: '확인',
    labelKey: 'icon.check',
    shapes: [
      { tag: 'path', attrs: { d: 'M5 12.5l4.5 4.5L19 7' } },
    ],
  },
  warning: {
    label: '주의',
    labelKey: 'icon.warning',
    shapes: [
      { tag: 'path', attrs: { d: 'M10.4 4.8L3.4 17a1.8 1.8 0 0 0 1.6 2.7h14a1.8 1.8 0 0 0 1.6-2.7l-7-12.2a1.8 1.8 0 0 0-3.2 0z' } },
      { tag: 'path', attrs: { d: 'M12 10v3.8' } },
      { tag: 'circle', role: 'dot', attrs: { cx: 12, cy: 16.7, r: 0.9 } },
    ],
  },
  quit: {
    label: '끝내기',
    labelKey: 'icon.quit',
    shapes: [
      { tag: 'path', attrs: { d: 'M12 3.75v7.25' } },
      { tag: 'path', attrs: { d: 'M17.5 6.75a7.75 7.75 0 1 1-11 0' } },
    ],
  },

  // ─────────────── 화면 보조 3종 ───────────────
  lock: {
    label: '보호된 화면',
    labelKey: 'icon.lock',
    shapes: [
      { tag: 'rect', attrs: { x: 5, y: 10.5, width: 14, height: 9.25, rx: 2 } },
      { tag: 'path', attrs: { d: 'M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5' } },
      { tag: 'circle', role: 'dot', attrs: { cx: 12, cy: 15, r: 0.95 } },
    ],
  },
  'mic-off': {
    label: '마이크 꺼짐',
    labelKey: 'icon.micOff',
    shapes: [
      { tag: 'path', attrs: { d: 'M9 9.5v1a3 3 0 0 0 5.1 2.1M15 10.5V6.5a3 3 0 0 0-5.4-1.8' } },
      { tag: 'path', attrs: { d: 'M6.25 10.75a5.75 5.75 0 0 0 9.6 4.2M17.5 12.2c.16-.47.25-.96.25-1.45M12 16.5v4M9 20.5h6' } },
      { tag: 'path', attrs: { d: 'M4 4l16 16' } },
    ],
  },
  close: {
    label: '닫기',
    labelKey: 'icon.close',
    shapes: [
      { tag: 'path', attrs: { d: 'M6 6l12 12M18 6L6 18' } },
    ],
  },

  // ─────────────── 그리기 도구 8종(35종 포함분) ───────────────
  rect: {
    label: '사각형',
    labelKey: 'icon.rect',
    shapes: [
      { tag: 'rect', attrs: { x: 3.75, y: 5.75, width: 16.5, height: 12.5, rx: 2.2 } },
    ],
  },
  ellipse: {
    label: '타원',
    labelKey: 'icon.ellipse',
    shapes: [
      { tag: 'ellipse', attrs: { cx: 12, cy: 12, rx: 8.25, ry: 6.25 } },
    ],
  },
  arrow: {
    label: '화살표',
    labelKey: 'icon.arrow',
    shapes: [
      { tag: 'path', attrs: { d: 'M5.5 18.5L18.5 5.5' } },
      { tag: 'path', attrs: { d: 'M10.5 5.5h8v8' } },
    ],
  },
  pen: {
    label: '펜',
    labelKey: 'icon.pen',
    shapes: [
      { tag: 'path', attrs: { d: 'M14.5 4.5l5 5L8.5 20.5H3.5v-5z' } },
      { tag: 'path', attrs: { d: 'M12.5 6.5l5 5' } },
    ],
  },
  marker: {
    label: '형광펜',
    labelKey: 'icon.marker',
    shapes: [
      { tag: 'path', attrs: { d: 'M14.5 4l5.5 5.5-7 7-3.5 1.5-2-2 1.5-3.5z' } },
      { tag: 'path', attrs: { d: 'M5 17.5L4 20h3.5l1-1' } },
      { tag: 'path', attrs: { d: 'M12.5 20h7.5' } },
    ],
  },
  mosaic: {
    label: '모자이크',
    labelKey: 'icon.mosaic',
    shapes: [
      { tag: 'rect', attrs: { x: 4, y: 4, width: 16, height: 16, rx: 2 } },
      { tag: 'path', attrs: { d: 'M4 9.33h16M4 14.67h16M9.33 4v16M14.67 4v16' } },
      { tag: 'rect', role: 'fill', attrs: { x: 4.5, y: 4.5, width: 4.6, height: 4.6, opacity: 0.55 } },
      { tag: 'rect', role: 'fill', attrs: { x: 14.9, y: 9.7, width: 4.6, height: 4.6, opacity: 0.55 } },
      { tag: 'rect', role: 'fill', attrs: { x: 9.7, y: 14.9, width: 4.6, height: 4.6, opacity: 0.55 } },
    ],
  },
  text: {
    label: '글자',
    labelKey: 'icon.text',
    shapes: [
      { tag: 'path', attrs: { d: 'M5.5 6.5h13' } },
      { tag: 'path', attrs: { d: 'M5.5 6.5v1.8M18.5 6.5v1.8' } },
      { tag: 'path', attrs: { d: 'M12 6.5v11.5' } },
      { tag: 'path', attrs: { d: 'M9.5 18h5' } },
    ],
  },
  redo: {
    label: '다시 하기',
    labelKey: 'icon.redo',
    shapes: [
      { tag: 'path', attrs: { d: 'M15.5 6L20 10.5 15.5 15' } },
      { tag: 'path', attrs: { d: 'M19.5 10.5H10a5.25 5.25 0 0 0 0 10.5h2.5' } },
    ],
  },

  // ─────────────── 추가 7종(같은 24 격자 규칙) ───────────────
  eyedropper: {
    label: '스포이트',
    labelKey: 'icon.eyedropper',
    shapes: [
      { tag: 'path', attrs: { d: 'M14.2 5.3l4.5 4.5a1.5 1.5 0 0 1 0 2.1l-1.3 1.3-6.6-6.6 1.3-1.3a1.5 1.5 0 0 1 2.1 0z' } },
      { tag: 'path', attrs: { d: 'M9.6 7.8l6.6 6.6' } },
      { tag: 'path', attrs: { d: 'M11.6 9.8L5.2 16.2a1.4 1.4 0 0 0-.4.9v2.4h2.4c.35 0 .7-.14.95-.4l6.4-6.4' } },
      { tag: 'circle', role: 'dot', attrs: { cx: 4.5, cy: 20.5, r: 0.9 } },
    ],
  },
  font: {
    label: '글꼴',
    labelKey: 'icon.font',
    shapes: [
      { tag: 'path', attrs: { d: 'M4 18L8 6.5L12 18' } },
      { tag: 'path', attrs: { d: 'M5.5 14h5' } },
      { tag: 'path', attrs: { d: 'M14.5 12.2a2.6 2.6 0 0 1 5.2.3v5.5' } },
      { tag: 'path', attrs: { d: 'M19.7 14.5h-2.9a2.1 2.1 0 0 0 0 3.5c1.2 0 2.9-.8 2.9-2' } },
    ],
  },
  slider: {
    label: '조절',
    labelKey: 'icon.slider',
    shapes: [
      { tag: 'path', attrs: { d: 'M4 12h6.5M15.5 12H20' } },
      { tag: 'circle', attrs: { cx: 13, cy: 12, r: 2.5 } },
      { tag: 'path', attrs: { d: 'M6 6.5v2M12 6.5v2M18 6.5v2' } },
      { tag: 'path', attrs: { d: 'M6 16.5v1M18 16.5v1' } },
    ],
  },
  'check-circle': {
    label: '완료',
    labelKey: 'icon.checkCircle',
    shapes: [
      { tag: 'circle', attrs: { cx: 12, cy: 12, r: 8.25 } },
      { tag: 'path', attrs: { d: 'M8.5 12.2l2.3 2.3 4.7-4.8' } },
    ],
  },
  'warning-triangle': {
    label: '경고',
    labelKey: 'icon.warningTriangle',
    shapes: [
      { tag: 'path', attrs: { d: 'M10.4 4.8L3.4 17a1.8 1.8 0 0 0 1.6 2.7h14a1.8 1.8 0 0 0 1.6-2.7l-7-12.2a1.8 1.8 0 0 0-3.2 0z' } },
      { tag: 'path', attrs: { d: 'M12 10v3.8' } },
      { tag: 'circle', role: 'dot', attrs: { cx: 12, cy: 16.7, r: 0.9 } },
    ],
  },
  'external-link': {
    label: '외부 링크',
    labelKey: 'icon.externalLink',
    shapes: [
      { tag: 'path', attrs: { d: 'M14 4.5h5.5V10' } },
      { tag: 'path', attrs: { d: 'M19.5 4.5L11 13' } },
      { tag: 'path', attrs: { d: 'M18.5 13.5v4a2 2 0 0 1-2 2h-10a2 2 0 0 1-2-2v-10a2 2 0 0 1 2-2h4' } },
    ],
  },
  'help-circle': {
    label: '도움말',
    labelKey: 'icon.helpCircle',
    shapes: [
      { tag: 'circle', attrs: { cx: 12, cy: 12, r: 8.25 } },
      { tag: 'path', attrs: { d: 'M9.6 9.5a2.4 2.4 0 0 1 4.6.8c0 1.5-2.2 1.9-2.2 3.3' } },
      { tag: 'circle', role: 'dot', attrs: { cx: 12, cy: 16.6, r: 0.9 } },
    ],
  },
  'window-top': {
    label: '창을 맨 위에',
    labelKey: 'integ.icon.windowTop',
    shapes: [
      { tag: 'path', attrs: { d: 'M12.5 7.25H5.7a2.2 2.2 0 0 0-2.2 2.2v9.35a2.2 2.2 0 0 0 2.2 2.2h12.6a2.2 2.2 0 0 0 2.2-2.2v-5.3' } },
      { tag: 'path', attrs: { d: 'M3.5 11.25h9.5' } },
      { tag: 'circle', role: 'dot', attrs: { cx: 6.1, cy: 9.25, r: 0.8 } },
      { tag: 'circle', role: 'dot', attrs: { cx: 8.6, cy: 9.25, r: 0.8 } },
      { tag: 'path', attrs: { d: 'M16.5 3.25l4.25 4.25-1.5 1.5-1.1-.35-2.15 2.15.3 1.4-1.1 1.1-3.4-3.4 1.1-1.1 1.4.3 2.15-2.15-.35-1.1z' } },
      { tag: 'path', attrs: { d: 'M13.5 10.5L10.75 13.25' } },
    ],
  },
  line: {
    label: '직선',
    labelKey: 'integ.icon.line',
    shapes: [
      { tag: 'path', attrs: { d: 'M5.5 18.5L18.5 5.5' } },
    ],
  },
  trash: {
    label: '모두 지우기',
    labelKey: 'integ.icon.trash',
    shapes: [
      { tag: 'path', attrs: { d: 'M4.5 7.25h15' } },
      { tag: 'path', attrs: { d: 'M9.5 7.25V5.4a1.4 1.4 0 0 1 1.4-1.4h2.2a1.4 1.4 0 0 1 1.4 1.4v1.85' } },
      { tag: 'path', attrs: { d: 'M6.75 7.25l.85 11.1a1.75 1.75 0 0 0 1.74 1.65h5.32a1.75 1.75 0 0 0 1.74-1.65l.85-11.1' } },
      { tag: 'path', attrs: { d: 'M10.25 11.2v5.3M13.75 11.2v5.3' } },
    ],
  },

  // ─────────────── 고른 도구용 채움 변형(*-fill 9종) ───────────────
  'line-fill': {
    label: '직선',
    labelKey: 'integ.icon.line',
    shapes: [
      { tag: 'path', attrs: { d: 'M5.5 18.5L18.5 5.5' } },
      { tag: 'circle', role: 'dot', attrs: { cx: 5.5, cy: 18.5, r: 2 } },
      { tag: 'circle', role: 'dot', attrs: { cx: 18.5, cy: 5.5, r: 2 } },
    ],
  },
  'rect-fill': {
    label: '사각형',
    labelKey: 'icon.rect',
    shapes: [
      { tag: 'rect', role: 'fill', attrs: { x: 3.75, y: 5.75, width: 16.5, height: 12.5, rx: 2.2, opacity: 0.34 } },
      { tag: 'rect', attrs: { x: 3.75, y: 5.75, width: 16.5, height: 12.5, rx: 2.2 } },
    ],
  },
  'ellipse-fill': {
    label: '타원',
    labelKey: 'icon.ellipse',
    shapes: [
      { tag: 'ellipse', role: 'fill', attrs: { cx: 12, cy: 12, rx: 8.25, ry: 6.25, opacity: 0.34 } },
      { tag: 'ellipse', attrs: { cx: 12, cy: 12, rx: 8.25, ry: 6.25 } },
    ],
  },
  'arrow-fill': {
    label: '화살표',
    labelKey: 'icon.arrow',
    shapes: [
      { tag: 'path', attrs: { d: 'M5.5 18.5L15.2 8.8' } },
      { tag: 'path', role: 'fill', attrs: { d: 'M18.8 5.2l-7.2 1.9 5.3 5.3z' } },
      { tag: 'path', attrs: { d: 'M18.8 5.2l-7.2 1.9 5.3 5.3z' } },
    ],
  },
  'pen-fill': {
    label: '펜',
    labelKey: 'icon.pen',
    shapes: [
      { tag: 'path', role: 'fill', attrs: { d: 'M14.5 4.5l5 5-9.5 9.5H5v-5z', opacity: 0.34 } },
      { tag: 'path', attrs: { d: 'M14.5 4.5l5 5L8.5 20.5H3.5v-5z' } },
      { tag: 'path', attrs: { d: 'M12.5 6.5l5 5' } },
    ],
  },
  'marker-fill': {
    label: '형광펜',
    labelKey: 'icon.marker',
    shapes: [
      { tag: 'path', role: 'fill', attrs: { d: 'M14.5 4l5.5 5.5-7 7-3.5 1.5-2-2 1.5-3.5z', opacity: 0.34 } },
      { tag: 'path', attrs: { d: 'M14.5 4l5.5 5.5-7 7-3.5 1.5-2-2 1.5-3.5z' } },
      { tag: 'path', attrs: { d: 'M5 17.5L4 20h3.5l1-1' } },
      { tag: 'path', attrs: { d: 'M12.5 20h7.5' } },
    ],
  },
  'mosaic-fill': {
    label: '모자이크',
    labelKey: 'icon.mosaic',
    shapes: [
      { tag: 'rect', role: 'fill', attrs: { x: 4.5, y: 4.5, width: 4.6, height: 4.6 } },
      { tag: 'rect', role: 'fill', attrs: { x: 14.9, y: 4.5, width: 4.6, height: 4.6, opacity: 0.5 } },
      { tag: 'rect', role: 'fill', attrs: { x: 9.7, y: 9.7, width: 4.6, height: 4.6 } },
      { tag: 'rect', role: 'fill', attrs: { x: 14.9, y: 9.7, width: 4.6, height: 4.6 } },
      { tag: 'rect', role: 'fill', attrs: { x: 4.5, y: 14.9, width: 4.6, height: 4.6, opacity: 0.5 } },
      { tag: 'rect', role: 'fill', attrs: { x: 9.7, y: 14.9, width: 4.6, height: 4.6 } },
      { tag: 'rect', attrs: { x: 4, y: 4, width: 16, height: 16, rx: 2 } },
      { tag: 'path', attrs: { d: 'M4 9.33h16M4 14.67h16M9.33 4v16M14.67 4v16' } },
    ],
  },
  'text-fill': {
    label: '글자',
    labelKey: 'icon.text',
    shapes: [
      { tag: 'rect', role: 'fill', attrs: { x: 4, y: 4.5, width: 16, height: 15, rx: 2, opacity: 0.25 } },
      { tag: 'path', attrs: { d: 'M5.5 6.5h13' } },
      { tag: 'path', attrs: { d: 'M5.5 6.5v1.8M18.5 6.5v1.8' } },
      { tag: 'path', attrs: { d: 'M12 6.5v11.5' } },
      { tag: 'path', attrs: { d: 'M9.5 18h5' } },
    ],
  },
  'eraser-fill': {
    label: '지우개',
    labelKey: 'icon.eraser',
    shapes: [
      { tag: 'path', role: 'fill', attrs: { d: 'M8 19.5l-4-4a1.5 1.5 0 0 1 0-2.1l9.4-9.4a1.5 1.5 0 0 1 2.1 0l4.5 4.5a1.5 1.5 0 0 1 0 2.1L12 19.5z', opacity: 0.34 } },
      { tag: 'path', attrs: { d: 'M8 19.5l-4-4a1.5 1.5 0 0 1 0-2.1l9.4-9.4a1.5 1.5 0 0 1 2.1 0l4.5 4.5a1.5 1.5 0 0 1 0 2.1L12 19.5z' } },
      { tag: 'path', attrs: { d: 'M9.5 8.5l6 6' } },
      { tag: 'path', attrs: { d: 'M11.5 19.5h8.5' } },
    ],
  },
};
