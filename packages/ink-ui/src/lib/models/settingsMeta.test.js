import test from 'node:test';
import assert from 'node:assert/strict';
import {
  describeSettingChange,
  formatFirstRunConflict,
  formatLocationSub,
  formatSettingValue,
  formatShapeOnHoldSub,
} from './settingsMeta.js';

test('formats setting values into short Korean labels', () => {
  assert.equal(formatSettingValue('general.theme', 'dark'), '다크');
  assert.equal(formatSettingValue('general.theme', 'system'), '시스템 따라');
  assert.equal(formatSettingValue('draw.width', 8), '8px');
  assert.equal(formatSettingValue('draw.shapeApply', 'suggest'), '물어보고 바꾸기');
  assert.equal(formatSettingValue('draw.shapeApply', 'auto'), '바로 바꾸기');
  assert.equal(formatSettingValue('record.fps', 60), '60fps');
  assert.equal(formatSettingValue('draw.startZoom', 1.75), '×1.75');
  assert.equal(formatSettingValue('record.micDevice', ''), '기본 장치');
  assert.equal(formatSettingValue('record.micDevice', 'USB 마이크'), 'USB 마이크');
  assert.equal(formatSettingValue('topmost.border', 0), '끔');
  assert.equal(formatSettingValue('topmost.border', 4), '4px');
  assert.equal(formatSettingValue('capture.cursor', true), '켬');
});

test('describes setting and shortcut import changes', () => {
  const shortcuts = [{ id: 'active', label: '활성 창 바로 캡처' }];
  const sChange = describeSettingChange(
    { path: 'shortcuts.active', before: 'Shift+F2', after: 'Ctrl+Alt+F2' },
    shortcuts,
  );
  assert.equal(sChange.label, '활성 창 바로 캡처');
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
});

test('formats settings location with MSIX/portable/installed prefix and shortened path', () => {
  assert.equal(
    formatLocationSub({ path: 'C:\\Users\\Teacher\\AppData\\Roaming\\screen-ink\\settings.json', portable: false }, 26),
    '설치 · …\\screen-ink\\settings.json',
  );
  assert.equal(
    formatLocationSub({ path: 'D:\\screen-ink\\data\\settings.json', portable: true }, 40),
    '포터블 · D:\\screen-ink\\data\\settings.json',
  );
  assert.equal(
    formatLocationSub(
      { path: 'C:\\Users\\Teacher\\AppData\\Roaming\\Shookshok\\settings.json', portable: false },
      26,
      'msix-store',
    ),
    'MSIX 패키지 · …\\Shookshok\\settings.json',
  );
  assert.equal(
    formatLocationSub(
      { path: 'C:\\Users\\Teacher\\AppData\\Roaming\\Shookshok\\settings.json', portable: true },
      26,
      'msix-direct',
    ),
    'MSIX 패키지 · …\\Shookshok\\settings.json',
  );
});

test('formats first-run conflict notice for Snipaste, PowerToys, and generic conflicts', () => {
  assert.equal(formatFirstRunConflict({ count: 0, combos: [] }), null);
  assert.deepEqual(
    formatFirstRunConflict({ count: 4, app: 'Snipaste', combos: ['F1', 'F3', 'Shift+F1', 'Ctrl+F1'] }),
    { prefix: 'F1·F3 키 4개는 ', app: 'Snipaste', suffix: '가 쓰고 있어요', canHowto: true },
  );
  assert.deepEqual(
    formatFirstRunConflict({ count: 1, app: 'PowerToys 항상 위', combos: ['Alt+Shift+D'] }),
    { prefix: '', app: 'PowerToys', suffix: "에서 '항상 위'를 끄면 Alt+Shift+D를 써요", canHowto: true },
  );
  assert.deepEqual(
    formatFirstRunConflict({ count: 2, combos: ['F1', 'F2'] }),
    { prefix: '', app: '', suffix: '단축키 2개가 겹치고 있어요', canHowto: false },
  );
  assert.deepEqual(
    formatFirstRunConflict({ count: 1, app: '카카오톡', combos: ['Alt+1'] }, 'windows'),
    { prefix: 'Alt 1 키는 ', app: '카카오톡', suffix: '이 쓰고 있어요', canHowto: true },
  );
});

test('formats shapeOnHold subtitle according to draw.shapeApply mode', () => {
  assert.equal(formatShapeOnHoldSub('suggest'), '0.5초 멈추면 바꿀지 물어봐요');
  assert.equal(formatShapeOnHoldSub(undefined), '0.5초 멈추면 바꿀지 물어봐요');
  assert.equal(formatShapeOnHoldSub('auto'), '0.5초 · Ctrl+Z = 원래 획');
});
