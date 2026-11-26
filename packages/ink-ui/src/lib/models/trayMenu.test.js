import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  TRAY_POPOVER_WIDTH_PX,
  TRAY_ROW_HEIGHT_PX,
  TRAY_TILES,
  findTypeaheadIndex,
  formatElapsed,
  getHangulChoseong,
  resolveMenuKeyAction,
  resolveTrayStatus,
} from './trayMenu.js';

const HERE = dirname(fileURLToPath(import.meta.url));

test('트레이 치수와 4칸 동작 타일 명세', () => {
  assert.ok(TRAY_POPOVER_WIDTH_PX >= 248 && TRAY_POPOVER_WIDTH_PX <= 260);
  assert.equal(TRAY_ROW_HEIGHT_PX, 28);
  assert.deepEqual(
    TRAY_TILES.map((t) => [t.id, t.icon, t.action]),
    [
      ['region', 'region', 'capture.region'],
      ['window', 'window', 'capture.window'],
      ['full', 'monitor', 'capture.full'],
      ['record', 'record', 'record.toggle'],
    ],
  );
});

test('formatElapsed는 밀리초를 MM:SS로 포맷한다', () => {
  assert.equal(formatElapsed(0), '00:00');
  assert.equal(formatElapsed(-500), '00:00');
  assert.equal(formatElapsed(12_900), '00:12');
  assert.equal(formatElapsed(65_000), '01:05');
});

test('resolveTrayStatus는 녹화 > 맥 권한 > 꺼짐 > 겹침 > 준비됨 우선순위를 따른다', () => {
  assert.deepEqual(
    resolveTrayStatus({
      platform: 'macos',
      hotkeysEnabled: false,
      permissions: { screen: false, accessibility: true },
      conflicts: { count: 2 },
      recording: { active: true, elapsedMs: 12_000 },
    }),
    { kind: 'recording', tone: 'danger', clickable: false, elapsedMs: 12_000 },
  );

  assert.deepEqual(
    resolveTrayStatus({
      platform: 'macos',
      hotkeysEnabled: false,
      permissions: { screen: false, accessibility: true },
      conflicts: { count: 2 },
      recording: { active: false },
    }),
    { kind: 'permission', tone: 'attn', clickable: true, action: 'firstRun.open' },
  );

  // Windows에서는 권한 필드가 있어도 권한 빠짐 상태로 가지 않는다
  assert.deepEqual(
    resolveTrayStatus({
      platform: 'windows',
      hotkeysEnabled: false,
      permissions: { screen: false, accessibility: false },
      conflicts: { count: 2 },
    }),
    { kind: 'paused', tone: 'off', clickable: false },
  );

  assert.deepEqual(
    resolveTrayStatus({
      platform: 'macos',
      hotkeysEnabled: true,
      permissions: { screen: true, accessibility: true },
      conflicts: { count: 3 },
    }),
    { kind: 'conflict', tone: 'attn', clickable: true, action: 'settings.open', count: 3 },
  );

  assert.deepEqual(
    resolveTrayStatus({
      platform: 'macos',
      hotkeysEnabled: true,
      permissions: { screen: true, accessibility: true },
      conflicts: { count: 0 },
    }),
    { kind: 'ready', tone: 'ok', clickable: false },
  );
});

test('키보드 ↑↓·Home·End·Enter·Esc 이동 및 비활성 건너뛰기', () => {
  const items = [
    { label: '영역' },
    { label: '창', disabled: true },
    { label: '전체' },
    { label: '그리기' },
  ];

  assert.deepEqual(resolveMenuKeyAction({ key: 'Escape' }, items, 0), { type: 'close' });
  assert.deepEqual(resolveMenuKeyAction({ key: 'ArrowDown' }, items, -1), { type: 'focus', index: 0 });
  assert.deepEqual(resolveMenuKeyAction({ key: 'ArrowDown' }, items, 0), { type: 'focus', index: 2 });
  assert.deepEqual(resolveMenuKeyAction({ key: 'ArrowDown' }, items, 3), { type: 'focus', index: 0 });
  assert.deepEqual(resolveMenuKeyAction({ key: 'ArrowUp' }, items, 0), { type: 'focus', index: 3 });
  assert.deepEqual(resolveMenuKeyAction({ key: 'ArrowUp' }, items, 2), { type: 'focus', index: 0 });
  assert.deepEqual(resolveMenuKeyAction({ key: 'Home' }, items, 3), { type: 'focus', index: 0 });
  assert.deepEqual(resolveMenuKeyAction({ key: 'End' }, items, 0), { type: 'focus', index: 3 });
  assert.deepEqual(resolveMenuKeyAction({ key: 'Enter' }, items, 2), { type: 'activate', index: 2 });
  assert.deepEqual(resolveMenuKeyAction({ key: 'Enter' }, items, 1), { type: 'noop' });
  assert.deepEqual(resolveMenuKeyAction({ key: 'Enter' }, items, -1), { type: 'noop' });
  assert.deepEqual(resolveMenuKeyAction({ key: 'g', ctrlKey: true }, items, 0), { type: 'noop' });
  assert.deepEqual(resolveMenuKeyAction({ key: '영', isComposing: true }, items, 0), { type: 'noop' });
});

test('첫 글자 이동은 영어 대소문자·순환과 한글 음절·초성을 지원한다', () => {
  const koItems = [
    { label: '영역' },
    { label: '그리기' },
    { label: '칠판' },
    { label: '확대' },
    { label: '설정…' },
    { label: '끝내기' },
  ];
  assert.equal(getHangulChoseong('그'), 'ㄱ');
  assert.equal(getHangulChoseong('A'), null);

  assert.equal(findTypeaheadIndex(koItems, -1, '그'), 1);
  assert.equal(findTypeaheadIndex(koItems, -1, 'ㄱ'), 1);
  assert.equal(findTypeaheadIndex(koItems, 1, 'ㅅ'), 4);
  assert.equal(findTypeaheadIndex(koItems, 4, 'ㅇ'), 0);

  const enItems = [
    { label: 'Region' },
    { label: 'Window' },
    { label: 'Record' },
    { label: 'Settings…' },
  ];
  assert.deepEqual(resolveMenuKeyAction({ key: 'r' }, enItems, -1), { type: 'focus', index: 0 });
  assert.deepEqual(resolveMenuKeyAction({ key: 'R' }, enItems, 0), { type: 'focus', index: 2 });
  assert.deepEqual(resolveMenuKeyAction({ key: 'r' }, enItems, 2), { type: 'focus', index: 0 });
  assert.deepEqual(resolveMenuKeyAction({ key: 's' }, enItems, 0), { type: 'focus', index: 3 });
  assert.deepEqual(resolveMenuKeyAction({ key: 'z' }, enItems, 0), { type: 'noop' });
});

test('tray 카탈로그에 금지 용어가 없다', () => {
  const ko = JSON.parse(readFileSync(join(HERE, '..', 'i18n', 'areas', 'tray.ko.json'), 'utf8'));
  const en = JSON.parse(readFileSync(join(HERE, '..', 'i18n', 'areas', 'tray.en.json'), 'utf8'));
  const bannedKo = ['트레이', '판서', '실시간 확대', '직전', '클립보드', '보안 화면'];
  for (const [k, v] of Object.entries(ko)) {
    for (const word of bannedKo) {
      assert.ok(!v.includes(word), `tray.ko.json[${k}]에 금지어 '${word}' 포함: ${v}`);
    }
  }
  const bannedEn = ['tray', 'overlay', 'clipboard'];
  for (const [k, v] of Object.entries(en)) {
    for (const word of bannedEn) {
      assert.ok(!v.toLowerCase().includes(word), `tray.en.json[${k}]에 금지어 '${word}' 포함: ${v}`);
    }
  }
});
