// S16 단축키 한 장(F17) — state.shortcuts에서 현재 값으로 세 열 생성(비운 칸 제외, 맥 기호 적용)
import { keycaps } from '@screen-ink/client';

const GLOBAL_ORDER = [
  { id: 'region', label: '범위 캡처' },
  { id: 'window', label: '창 캡처' },
  { id: 'record', label: '화면 녹화' },
  { id: 'draw', label: '판서' },
  { id: 'zoom', label: '실시간 확대' },
  { id: 'board', label: '칠판' },
  { id: 'topmost', label: '창 항상 위' },
  { id: 'pause', label: '단축키 켜기·끄기' },
  { id: 'restore', label: '직전 판서 열기' },
];

/**
 * 여러 칸을 한 줄로 묶는다. 비운 칸(`!combo`)은 빼고, 전부 비었으면 null을 반환한다.
 * @param {Map<string, { label: string, combo: string }>} byId
 * @param {Array<{ id: string, shortLabel?: string }>} entries
 * @param {string} platform
 * @param {{ fixedLabel?: string }} [opts]
 */
function buildGroupedRow(byId, entries, platform, opts = {}) {
  const present = [];
  for (const entry of entries) {
    const row = byId.get(entry.id);
    if (row && row.combo) {
      present.push({
        label: entry.shortLabel ?? row.label,
        combo: row.combo,
        caps: keycaps(row.combo, platform),
      });
    }
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

  return {
    label,
    keys: present.flatMap((p) => p.caps),
  };
}

/**
 * `state.shortcuts`에서 S16 단축키 한 장의 세 열을 만든다.
 * @param {Array<{ id: string, scope: string, group: string, label: string, combo: string, windowsOnly?: boolean }>} shortcuts
 * @param {{ platform?: 'windows' | 'macos', mode?: 'draw' | 'capture' | 'zoom' }} [options]
 * @returns {Array<{ id: string, title: string, active: boolean, rows: Array<{ label: string, keys?: string[], note?: string, noteMono?: boolean }> }>}
 */
export function buildCheatSheetColumns(shortcuts = [], { platform = 'windows', mode = 'draw' } = {}) {
  const byId = new Map(shortcuts.map((s) => [s.id, s]));
  const isMac = platform === 'macos';
  const modCtrl = isMac ? '⌘' : 'Ctrl';
  const modAlt = isMac ? '⌥' : 'Alt';
  const modShift = isMac ? '⇧' : 'Shift';

  // 1열: 어디서나 (최대 8줄, 비운 칸 제외)
  const globalRows = [];
  for (const item of GLOBAL_ORDER) {
    const row = byId.get(item.id);
    if (!row || !row.combo) continue;
    globalRows.push({
      label: item.label,
      keys: keycaps(row.combo, platform),
    });
    if (globalRows.length >= 8) break;
  }

  // 2열: 지금 화면 (주황 강조) + 3열: 보조/도형 조작
  let modeTitle = '판서 화면 — 지금';
  let modeRows = [];
  let extraTitle = '끌어서 도형';
  let extraRows = [];

  if (mode === 'capture') {
    modeTitle = '캡처 화면 — 지금';
    modeRows = [
      buildGroupedRow(byId, [{ id: 'c-detect', shortLabel: '창 ↔ 요소 전환' }], platform),
      buildGroupedRow(byId, [{ id: 'c-copy', shortLabel: '복사하고 끝내기' }], platform),
      buildGroupedRow(
        byId,
        [
          { id: 'c-save', shortLabel: '저장' },
          { id: 'c-quick', shortLabel: '빠른 저장' },
        ],
        platform,
      ),
      buildGroupedRow(byId, [{ id: 'c-last', shortLabel: '직전 범위' }], platform),
      buildGroupedRow(
        byId,
        [
          { id: 'c-prev', shortLabel: '이전' },
          { id: 'c-next', shortLabel: '다음' },
        ],
        platform,
        { fixedLabel: '지난 범위' },
      ),
      buildGroupedRow(byId, [{ id: 'c-full', shortLabel: '모니터 전체' }], platform),
      buildGroupedRow(
        byId,
        [
          { id: 'c-cursor', shortLabel: '커서' },
          { id: 'c-color', shortLabel: '색 복사' },
        ],
        platform,
      ),
      buildGroupedRow(byId, [{ id: 'c-cancel', shortLabel: '취소' }], platform),
    ].filter(Boolean);

    extraTitle = '범위 미세 조작';
    extraRows = [
      { label: '넓히기·좁히기', keys: ['휠'] },
      { label: '1px 이동', keys: ['방향키'] },
      { label: '1px 늘리기', keys: [modCtrl, '방향키'] },
      { label: '1px 줄이기', keys: [modShift, '방향키'] },
      { label: '자석 잠시 끄기', keys: [modAlt] },
      { label: '확정 · 복사', note: '더블클릭' },
      { label: '다시 고르기', note: '오른쪽 클릭' },
    ];
  } else if (mode === 'zoom') {
    modeTitle = '실시간 확대 — 지금';
    modeRows = [
      buildGroupedRow(byId, [{ id: 'z-in', shortLabel: '더 확대' }], platform),
      buildGroupedRow(byId, [{ id: 'z-out', shortLabel: '덜 확대' }], platform),
      { label: '배율 조절', keys: ['휠'] },
      buildGroupedRow(byId, [{ id: 'draw', shortLabel: '멈추고 판서' }], platform),
      buildGroupedRow(byId, [{ id: 'z-exit', shortLabel: '끝내기' }], platform),
    ].filter(Boolean);

    extraTitle = '확대 조작';
    extraRows = [
      { label: '배율 올리기·내리기', keys: ['휠'] },
      { label: '화면 따라가기', note: '커서 이동' },
      { label: '끝내기', note: '오른쪽 클릭' },
    ];
  } else {
    // 기본: 판서 화면(F17)
    modeTitle = '판서 화면 — 지금';
    modeRows = [
      buildGroupedRow(
        byId,
        [
          { id: 'd-pen', shortLabel: '펜' },
          { id: 'd-marker', shortLabel: '형광펜' },
          { id: 'd-text', shortLabel: '글자' },
        ],
        platform,
      ),
      buildGroupedRow(
        byId,
        [
          { id: 'd-select', shortLabel: '고르기' },
          { id: 'd-eraser', shortLabel: '지우개' },
        ],
        platform,
      ),
      buildGroupedRow(byId, [{ id: 'd-clear', shortLabel: '모두 지우기' }], platform),
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
        { fixedLabel: '색' },
      ),
      buildGroupedRow(
        byId,
        [
          { id: 'd-white', shortLabel: '흰' },
          { id: 'd-black', shortLabel: '검은 칠판' },
        ],
        platform,
      ),
      buildGroupedRow(
        byId,
        [
          { id: 'd-undo', shortLabel: '되돌리기' },
          { id: 'd-redo', shortLabel: '다시' },
        ],
        platform,
      ),
      { label: '확대 · 굵기', keys: ['휠', `${modCtrl} 휠`] },
      buildGroupedRow(byId, [{ id: 'd-exit', shortLabel: '끝내기 · 보관' }], platform),
    ].filter(Boolean);

    extraTitle = '끌어서 도형';
    const centerRow = buildGroupedRow(byId, [{ id: 'd-center', shortLabel: '커서 가운데로' }], platform);
    extraRows = [
      { label: '직선', keys: [modShift] },
      { label: '사각형', keys: [modCtrl] },
      { label: '화살표', keys: [modCtrl, modShift] },
      { label: '원', keys: ['Tab'] },
      { label: '점선으로', keys: [`+ ${modAlt}`] },
      { label: '멈추면 도형', note: '0.5초', noteMono: true },
      { label: '확대 중 이동', note: '오른쪽 클릭' },
      ...(centerRow ? [centerRow] : []),
    ];
  }

  return [
    { id: 'global', title: '어디서나', active: false, rows: globalRows },
    { id: `mode-${mode}`, title: modeTitle, active: true, rows: modeRows },
    { id: `extra-${mode}`, title: extraTitle, active: false, rows: extraRows },
  ];
}
