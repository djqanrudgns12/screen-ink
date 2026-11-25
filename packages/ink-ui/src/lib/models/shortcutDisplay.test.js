import test from 'node:test';
import assert from 'node:assert/strict';
import {
  formatComboReadable,
  formatConflictBanner,
  getShortcutRowDisplay,
  subjectParticle,
} from './shortcutDisplay.js';

test('selects Korean subject particle 이/가 for Hangul endings and 가 for English', () => {
  assert.equal(subjectParticle('Snipaste'), '가');
  assert.equal(subjectParticle('PowerToys'), '가');
  assert.equal(subjectParticle('PowerToys 항상 위'), '가');
  assert.equal(subjectParticle('카카오톡'), '이');
  assert.equal(subjectParticle('반디캠'), '이');
  assert.equal(subjectParticle('탐색기'), '가');
  assert.equal(subjectParticle('메모장 '), '이');
});

test('formats readable combos for Windows and macOS', () => {
  assert.equal(formatComboReadable('Alt+Shift+D', 'windows'), 'Alt Shift D');
  assert.equal(formatComboReadable('Option+Shift+D', 'macos'), '⌥⇧D');
  assert.equal(formatComboReadable('Cmd+1', 'macos'), '⌘1');
  assert.equal(formatComboReadable('F1', 'windows'), 'F1');
});

test('formats conflict banner for F-key sets, PowerToys single/multiple combos, macOS, and unknown app', () => {
  assert.equal(formatConflictBanner({ count: 0, combos: [] }), null);

  // F04 Snipaste F-key group
  assert.deepEqual(
    formatConflictBanner(
      { count: 4, app: 'Snipaste', combos: ['F1', 'F3', 'Shift+F1', 'Ctrl+F1'] },
      'windows',
    ),
    { lead: 'Snipaste', message: '가 F1·F3 키 4개를 먼저 쓰고 있어요' },
  );

  // Single F-key
  assert.deepEqual(
    formatConflictBanner({ count: 1, app: 'Snipaste', combos: ['F1'] }, 'windows'),
    { lead: 'Snipaste', message: '가 F1 키를 먼저 쓰고 있어요' },
  );

  // PowerToys Always On Top (Alt+Shift+D) on Windows
  assert.deepEqual(
    formatConflictBanner(
      { count: 1, app: 'PowerToys 항상 위', combos: ['Alt+Shift+D'] },
      'windows',
    ),
    { lead: 'PowerToys 항상 위', message: '가 Alt Shift D 키를 먼저 쓰고 있어요' },
  );

  // Non-F-key multiple combos (up to 2 + 외 n개) on Windows and macOS
  assert.deepEqual(
    formatConflictBanner(
      { count: 3, app: '카카오톡', combos: ['Alt+Shift+D', 'Alt+1', 'Alt+2'] },
      'windows',
    ),
    { lead: '카카오톡', message: '이 Alt Shift D · Alt 1 외 1개를 먼저 쓰고 있어요' },
  );
  assert.deepEqual(
    formatConflictBanner(
      { count: 3, app: '카카오톡', combos: ['Option+Shift+D', 'Option+1', 'Option+2'] },
      'macos',
    ),
    { lead: '카카오톡', message: '이 ⌥⇧D · ⌥1 외 1개를 먼저 쓰고 있어요' },
  );

  // Two non-F-key combos
  assert.deepEqual(
    formatConflictBanner(
      { count: 2, app: 'PowerToys 항상 위', combos: ['Alt+Shift+D', 'F1'] },
      'windows',
    ),
    { lead: 'PowerToys 항상 위', message: '가 Alt Shift D · F1 키를 먼저 쓰고 있어요' },
  );

  // Unknown/multiple apps
  assert.deepEqual(
    formatConflictBanner({ count: 2, combos: ['F1', 'Alt+Shift+D'] }, 'windows'),
    { lead: '', message: '단축키 2개가 겹치고 있어요' },
  );
});

test('computes calm row display for warn vs conflict vs recording rows', () => {
  const warnRow = { status: 'warn', reason: '도움말(거의 모든 앱)' };

  // Idle warn row: no dot, no bottom warning, reason only in hoverSub
  assert.deepEqual(getShortcutRowDisplay(warnRow, { isRecording: false, bannerApp: 'Snipaste' }), {
    dotStatus: '',
    sub: '',
    hoverSub: '도움말(거의 모든 앱)',
    note: '',
    error: '',
    warning: '',
    suggestions: [],
  });

  // Recording warn row: recording prompt in sub, reason in note (--ink-sub)
  assert.deepEqual(getShortcutRowDisplay(warnRow, { isRecording: true, bannerApp: 'Snipaste' }), {
    dotStatus: '',
    sub: '키를 누르세요 · Esc 취소',
    hoverSub: '',
    note: '도움말(거의 모든 앱)',
    error: '',
    warning: '',
    suggestions: [],
  });

  // Conflict row covered by top banner: dot only, no duplicate bottom text
  const snipConflict = { status: 'conflict', owner: 'Snipaste', reason: 'Snipaste가 먼저 쓰고 있어요' };
  assert.deepEqual(getShortcutRowDisplay(snipConflict, { isRecording: false, bannerApp: 'Snipaste' }), {
    dotStatus: 'attn',
    sub: '',
    hoverSub: '',
    note: '',
    error: '',
    warning: '',
    suggestions: [],
  });

  // Conflict row NOT covered by single-app banner: dot + bottom reason
  assert.deepEqual(getShortcutRowDisplay(snipConflict, { isRecording: false, bannerApp: '' }), {
    dotStatus: 'attn',
    sub: '',
    hoverSub: '',
    note: '',
    error: '',
    warning: 'Snipaste가 먼저 쓰고 있어요',
    suggestions: [],
  });

  // Recording error feedback takes precedence over warn hover/note
  assert.deepEqual(
    getShortcutRowDisplay(warnRow, {
      isRecording: true,
      feedback: { error: '이미 쓰고 있어요', suggestions: ['Ctrl+Alt+1'] },
    }),
    {
      dotStatus: '',
      sub: '키를 누르세요 · Esc 취소',
      hoverSub: '',
      note: '',
      error: '이미 쓰고 있어요',
      warning: '',
      suggestions: ['Ctrl+Alt+1'],
    },
  );
});
