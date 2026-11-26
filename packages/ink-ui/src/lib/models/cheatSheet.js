// S16 단축키 한 장(F17) — state.shortcuts의 현재 값으로 세 열 생성(비운 칸 제외, 쉬운 말·다국어·OS 맞춤, R2-FIX-7)
import { keycaps } from '@screen-ink/client';
import { normalizeLang, normalizeOs, translate } from '../i18n/core.js';

const GLOBAL_ORDER = [
  { id: 'region', key: 'shortcuts.slot.region' },
  { id: 'window', key: 'shortcuts.slot.window' },
  { id: 'record', key: 'shortcuts.slot.record' },
  { id: 'draw', key: 'shortcuts.slot.draw' },
  { id: 'zoom', key: 'shortcuts.slot.zoom' },
  { id: 'board', key: 'shortcuts.slot.board' },
  { id: 'topmost', key: 'shortcuts.slot.topmost' },
  { id: 'pause', key: 'shortcuts.slot.pause' },
  { id: 'restore', key: 'shortcuts.slot.restore' },
];

/**
 * 여러 칸을 한 줄로 묶는다. 비운 칸(`!combo`)은 빼고, 전부 비었으면 null을 반환한다.
 * 다중 키 조합이 둘 이상일 때는 키캡이 붙어 보이지 않도록 `keyGroups` + `separator`("또는"/"or")로 나눈다(R2-FIX-7).
 * @param {Map<string, { label: string, combo: string, windowsOnly?: boolean }>} byId
 * @param {Array<{ id: string, key?: string, shortLabel?: string }>} entries
 * @param {string} platform
 * @param {{ fixedLabel?: string, separator?: string, tr?: (k: string) => string }} [opts]
 */
function buildGroupedRow(byId, entries, platform, opts = {}) {
  const isMac = normalizeOs(platform) === 'mac';
  const tr = opts.tr ?? ((k) => k);
  const present = [];
  for (const entry of entries) {
    const row = byId.get(entry.id);
    if (!row || !row.combo) continue;
    if (isMac && row.windowsOnly) continue;
    present.push({
      label: entry.key ? tr(entry.key) : entry.shortLabel ?? row.label,
      combo: row.combo,
      caps: keycaps(row.combo, platform),
    });
  }
  if (present.length === 0) return null;

  const label = opts.fixedLabel ?? present.map((p) => p.label).join(' · ');

  // 두 칸이 같은 수식키로 시작하면(예: Ctrl+Z / Ctrl+Y) 공통 수식키를 한 번만 묶어 F17 밀도를 유지한다.
  if (
    present.length === 2 &&
    present[0].caps.length === 2 &&
    present[1].caps.length === 2 &&
    present[0].caps[0] === present[1].caps[0]
  ) {
    return {
      label,
      keys: [present[0].caps[0], present[0].caps[1], present[1].caps[1]],
    };
  }

  // 둘 이상 묶였는데 다중 키 조합이 포함되면 그룹 사이에 구분자("또는"/"or")를 둔다(R2-FIX-7).
  const hasMultiKey = present.length >= 2 && present.some((p) => p.caps.length >= 2);
  if (hasMultiKey) {
    return {
      label,
      keyGroups: present.map((p) => p.caps),
      separator: opts.separator ?? tr('shortcuts.cheat.or'),
    };
  }

  return {
    label,
    keys: present.flatMap((p) => p.caps),
  };
}

/**
 * `state.shortcuts`에서 S16 단축키 한 장의 세 열을 만든다.
 * @param {Array<{ id: string, scope: string, group: string, label: string, combo: string, windowsOnly?: boolean }>} shortcuts
 * @param {{ platform?: 'windows' | 'macos', mode?: 'draw' | 'capture' | 'zoom', lang?: string }} [options]
 * @returns {Array<{
 *   id: string,
 *   title: string,
 *   active: boolean,
 *   rows: Array<{
 *     label: string,
 *     keys?: string[],
 *     keyGroups?: string[][],
 *     separator?: string,
 *     keyPrefix?: string,
 *     note?: string,
 *     noteMono?: boolean,
 *   }>,
 * }>}
 */
export function buildCheatSheetColumns(
  shortcuts = [],
  { platform = 'windows', mode = 'draw', lang = 'ko' } = {},
) {
  const byId = new Map(shortcuts.map((s) => [s.id, s]));
  const osKey = normalizeOs(platform);
  const isMac = osKey === 'mac';
  const l = normalizeLang(lang);
  const tr = (key) => translate(l, key, undefined, osKey);

  const modCtrl = isMac ? '⌘' : 'Ctrl';
  const modAlt = isMac ? '⌥' : 'Alt';
  const modShift = isMac ? '⇧' : 'Shift';
  const wheelKey = tr('shortcuts.cheat.key.wheel');
  const arrowsKey = tr('shortcuts.cheat.key.arrows');
  const orSep = tr('shortcuts.cheat.or');

  // 1열: 언제든 (최대 8줄, 비운 칸 제외)
  const globalRows = [];
  for (const item of GLOBAL_ORDER) {
    const row = byId.get(item.id);
    if (!row || !row.combo) continue;
    if (isMac && row.windowsOnly) continue;
    globalRows.push({
      label: tr(item.key),
      keys: keycaps(row.combo, platform),
    });
    if (globalRows.length >= 8) break;
  }

  // 2열: 지금 화면 (주황 강조) + 3열: 보조/도형 조작
  let modeTitle = tr('shortcuts.cheat.col.drawNow');
  let modeRows = [];
  let extraTitle = tr('shortcuts.cheat.col.dragDraw');
  let extraRows = [];

  if (mode === 'capture') {
    modeTitle = tr('shortcuts.cheat.col.captureNow');
    modeRows = [
      buildGroupedRow(byId, [{ id: 'c-detect', key: 'shortcuts.slot.cDetect' }], platform, { tr }),
      buildGroupedRow(byId, [{ id: 'c-copy', key: 'shortcuts.slot.cCopy' }], platform, { tr }),
      buildGroupedRow(
        byId,
        [
          { id: 'c-save', key: 'shortcuts.slot.cSave' },
          { id: 'c-quick', key: 'shortcuts.slot.cQuick' },
        ],
        platform,
        { fixedLabel: tr('shortcuts.cheat.row.saveQuick'), separator: orSep, tr },
      ),
      buildGroupedRow(byId, [{ id: 'c-last', key: 'shortcuts.slot.cLast' }], platform, { tr }),
      buildGroupedRow(
        byId,
        [{ id: 'c-prev' }, { id: 'c-next' }],
        platform,
        { fixedLabel: tr('shortcuts.cheat.row.prevRange'), tr },
      ),
      buildGroupedRow(byId, [{ id: 'c-full', key: 'shortcuts.slot.cFull' }], platform, { tr }),
      buildGroupedRow(
        byId,
        [{ id: 'c-cursor' }, { id: 'c-color' }],
        platform,
        { fixedLabel: tr('shortcuts.cheat.row.cursorColor'), tr },
      ),
      buildGroupedRow(byId, [{ id: 'c-cancel', key: 'shortcuts.slot.cCancel' }], platform, { tr }),
    ].filter(Boolean);

    extraTitle = tr('shortcuts.cheat.col.fineCapture');
    extraRows = [
      { label: tr('shortcuts.cheat.gesture.resize'), keys: [wheelKey] },
      { label: tr('shortcuts.cheat.gesture.move1px'), keys: [arrowsKey] },
      { label: tr('shortcuts.cheat.gesture.grow1px'), keys: [modCtrl, arrowsKey] },
      { label: tr('shortcuts.cheat.gesture.shrink1px'), keys: [modShift, arrowsKey] },
      { label: tr('shortcuts.cheat.gesture.pauseSnap'), keys: [modAlt] },
      { label: tr('shortcuts.cheat.gesture.confirmCopy'), note: tr('shortcuts.cheat.note.doubleClick') },
      { label: tr('shortcuts.cheat.gesture.reselect'), note: tr('shortcuts.cheat.note.rightClick') },
    ];
  } else if (mode === 'zoom') {
    modeTitle = tr('shortcuts.cheat.col.zoomNow');
    // 확대 크기는 휠로만, 확대 중 잡는 키는 끝내기(Esc) 하나 — 방향키·클릭은 아래 앱으로
    modeRows = [
      { label: tr('shortcuts.cheat.gesture.zoomLevel'), keys: [wheelKey] },
      buildGroupedRow(byId, [{ id: 'draw' }], platform, {
        fixedLabel: tr('shortcuts.cheat.row.stopAndDraw'),
        tr,
      }),
      buildGroupedRow(byId, [{ id: 'z-exit' }], platform, {
        fixedLabel: tr('shortcuts.cheat.row.exit'),
        tr,
      }),
    ].filter(Boolean);

    extraTitle = tr('shortcuts.cheat.col.zoomControls');
    extraRows = [
      { label: tr('shortcuts.cheat.gesture.scrollBelow'), keys: [modShift, wheelKey] },
      { label: tr('shortcuts.cheat.gesture.followCursor'), note: tr('shortcuts.cheat.note.cursorMove') },
      { label: tr('shortcuts.cheat.gesture.arrowsClick'), note: tr('shortcuts.cheat.note.passThrough') },
    ];
  } else {
    // 기본: 그리는 중(F17)
    modeTitle = tr('shortcuts.cheat.col.drawNow');
    modeRows = [
      buildGroupedRow(
        byId,
        [
          { id: 'd-pen', key: 'shortcuts.slot.dPen' },
          { id: 'd-marker', key: 'shortcuts.slot.dMarker' },
          { id: 'd-text', key: 'shortcuts.slot.dText' },
        ],
        platform,
        { fixedLabel: tr('shortcuts.cheat.row.toolsBasic'), tr },
      ),
      buildGroupedRow(
        byId,
        [{ id: 'd-select' }, { id: 'd-eraser' }],
        platform,
        { fixedLabel: tr('shortcuts.cheat.row.toolsEdit'), tr },
      ),
      buildGroupedRow(byId, [{ id: 'd-clear', key: 'shortcuts.slot.dClear' }], platform, { tr }),
      buildGroupedRow(
        byId,
        [
          { id: 'd-red' },
          { id: 'd-yellow' },
          { id: 'd-green' },
          { id: 'd-orange' },
          { id: 'd-blue' },
          { id: 'd-lime' },
        ],
        platform,
        { fixedLabel: tr('shortcuts.cheat.row.colors'), tr },
      ),
      buildGroupedRow(
        byId,
        [{ id: 'd-white' }, { id: 'd-black' }],
        platform,
        { fixedLabel: tr('shortcuts.cheat.row.boards'), tr },
      ),
      buildGroupedRow(
        byId,
        [
          { id: 'd-undo', key: 'shortcuts.slot.dUndo' },
          { id: 'd-redo', key: 'shortcuts.slot.dRedo' },
        ],
        platform,
        { fixedLabel: tr('shortcuts.cheat.row.undoRedo'), separator: orSep, tr },
      ),
      {
        label: tr('shortcuts.cheat.row.zoomWidth'),
        keyGroups: [[wheelKey], [modCtrl, wheelKey]],
        separator: orSep,
      },
      buildGroupedRow(byId, [{ id: 'd-exit' }], platform, {
        fixedLabel: tr('shortcuts.cheat.row.exitKeep'),
        tr,
      }),
    ].filter(Boolean);

    extraTitle = tr('shortcuts.cheat.col.dragDraw');
    const centerRow = buildGroupedRow(byId, [{ id: 'd-center' }], platform, {
      fixedLabel: tr('shortcuts.cheat.row.centerCursor'),
      tr,
    });
    extraRows = [
      { label: tr('shortcuts.cheat.gesture.straightLine'), keys: [modShift] },
      { label: tr('shortcuts.cheat.gesture.rectangle'), keys: [modCtrl] },
      { label: tr('shortcuts.cheat.gesture.arrow'), keys: [modCtrl, modShift] },
      { label: tr('shortcuts.cheat.gesture.circle'), keys: ['Tab'] },
      { label: tr('shortcuts.cheat.gesture.dashed'), keyPrefix: '+', keys: [modAlt] },
      {
        label: tr('shortcuts.cheat.gesture.holdShape'),
        note: tr('shortcuts.cheat.gesture.holdTime'),
        noteMono: true,
      },
      { label: tr('shortcuts.cheat.gesture.panZoom'), note: tr('shortcuts.cheat.note.rightClick') },
      ...(centerRow ? [centerRow] : []),
    ];
  }

  return [
    { id: 'global', title: tr('shortcuts.cheat.col.global'), active: false, rows: globalRows },
    { id: `mode-${mode}`, title: modeTitle, active: true, rows: modeRows },
    { id: `extra-${mode}`, title: extraTitle, active: false, rows: extraRows },
  ];
}
