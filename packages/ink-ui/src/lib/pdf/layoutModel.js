// 내 단축키 PDF(시안 4 책상 옆 한 장) 순수 레이아웃 모델 — 두 OS·두 언어 동일 구조, 묶음 비분할 2쪽 넘침 지원
import defaults from '../../../../../docs/shortcuts/defaults-v1.json' with { type: 'json' };
import { normalizeLang, normalizeOs, translate } from '../i18n/core.js';

export const PAGE_SIZES = {
  A4: { name: 'A4', width: 841.89, height: 595.28 },
  LETTER: { name: 'LETTER', width: 792, height: 612 },
};

const HERO_DEFS = [
  { id: 'region', icon: 'region' },
  { id: 'window', icon: 'window' },
  { id: 'record', icon: 'record' },
  { id: 'draw', icon: 'draw' },
  { id: 'zoom', icon: 'zoom' },
  { id: 'board', icon: 'board' },
];

const HERO_IDS = new Set(HERO_DEFS.map((h) => h.id));
const GLOBAL_EXTRA_ORDER = ['last', 'full', 'active', 'delay', 'restore', 'pause', 'topmost'];
const LATER_EXTRA_ORDER = ['pin', 'pinToggle', 'timer'];

const DRAW_COLOR_DEFS = [
  { id: 'd-red', key: 'pdf.color.red', hex: '#E5484D' },
  { id: 'd-yellow', key: 'pdf.color.yellow', hex: '#F2B705' },
  { id: 'd-green', key: 'pdf.color.green', hex: '#2BA36B' },
  { id: 'd-orange', key: 'pdf.color.orange', hex: '#F97316' },
  { id: 'd-blue', key: 'pdf.color.blue', hex: '#2F6FED' },
  { id: 'd-lime', key: 'pdf.color.lime', hex: '#84CC16' },
];

const DRAW_LEFT_ORDER = [
  'd-pen',
  'd-marker',
  'd-text',
  'd-select',
  'd-eraser',
  'd-clear',
  'd-spot',
  'd-check',
  'd-number',
  'd-white',
  'd-black',
];

const DRAW_RIGHT_ORDER = [
  'd-undo',
  'd-redo',
  'd-copy',
  'd-save',
  'd-restore',
  'd-center',
  'd-zin',
  'd-zout',
  'd-wup',
  'd-wdown',
  'd-help',
  'd-exit',
];

const SLOT_I18N_KEYS = {
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

const MAC_KEY_MAP = {
  ctrl: '⌃',
  control: '⌃',
  option: '⌥',
  alt: '⌥',
  shift: '⇧',
  cmd: '⌘',
  command: '⌘',
  meta: '⌘',
  up: '↑',
  arrowup: '↑',
  down: '↓',
  arrowdown: '↓',
  left: '←',
  arrowleft: '←',
  right: '→',
  arrowright: '→',
  enter: '↩',
  return: '↩',
  tab: '⇥',
  backspace: '⌫',
  delete: '⌫',
  esc: 'Esc',
  escape: 'Esc',
  space: 'Space',
};

const WIN_KEY_MAP = {
  ctrl: 'Ctrl',
  control: 'Ctrl',
  alt: 'Alt',
  option: 'Alt',
  shift: 'Shift',
  win: 'Win',
  meta: 'Win',
  super: 'Win',
  cmd: 'Ctrl',
  command: 'Ctrl',
  up: '↑',
  arrowup: '↑',
  down: '↓',
  arrowdown: '↓',
  left: '←',
  arrowleft: '←',
  right: '→',
  arrowright: '→',
  enter: 'Enter',
  return: 'Enter',
  tab: 'Tab',
  backspace: 'Backspace',
  delete: 'Del',
  esc: 'Esc',
  escape: 'Esc',
  space: 'Space',
};

/**
 * 미국 지역 여부 판별 (영어 + 미국 지역이면 Letter 용지 사용).
 * @param {string | undefined} region
 */
/**
 * 이 컴퓨터 시간대의 오늘 날짜 `YYYY-MM-DD`(파일 이름·PDF 아래 줄). `toISOString()`은 세계 표준시라
 * 한국 아침 9시 전에는 어제 날짜가 된다(2026-11-28 맥 실기에서 발견).
 * @param {Date} [now]
 */
export function localDateStamp(now = new Date()) {
  const p = (/** @type {number} */ n) => String(n).padStart(2, '0');
  return `${now.getFullYear()}-${p(now.getMonth() + 1)}-${p(now.getDate())}`;
}

export function isUsRegion(region) {
  if (!region) return false;
  const s = String(region).trim().toUpperCase();
  return s === 'US' || s === 'USA' || s.endsWith('-US') || s.endsWith('_US');
}

function normalizeComboForCompare(combo, osKey) {
  if (!combo) return '';
  return String(combo)
    .split('+')
    .map((part) => {
      const low = part.trim().toLowerCase();
      if (low === 'control' || low === 'ctrl') return 'ctrl';
      if (low === 'option' || low === 'alt') return osKey === 'mac' ? 'option' : 'alt';
      if (low === 'command' || low === 'cmd' || low === 'meta') return osKey === 'mac' ? 'cmd' : 'win';
      if (low === 'escape' || low === 'esc') return 'esc';
      if (low === 'return' || low === 'enter') return 'enter';
      if (low === 'spacebar' || low === 'space') return 'space';
      return low;
    })
    .filter(Boolean)
    .sort()
    .join('+');
}

/**
 * 슬롯 값이 defaults-v1.json과 다른지 판별한다.
 * @param {{ id?: string, combo?: string, modified?: boolean }} row
 * @param {'mac' | 'win'} osKey
 */
export function isShortcutModified(row, osKey) {
  if (typeof row?.modified === 'boolean') return row.modified;
  const def = defaults.slots?.[row?.id]?.[osKey];
  if (!def || !row?.combo) return false;
  return normalizeComboForCompare(row.combo, osKey) !== normalizeComboForCompare(def, osKey);
}

/**
 * 단축키 문자열을 OS별 키캡 토큰 배열로 변환한다.
 * @param {string} combo
 * @param {'mac' | 'win'} osKey
 * @returns {string[]}
 */
export function comboToTokens(combo, osKey) {
  if (!combo) return [];
  const table = osKey === 'mac' ? MAC_KEY_MAP : WIN_KEY_MAP;
  return String(combo)
    .split('+')
    .map((raw) => {
      const trimmed = raw.trim();
      if (!trimmed) return '';
      const mapped = table[trimmed.toLowerCase()];
      if (mapped) return mapped;
      return trimmed.length === 1 ? trimmed.toUpperCase() : trimmed;
    })
    .filter(Boolean);
}

function slotLabel(row, lang, osKey) {
  const key = SLOT_I18N_KEYS[row.id];
  if (key) return translate(lang, key, undefined, osKey);
  return row.label || row.id;
}

function buildShortcutRow(row, lang, osKey) {
  return {
    id: row.id,
    label: slotLabel(row, lang, osKey),
    tokens: comboToTokens(row.combo, osKey),
    modified: isShortcutModified(row, osKey),
  };
}

/**
 * 묶음 하나의 대략적인 세로 높이(pt)를 계산한다 (묶음 중간에서 자르지 않고 2쪽 넘침 판정용).
 * @param {object} sec
 */
function estimateSectionHeight(sec) {
  const headerH = 18;
  if (sec.kind === 'drawGrid') {
    const stripH = 34;
    const maxRows = Math.max(sec.leftRows.length, sec.rightRows.length);
    return headerH + stripH + maxRows * 15.6 + (sec.extraRows?.length ?? 0) * 15.6 + 6;
  }
  return headerH + (sec.rows?.length ?? 0) * 15.6 + 6;
}

/**
 * 3열 묶음들을 가용 높이에 맞춰 1쪽과 2쪽으로 나눈다. 묶음 내부는 절대 자르지 않는다.
 */
function paginateColumns(colSectionsList, availableHeight) {
  const page1Cols = [[], [], []];
  const page2Cols = [[], [], []];
  let hasPage2 = false;

  for (let colIdx = 0; colIdx < 3; colIdx += 1) {
    let used = 0;
    for (const sec of colSectionsList[colIdx]) {
      const h = estimateSectionHeight(sec);
      if (page1Cols[colIdx].length > 0 && used + h > availableHeight) {
        page2Cols[colIdx].push(sec);
        hasPage2 = true;
      } else {
        page1Cols[colIdx].push(sec);
        used += h;
      }
    }
  }

  return { page1Cols, page2Cols, hasPage2 };
}

/**
 * `InkState.shortcuts`에서 시안 4(책상 옆 한 장) PDF 레이아웃 모델을 만든다.
 * @param {{
 *   shortcuts?: Array<{ id: string, scope?: string, group?: string, label?: string, combo?: string, modified?: boolean, windowsOnly?: boolean, later?: boolean }>,
 *   lang?: string,
 *   os?: string,
 *   date?: string,
 *   version?: string,
 *   region?: string,
 * }} [options]
 */
export function buildShortcutSheetModel({
  shortcuts = [],
  lang = 'ko',
  os = 'windows',
  date,
  version = '0.1.0',
  region,
} = {}) {
  const l = normalizeLang(lang);
  const osKey = normalizeOs(os);
  const tr = (key, params) => translate(l, key, params, osKey);

  const dateStr = date || localDateStamp();
  const verRaw = String(version || '0.1.0').trim();
  const verStr = verRaw.startsWith('v') ? verRaw : `v${verRaw}`;
  const osLabel = tr(osKey === 'mac' ? 'pdf.os.mac' : 'pdf.os.win');
  const useLetter = l === 'en' && isUsRegion(region);
  const pageSize = useLetter ? PAGE_SIZES.LETTER : PAGE_SIZES.A4;

  // 비운 칸과 macOS 비지원(windowsOnly) 슬롯 제거
  const active = (shortcuts || []).filter((s) => {
    if (!s || !s.combo || !String(s.combo).trim()) return false;
    if (osKey === 'mac' && (s.windowsOnly || defaults.slots?.[s.id]?.mac === null)) return false;
    return true;
  });
  const byId = new Map(active.map((s) => [s.id, s]));

  // 상단 6개 큰 타일
  const hero = HERO_DEFS.map((def) => {
    const row = byId.get(def.id);
    if (!row) return null;
    return {
      id: def.id,
      icon: def.icon,
      title: slotLabel(row, l, osKey),
      tokens: comboToTokens(row.combo, osKey),
      modified: isShortcutModified(row, osKey),
    };
  }).filter(Boolean);

  // 1열: 찍은 뒤 + 언제든
  const usedCaptureIds = new Set();
  const captureRows = [];
  for (const s of active) {
    if (s.scope === 'capture' || s.group === 'capture-mode') {
      captureRows.push(buildShortcutRow(s, l, osKey));
      usedCaptureIds.add(s.id);
    }
  }

  const anytimeRows = [];
  const usedGlobalIds = new Set([...HERO_IDS, ...LATER_EXTRA_ORDER]);
  for (const id of GLOBAL_EXTRA_ORDER) {
    const s = byId.get(id);
    if (s) {
      anytimeRows.push(buildShortcutRow(s, l, osKey));
      usedGlobalIds.add(id);
    }
  }
  for (const s of active) {
    if ((s.scope === 'global' || !s.scope) && !usedGlobalIds.has(s.id) && !s.later) {
      anytimeRows.push(buildShortcutRow(s, l, osKey));
      usedGlobalIds.add(s.id);
    }
  }

  const col0Sections = [];
  if (captureRows.length > 0) {
    col0Sections.push({
      id: 'afterCapture',
      title: tr('pdf.section.afterCapture'),
      tag: tr('pdf.section.afterCaptureTag'),
      kind: 'rows',
      rows: captureRows,
    });
  }
  if (anytimeRows.length > 0) {
    col0Sections.push({
      id: 'anytime',
      title: tr('pdf.section.anytime'),
      tag: tr('pdf.section.anytimeTag'),
      kind: 'rows',
      rows: anytimeRows,
    });
  }

  // 2열: 그리는 중 (색 6종 스트립 + 2소열)
  const drawColors = DRAW_COLOR_DEFS.map((c) => {
    const s = byId.get(c.id);
    if (!s) return null;
    return {
      id: c.id,
      name: tr(c.key),
      hex: c.hex,
      tokens: comboToTokens(s.combo, osKey),
      modified: isShortcutModified(s, osKey),
    };
  }).filter(Boolean);

  const usedDrawIds = new Set(DRAW_COLOR_DEFS.map((c) => c.id));
  const drawLeft = DRAW_LEFT_ORDER.map((id) => {
    const s = byId.get(id);
    if (!s) return null;
    usedDrawIds.add(id);
    return buildShortcutRow(s, l, osKey);
  }).filter(Boolean);

  const drawRight = DRAW_RIGHT_ORDER.map((id) => {
    const s = byId.get(id);
    if (!s) return null;
    usedDrawIds.add(id);
    return buildShortcutRow(s, l, osKey);
  }).filter(Boolean);

  const customDrawRows = [];
  for (const s of active) {
    if ((s.scope === 'draw' || s.group === 'draw-mode') && !usedDrawIds.has(s.id)) {
      customDrawRows.push(buildShortcutRow(s, l, osKey));
      usedDrawIds.add(s.id);
    }
  }

  const col1Sections = [];
  if (drawColors.length > 0 || drawLeft.length > 0 || drawRight.length > 0) {
    col1Sections.push({
      id: 'whileDrawing',
      title: tr('pdf.section.whileDrawing'),
      tag: tr('pdf.section.whileDrawingTag'),
      kind: 'drawGrid',
      colors: drawColors,
      leftRows: drawLeft,
      rightRows: drawRight,
    });
  }
  if (customDrawRows.length > 0) {
    col1Sections.push({
      id: 'whileDrawingExtra',
      title: `${tr('pdf.section.whileDrawing')} · ${tr('pdf.section.continued')}`,
      tag: tr('pdf.section.whileDrawingTag'),
      kind: 'rows',
      rows: customDrawRows,
    });
  }

  // 3열: 끌어서 그리기 + 확대 · 세밀하게
  const modCtrl = osKey === 'mac' ? '⌘' : 'Ctrl';
  const modAlt = osKey === 'mac' ? '⌥' : 'Alt';
  const modShift = osKey === 'mac' ? '⇧' : 'Shift';
  const dragTok = tr('pdf.key.drag');
  const wheelTok = tr('pdf.key.wheel');
  const arrowsTok = tr('pdf.key.arrows');

  const dragRows = [
    { label: tr('shortcuts.cheat.gesture.straightLine'), tokens: [modShift, dragTok] },
    { label: tr('shortcuts.cheat.gesture.rectangle'), tokens: [modCtrl, dragTok] },
    { label: tr('shortcuts.cheat.gesture.arrow'), tokens: [modCtrl, modShift, dragTok] },
    { label: tr('shortcuts.cheat.gesture.circle'), tokens: [osKey === 'mac' ? '⇥' : 'Tab', dragTok] },
    { label: tr('shortcuts.cheat.gesture.dashed'), keyPrefix: '+', tokens: [modAlt] },
    { label: tr('shortcuts.cheat.gesture.holdShape'), note: tr('shortcuts.cheat.gesture.holdTime') },
    {
      label: tr('pdf.row.zoomWidth'),
      keyGroups: [[wheelTok], [modCtrl, wheelTok]],
      separator: tr('shortcuts.cheat.or'),
    },
  ];

  const zoomExit = byId.get('z-exit');
  const zoomFineRows = [
    ...(zoomExit ? [buildShortcutRow(zoomExit, l, osKey)] : []),
    { label: tr('pdf.row.zoomLevel'), tokens: [wheelTok] },
    { label: tr('pdf.row.scrollBelow'), tokens: [modShift, wheelTok] },
    { label: tr('pdf.row.followCursor'), note: tr('shortcuts.cheat.note.cursorMove') },
    { label: tr('pdf.row.resize'), tokens: [wheelTok] },
    { label: tr('pdf.row.move1px'), tokens: [arrowsTok] },
    { label: tr('pdf.row.grow1px'), tokens: [modCtrl, arrowsTok] },
  ];

  // 표준 묶음에 속하지 않는 사용자 정의/추가 묶음이 있으면 해당 열에 섹션으로 추가
  const customSections = new Map();
  for (const s of active) {
    if (
      HERO_IDS.has(s.id) ||
      usedCaptureIds.has(s.id) ||
      usedGlobalIds.has(s.id) ||
      usedDrawIds.has(s.id) ||
      s.id === 'z-exit'
    ) {
      continue;
    }
    const gId = s.group || 'custom';
    if (!customSections.has(gId)) {
      customSections.set(gId, {
        id: `custom-${gId}`,
        title: s.groupTitle || gId,
        tag: tr('pdf.section.continued'),
        kind: 'rows',
        rows: [],
      });
    }
    customSections.get(gId).rows.push(buildShortcutRow(s, l, osKey));
  }

  const col2Sections = [
    {
      id: 'dragDraw',
      title: tr('pdf.section.dragDraw'),
      tag: tr('pdf.section.dragDrawTag'),
      kind: 'rows',
      rows: dragRows,
    },
    {
      id: 'zoomFine',
      title: tr('pdf.section.zoomFine'),
      tag: tr('pdf.section.zoomFineTag'),
      kind: 'rows',
      rows: zoomFineRows,
    },
  ];

  // 추가 묶음을 높이가 가장 낮은 열 순으로 분배
  const allCols = [col0Sections, col1Sections, col2Sections];
  for (const extraSec of customSections.values()) {
    const heights = allCols.map((secs) => secs.reduce((sum, sec) => sum + estimateSectionHeight(sec), 0));
    let minIdx = 0;
    if (heights[1] < heights[minIdx]) minIdx = 1;
    if (heights[2] < heights[minIdx]) minIdx = 2;
    allCols[minIdx].push(extraSec);
  }

  // 본문 1쪽 가용 높이 계산 (여백·헤더·키가이드·히어로·푸터 제외)
  const topReserved = 22 + 34 + 18 + (hero.length > 0 ? 64 : 0) + 18;
  const bottomReserved = 36;
  const availableColHeight = pageSize.height - topReserved - bottomReserved;

  const { page1Cols, page2Cols, hasPage2 } = paginateColumns(allCols, availableColHeight);

  // 하단 보조 기능 목록 (pin, pinToggle, timer 등 비어 있지 않은 후속/보조 항목)
  const footerExtras = LATER_EXTRA_ORDER.map((id) => byId.get(id))
    .filter(Boolean)
    .map((s) => {
      const toks = comboToTokens(s.combo, osKey);
      const comboText = osKey === 'mac' ? toks.join('') : toks.join('+');
      return `${slotLabel(s, l, osKey)} ${comboText}`;
    });
  const footerLeft =
    footerExtras.length > 0 ? `${tr('pdf.footer.extra')}: ${footerExtras.join(' · ')}` : '';
  const metaLine = `${osLabel} · ${dateStr} · ${verStr}`;

  const tipText = tr('pdf.tip');
  const pages = [
    {
      pageNumber: 1,
      showHero: hero.length > 0,
      columns: page1Cols,
      tip: hasPage2 ? null : tipText,
    },
  ];
  if (hasPage2) {
    pages.push({
      pageNumber: 2,
      showHero: false,
      columns: page2Cols,
      tip: tipText,
    });
  }

  return {
    lang: l,
    os: osKey,
    pageSize,
    header: {
      title: tr('pdf.title'),
      badge: tr('pdf.badge'),
      legendCustom: tr('pdf.legend.custom'),
      meta: metaLine,
    },
    keyGuide: {
      label: tr('pdf.legend.keysLabel'),
      text: tr('pdf.legend.keys'),
    },
    heroHeading: tr('pdf.hero.heading'),
    hero,
    footer: {
      left: footerLeft,
      right: `${tr('pdf.title')} · ${metaLine}`,
      pagePattern: (p, total) => tr('pdf.footer.page', { page: p, total }),
    },
    pages,
  };
}

/**
 * 시험 검증을 위해 레이아웃 모델에서 렌더될 모든 텍스트 조각을 추출한다.
 * @param {ReturnType<typeof buildShortcutSheetModel>} model
 * @returns {string[]}
 */
export function extractTextFragments(model) {
  const out = [];
  const push = (v) => {
    if (typeof v === 'string' && v.trim()) out.push(v);
  };

  push(model.header.title);
  push(model.header.badge);
  push(model.header.legendCustom);
  push(model.header.meta);
  push(model.keyGuide.label);
  push(model.keyGuide.text);
  push(model.heroHeading);

  for (const h of model.hero) {
    push(h.title);
    for (const tok of h.tokens) push(tok);
  }

  for (const page of model.pages) {
    for (const col of page.columns) {
      for (const sec of col) {
        push(sec.title);
        push(sec.tag);
        if (sec.kind === 'drawGrid') {
          for (const c of sec.colors) {
            push(c.name);
            for (const tok of c.tokens) push(tok);
          }
          for (const r of [...sec.leftRows, ...sec.rightRows]) {
            push(r.label);
            for (const tok of r.tokens ?? []) push(tok);
          }
        } else {
          for (const r of sec.rows ?? []) {
            push(r.label);
            if (r.keyPrefix) push(r.keyPrefix);
            if (r.note) push(r.note);
            if (r.separator) push(r.separator);
            for (const tok of r.tokens ?? []) push(tok);
            for (const grp of r.keyGroups ?? []) {
              for (const tok of grp) push(tok);
            }
          }
        }
      }
    }
    if (page.tip) push(page.tip);
    if (model.pages.length > 1) {
      push(model.footer.pagePattern(page.pageNumber, model.pages.length));
    }
  }

  if (model.footer.left) push(model.footer.left);
  push(model.footer.right);

  return out;
}
