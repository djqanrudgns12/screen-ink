import test from 'node:test';
import assert from 'node:assert/strict';
import {
  describeSettingChange,
  formatFirstRunConflict,
  formatLocationSub,
  formatSettingValue,
  formatShapeOnHoldSub,
} from './settingsMeta.js';

test('formats setting values into short Korean and English labels', () => {
  assert.equal(formatSettingValue('general.theme', 'dark'), '어둡게');
  assert.equal(formatSettingValue('general.theme', 'system'), '기기 설정');
  assert.equal(formatSettingValue('general.theme', 'dark', 'en'), 'Dark');
  assert.equal(formatSettingValue('general.theme', 'system', 'en'), 'System');
  assert.equal(formatSettingValue('draw.width', 8), '8px');
  assert.equal(formatSettingValue('draw.shapeApply', 'suggest'), '묻고 바꾸기');
  assert.equal(formatSettingValue('draw.shapeApply', 'auto'), '바로 바꾸기');
  assert.equal(formatSettingValue('draw.shapeApply', 'suggest', 'en'), 'Ask first');
  assert.equal(formatSettingValue('record.fps', 60), '60fps');
  assert.equal(formatSettingValue('draw.startZoom', 1.75), '×1.75');
  assert.equal(formatSettingValue('record.micDevice', ''), '기본 장치');
  assert.equal(formatSettingValue('record.micDevice', '', 'en'), 'System default');
  assert.equal(formatSettingValue('record.micDevice', 'USB 마이크'), 'USB 마이크');
  assert.equal(formatSettingValue('topmost.border', 0), '끔');
  assert.equal(formatSettingValue('topmost.border', 4), '4px');
  assert.equal(formatSettingValue('capture.cursor', true), '켬');
  assert.equal(formatSettingValue('capture.cursor', false, 'en'), 'Off');
});

test('describes setting and shortcut import changes', () => {
  const shortcuts = [{ id: 'active', label: '지금 창 바로 찍기' }];
  const sChange = describeSettingChange(
    { path: 'shortcuts.active', before: 'Shift+F2', after: 'Ctrl+Alt+F2' },
    shortcuts,
  );
  assert.equal(sChange.label, '지금 창 바로 찍기');
  assert.equal(sChange.isShortcut, true);
  assert.equal(sChange.beforeCombo, 'Shift+F2');
  assert.equal(sChange.afterCombo, 'Ctrl+Alt+F2');

  const vChange = describeSettingChange(
    { path: 'draw.width', before: 4, after: 8 },
    shortcuts,
  );
  assert.equal(vChange.label, '굵기');
  assert.equal(vChange.isShortcut, false);
  assert.equal(vChange.beforeText, '4px');
  assert.equal(vChange.afterText, '8px');

  const enChange = describeSettingChange(
    { path: 'capture.magnet', before: true, after: false },
    shortcuts,
    'en',
  );
  assert.equal(enChange.label, 'Snap to edges');
  assert.equal(enChange.beforeText, 'On');
  assert.equal(enChange.afterText, 'Off');
});

test('formats settings location with store/portable/installed/app-bundle prefix and shortened path', () => {
  assert.equal(
    formatLocationSub({ path: 'C:\\Users\\Teacher\\AppData\\Roaming\\screen-ink\\settings.json', portable: false }, 26, undefined, 'windows', 'ko'),
    '설치판 · …\\screen-ink\\settings.json',
  );
  assert.equal(
    formatLocationSub({ path: '/Users/teacher/Library/Application Support/Shookshok/settings.json', portable: false }, 26, undefined, 'macos', 'ko'),
    '앱 번들 · …/Shookshok/settings.json',
  );
  assert.equal(
    formatLocationSub({ path: 'D:\\screen-ink\\data\\settings.json', portable: true }, 40, undefined, 'windows', 'ko'),
    '무설치판 · D:\\screen-ink\\data\\settings.json',
  );
  assert.equal(
    formatLocationSub(
      { path: 'C:\\Users\\Teacher\\AppData\\Roaming\\Shookshok\\settings.json', portable: false },
      26,
      'msix-store',
    ),
    '스토어 앱 · …\\Shookshok\\settings.json',
  );
  assert.equal(
    formatLocationSub(
      { path: 'C:\\Users\\Teacher\\AppData\\Roaming\\Shookshok\\settings.json', portable: true },
      26,
      'msix-direct',
      'windows',
      'en',
    ),
    'Store app · …\\Shookshok\\settings.json',
  );
});

test('formats first-run conflict notice for Snipaste, PowerToys, and generic conflicts', () => {
  assert.equal(formatFirstRunConflict({ count: 0, combos: [] }), null);
  assert.deepEqual(
    formatFirstRunConflict({ count: 4, app: 'Snipaste', combos: ['F1', 'F3', 'Shift+F1', 'Ctrl+F1'] }),
    { prefix: '', app: 'Snipaste', suffix: '에서 F1·F3 등 4개를 쓰고 있어요', canHowto: true },
  );
  assert.deepEqual(
    formatFirstRunConflict({ count: 1, app: 'PowerToys 항상 위', combos: ['Alt+Shift+D'] }, 'windows'),
    { prefix: '', app: 'PowerToys', suffix: "에서 '항상 위'를 끄면 Alt Shift D 사용 가능", canHowto: true },
  );
  assert.deepEqual(
    formatFirstRunConflict({ count: 2, combos: ['F1', 'F2'] }),
    { prefix: '', app: '', suffix: '단축키 2개가 겹치고 있어요', canHowto: false },
  );
  assert.deepEqual(
    formatFirstRunConflict({ count: 1, app: '카카오톡', combos: ['Alt+1'] }, 'windows'),
    { prefix: '', app: '카카오톡', suffix: '에서 Alt 1 키를 쓰고 있어요', canHowto: true },
  );
  assert.deepEqual(
    formatFirstRunConflict({ count: 1, app: 'Slack', combos: ['Option+1'] }, 'macos', 'en'),
    { prefix: '', app: 'Slack', suffix: ' is using ⌥1', canHowto: true },
  );
});

test('formats shapeOnHold subtitle according to draw.shapeApply mode and OS', () => {
  assert.equal(formatShapeOnHoldSub('suggest', 'windows', 'ko'), '0.5초 멈추면 바꿀지 물어봐요');
  assert.equal(formatShapeOnHoldSub(undefined, 'macos', 'en'), 'Hold 0.5s to ask first');
  assert.equal(formatShapeOnHoldSub('auto', 'windows', 'ko'), '0.5초 · Ctrl Z로 원래 선');
  assert.equal(formatShapeOnHoldSub('auto', 'macos', 'ko'), '0.5초 · ⌘Z로 원래 선');
  assert.equal(formatShapeOnHoldSub('auto', 'macos', 'en'), '0.5s · ⌘Z to revert');
});

test('formats update.auto and annotate.font metadata labels and values', () => {
  const upd = describeSettingChange({ path: 'update.auto', before: true, after: false }, [], 'ko');
  assert.equal(upd.label, '자동으로 새 판 확인');
  assert.equal(upd.beforeText, '켬');
  assert.equal(upd.afterText, '끔');

  const fontChange = describeSettingChange(
    { path: 'annotate.font', before: '', after: JSON.stringify({ f: 'Paperlogy', w: 600 }) },
    [],
    'ko',
  );
  assert.equal(fontChange.label, '주석 글꼴');
  assert.equal(fontChange.beforeText, '비움');
  assert.equal(fontChange.afterText, 'Paperlogy SB');
});
