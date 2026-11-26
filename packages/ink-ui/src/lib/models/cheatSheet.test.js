import test from 'node:test';
import assert from 'node:assert/strict';
import { buildCheatSheetColumns } from './cheatSheet.js';

const SAMPLE_WIN = [
  { id: 'region', scope: 'global', group: 'capture', label: '영역 찍기', combo: 'F1' },
  { id: 'window', scope: 'global', group: 'capture', label: '창 찍기', combo: 'F2' },
  { id: 'record', scope: 'global', group: 'capture', label: '화면 녹화', combo: 'F3' },
  { id: 'draw', scope: 'global', group: 'tools', label: '그리기', combo: 'Alt+1' },
  { id: 'zoom', scope: 'global', group: 'tools', label: '확대', combo: 'Alt+2' },
  { id: 'board', scope: 'global', group: 'tools', label: '칠판', combo: 'Alt+3' },
  { id: 'topmost', scope: 'global', group: 'tools', label: '창을 맨 위에', combo: 'Alt+Shift+D' },
  { id: 'pause', scope: 'global', group: 'quick', label: '단축키 켜기·끄기', combo: 'Alt+`' },
  { id: 'c-save', scope: 'capture', group: 'capture-mode', label: '저장', combo: 'Ctrl+S' },
  { id: 'c-quick', scope: 'capture', group: 'capture-mode', label: '빠른 저장', combo: 'Ctrl+Shift+S' },
  { id: 'd-pen', scope: 'draw', group: 'draw-mode', label: '펜', combo: 'P' },
  { id: 'd-marker', scope: 'draw', group: 'draw-mode', label: '형광펜', combo: '' },
  { id: 'd-text', scope: 'draw', group: 'draw-mode', label: '글자', combo: 'T' },
  { id: 'd-undo', scope: 'draw', group: 'draw-mode', label: '되돌리기', combo: 'Ctrl+Z' },
  { id: 'd-redo', scope: 'draw', group: 'draw-mode', label: '다시 하기', combo: 'Ctrl+Y' },
  { id: 'd-center', scope: 'draw', group: 'draw-mode', label: '커서를 화면 가운데로', combo: 'Space' },
  { id: 'd-exit', scope: 'draw', group: 'draw-mode', label: '그리기 끝내기', combo: 'Esc' },
];

test('현재 단축키로 3열을 만들고 비운 칸은 건너뛴다', () => {
  const cols = buildCheatSheetColumns(SAMPLE_WIN, { platform: 'windows', mode: 'draw', lang: 'ko' });
  assert.equal(cols.length, 3);
  assert.equal(cols[0].title, '언제든');
  assert.equal(cols[0].rows.length, 8);
  assert.deepEqual(cols[0].rows[6], { label: '창을 맨 위에', keys: ['Alt', 'Shift', 'D'] });

  // d-marker가 비었으므로 앞의 펜·글자 키만 모인다
  assert.equal(cols[1].active, true);
  assert.equal(cols[1].title, '그리는 중 — 지금');
  assert.deepEqual(cols[1].rows[0], { label: '펜 · 형광펜 · 글자', keys: ['P', 'T'] });
  // Ctrl+Z와 Ctrl+Y는 공통 수식키 Ctrl을 한 번만 묶는다
  assert.deepEqual(cols[1].rows[1], { label: '되돌리기 · 다시 하기', keys: ['Ctrl', 'Z', 'Y'] });
  // 확대 · 굵기는 두 조합 사이에 '또는' 구분자를 둔다
  assert.deepEqual(cols[1].rows[2], {
    label: '확대 · 굵기',
    keyGroups: [['휠'], ['Ctrl', '휠']],
    separator: '또는',
  });
  // 점선으로는 + 접두사와 ⌥/Alt 키캡을 분리한다
  assert.deepEqual(cols[2].rows[4], { label: '점선으로', keyPrefix: '+', keys: ['Alt'] });
  assert.deepEqual(cols[2].rows.at(-1), { label: '커서 가운데로', keys: ['Space'] });
});

test('macOS 기호와 다중 키 묶음 사이 "또는" 구분자를 적용한다(R2-FIX-7)', () => {
  const macShortcuts = [
    { id: 'region', scope: 'global', group: 'capture', label: '영역 찍기', combo: '' },
    { id: 'draw', scope: 'global', group: 'tools', label: '그리기', combo: 'Option+1' },
    { id: 'pause', scope: 'global', group: 'quick', label: '단축키 켜기·끄기', combo: 'Option+`' },
    { id: 'd-undo', scope: 'draw', group: 'draw-mode', label: '되돌리기', combo: 'Cmd+Z' },
    { id: 'd-redo', scope: 'draw', group: 'draw-mode', label: '다시 하기', combo: 'Shift+Cmd+Z' },
    { id: 'd-center', scope: 'draw', group: 'draw-mode', label: '커서를 화면 가운데로', combo: '' },
  ];
  const cols = buildCheatSheetColumns(macShortcuts, { platform: 'macos', mode: 'draw', lang: 'ko' });
  assert.deepEqual(cols[0].rows, [
    { label: '그리기', keys: ['⌥', '1'] },
    { label: '단축키 켜기·끄기', keys: ['⌥', '`'] },
  ]);
  // ⌘Z 와 ⇧⌘Z 는 5개 키캡이 붙지 않고 keyGroups + separator('또는')로 나뉜다
  assert.deepEqual(cols[1].rows[0], {
    label: '되돌리기 · 다시 하기',
    keyGroups: [
      ['⌘', 'Z'],
      ['⇧', '⌘', 'Z'],
    ],
    separator: '또는',
  });
  assert.deepEqual(cols[2].rows[0], { label: '직선', keys: ['⇧'] });
  assert.deepEqual(cols[2].rows[1], { label: '사각형', keys: ['⌘'] });
  assert.deepEqual(cols[2].rows[4], { label: '점선으로', keyPrefix: '+', keys: ['⌥'] });
  assert.equal(cols[2].rows.some((r) => r.label === '커서 가운데로'), false);
});

test('Windows 캡처 한 장에서 저장 · 빠른 저장을 keyGroups + "또는"으로 구분한다(R2-FIX-7)', () => {
  const cols = buildCheatSheetColumns(SAMPLE_WIN, { platform: 'windows', mode: 'capture', lang: 'ko' });
  const saveRow = cols[1].rows.find((r) => r.label === '저장 · 빠른 저장');
  assert.deepEqual(saveRow, {
    label: '저장 · 빠른 저장',
    keyGroups: [
      ['Ctrl', 'S'],
      ['Ctrl', 'Shift', 'S'],
    ],
    separator: '또는',
  });
});

test('영어 판 한 장은 영어 문구와 or 구분자를 쓰고 한글을 포함하지 않는다', () => {
  const cols = buildCheatSheetColumns(SAMPLE_WIN, { platform: 'windows', mode: 'capture', lang: 'en' });
  assert.equal(cols[0].title, 'Anytime');
  assert.equal(cols[1].title, 'After capture — now');
  assert.equal(cols[2].title, 'Fine-tune area');
  const saveRow = cols[1].rows.find((r) => r.label === 'Save · Quick save');
  assert.equal(saveRow?.separator, 'or');
  const allText = JSON.stringify(cols);
  assert.equal(/[가-힣]/.test(allText), false, `영어 한 장에 한글 남음: ${allText}`);
});

test('확대 한 장: 확대 크기는 휠, 키는 그리기·끝내기만 — 방향키는 아래 앱으로', () => {
  const shortcuts = [
    ...SAMPLE_WIN,
    { id: 'z-exit', scope: 'zoom', group: 'zoom-mode', label: '확대 끝내기', combo: 'Esc' },
  ];
  const cols = buildCheatSheetColumns(shortcuts, { platform: 'windows', mode: 'zoom', lang: 'ko' });
  assert.equal(cols[1].active, true);
  assert.deepEqual(cols[1].rows, [
    { label: '확대 크기 조절', keys: ['휠'] },
    { label: '멈추고 그리기', keys: ['Alt', '1'] },
    { label: '끝내기', keys: ['Esc'] },
  ]);
  assert.equal(cols[2].rows.some((r) => (r.keys || []).includes('방향키') && r.label !== '방향키·클릭'), false);
  assert.deepEqual(cols[2].rows[0], { label: '아래 앱 스크롤', keys: ['Shift', '휠'] });
});
