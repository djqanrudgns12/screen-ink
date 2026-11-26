import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { FILL_ICON_NAMES, ICONS, ICON_NAMES } from './icons.js';
import { CATALOGS } from '../i18n/core.js';

const SVG_DIR = join(
  dirname(fileURLToPath(import.meta.url)),
  '..',
  '..',
  '..',
  '..',
  '..',
  'docs',
  'plans',
  '2026-11-27',
  'candidates',
  'icons',
  '10-custom-shookshok',
  'svg',
);

const LEGACY_24 = [
  'region', 'window', 'record', 'draw', 'zoom', 'board', 'pin', 'restore',
  'folder', 'shortcuts', 'settings', 'info', 'copy', 'save', 'monitor', 'mic',
  'magnet', 'dark', 'delay', 'eraser', 'undo', 'check', 'warning', 'quit',
];

const ADDED_7 = [
  'eyedropper', 'font', 'slider', 'check-circle', 'warning-triangle', 'external-link', 'help-circle',
];

test('기존 F01 24종 순서와 키가 그대로 유지된다', () => {
  assert.deepEqual(ICON_NAMES.slice(0, 24), LEGACY_24);
});

test('10-custom-shookshok 35종 SVG와 추가 7종·채움 8종이 모두 ICONS에 있다', () => {
  const svgs = readdirSync(SVG_DIR)
    .filter((f) => f.endsWith('.svg'))
    .map((f) => f.replace(/\.svg$/, ''))
    .sort();
  assert.equal(svgs.length, 35);
  for (const name of [...svgs, ...ADDED_7, ...FILL_ICON_NAMES]) {
    assert.ok(ICONS[name], `${name} 아이콘 정의 누락`);
    assert.ok(Array.isArray(ICONS[name].shapes) && ICONS[name].shapes.length > 0, `${name} 도형 누락`);
    assert.ok(typeof ICONS[name].labelKey === 'string' && ICONS[name].labelKey in CATALOGS.ko, `${name} i18n 키 누락`);
  }
});

test('ICON_NAMES의 모든 이름이 ICONS에 있고 중복이 없다', () => {
  const uniq = new Set(ICON_NAMES);
  assert.equal(uniq.size, ICON_NAMES.length);
  for (const name of ICON_NAMES) {
    assert.ok(ICONS[name], `${name} 누락`);
  }
});
