import test from 'node:test';
import assert from 'node:assert/strict';
import { buildCheatSheetColumns } from './cheatSheet.js';

const SAMPLE_WIN = [
  { id: 'region', scope: 'global', group: 'capture', label: '범위 캡처', combo: 'F1' },
  { id: 'window', scope: 'global', group: 'capture', label: '창 캡처', combo: 'F2' },
  { id: 'record', scope: 'global', group: 'capture', label: '화면 녹화 시작·중지', combo: 'F3' },
  { id: 'draw', scope: 'global', group: 'tools', label: '판서', combo: 'Alt+1' },
  { id: 'zoom', scope: 'global', group: 'tools', label: '실시간 확대', combo: 'Alt+2' },
  { id: 'board', scope: 'global', group: 'tools', label: '칠판', combo: 'Alt+3' },
  { id: 'topmost', scope: 'global', group: 'tools', label: '창 항상 위', combo: 'Alt+Shift+D' },
  { id: 'pause', scope: 'global', group: 'quick', label: '단축키 켜기·끄기', combo: 'Alt+`' },
  { id: 'd-pen', scope: 'draw', group: 'draw-mode', label: '펜', combo: 'P' },
  { id: 'd-marker', scope: 'draw', group: 'draw-mode', label: '형광펜', combo: '' },
  { id: 'd-text', scope: 'draw', group: 'draw-mode', label: '글자', combo: 'T' },
  { id: 'd-undo', scope: 'draw', group: 'draw-mode', label: '되돌리기', combo: 'Ctrl+Z' },
  { id: 'd-redo', scope: 'draw', group: 'draw-mode', label: '다시 하기', combo: 'Ctrl+Y' },
  { id: 'd-center', scope: 'draw', group: 'draw-mode', label: '커서를 화면 가운데로', combo: 'Space' },
  { id: 'd-exit', scope: 'draw', group: 'draw-mode', label: '판서 끝내기', combo: 'Esc' },
];

test('builds 3 columns from live shortcuts and skips cleared slots', () => {
  const cols = buildCheatSheetColumns(SAMPLE_WIN, { platform: 'windows', mode: 'draw' });
  assert.equal(cols.length, 3);
  assert.equal(cols[0].rows.length, 8);
  assert.deepEqual(cols[0].rows[6], { label: '창 항상 위', keys: ['Alt', 'Shift', 'D'] });

  // d-marker is empty so it is omitted from the first draw row
  assert.equal(cols[1].active, true);
  assert.deepEqual(cols[1].rows[0], { label: '펜 · 글자', keys: ['P', 'T'] });
  // Ctrl+Z and Ctrl+Y share Ctrl prefix
  assert.deepEqual(cols[1].rows[1], { label: '되돌리기 · 다시', keys: ['Ctrl', 'Z', 'Y'] });
  assert.deepEqual(cols[2].rows.at(-1), { label: '커서 가운데로', keys: ['Space'] });
});

test('formats macOS symbols and omits cleared global shortcuts', () => {
  const macShortcuts = [
    { id: 'region', scope: 'global', group: 'capture', label: '범위 캡처', combo: '' },
    { id: 'draw', scope: 'global', group: 'tools', label: '판서', combo: 'Option+1' },
    { id: 'pause', scope: 'global', group: 'quick', label: '단축키 켜기·끄기', combo: 'Option+`' },
    { id: 'd-undo', scope: 'draw', group: 'draw-mode', label: '되돌리기', combo: 'Cmd+Z' },
    { id: 'd-redo', scope: 'draw', group: 'draw-mode', label: '다시 하기', combo: 'Shift+Cmd+Z' },
    { id: 'd-center', scope: 'draw', group: 'draw-mode', label: '커서를 화면 가운데로', combo: '' },
  ];
  const cols = buildCheatSheetColumns(macShortcuts, { platform: 'macos', mode: 'draw' });
  assert.deepEqual(cols[0].rows, [
    { label: '판서', keys: ['⌥', '1'] },
    { label: '단축키 켜기·끄기', keys: ['⌥', '`'] },
  ]);
  assert.deepEqual(cols[1].rows[0], { label: '되돌리기 · 다시', keys: ['⌘', 'Z', '⇧', '⌘', 'Z'] });
  assert.deepEqual(cols[2].rows[0], { label: '직선', keys: ['⇧'] });
  assert.deepEqual(cols[2].rows[1], { label: '사각형', keys: ['⌘'] });
  assert.equal(cols[2].rows.some((r) => r.label === '커서 가운데로'), false);
});
