import { test } from 'node:test';
import assert from 'node:assert/strict';
import { CATALOGS, LANGS, normalizeLang, normalizeOs, placeholders, translate } from './core.js';

test('두 언어의 키가 같고 값 끼우기 이름도 같다', () => {
  const ko = Object.keys(CATALOGS.ko).sort();
  for (const l of LANGS) {
    assert.deepEqual(Object.keys(CATALOGS[l]).sort(), ko, `${l} 키 집합`);
    for (const k of ko) assert.deepEqual(placeholders(CATALOGS[l][k]), placeholders(CATALOGS.ko[k]), `${l}:${k}`);
  }
});

test('빈 글이 없고 키는 점 표기', () => {
  for (const l of LANGS) {
    for (const [k, v] of Object.entries(CATALOGS[l])) {
      assert.ok(v.trim().length > 0, `${l}:${k} 비어 있음`);
      assert.match(k, /^[a-z][a-zA-Z0-9]*(\.[a-zA-Z0-9]+)+(~mac|~win)?$/, k);
      if (k.includes('~')) assert.ok(k.split('~')[0] in CATALOGS[l], `${l}:${k} — OS 변형은 기본 키도 있어야 해요(다른 OS의 대신 글)`);
    }
  }
});

test('번역·값 끼우기·대신하기', () => {
  assert.equal(translate('en', 'settings.tab.general'), 'General');
  assert.equal(translate('ko', 'island.hotkeys.onSome', { n: 2 }), '단축키를 켰어요 · 2개는 다른 앱이 쓰고 있어요');
  assert.equal(translate('en-US', 'island.hotkeys.onSome', { n: 2 }), 'Shortcuts are on · 2 are used by other apps');
  assert.equal(translate('en', 'no.such.key'), 'no.such.key', '없는 키는 키 그대로');
  assert.equal(translate('ko', 'island.hotkeys.onSome'), '단축키를 켰어요 · {n}개는 다른 앱이 쓰고 있어요', '값이 없으면 자리 표시 그대로');
  assert.equal(normalizeLang('EN'), 'en');
  assert.equal(normalizeLang(undefined), 'ko');
  assert.equal(normalizeOs('macos'), 'mac');
  assert.equal(normalizeOs('windows'), 'win');
});

test('OS 변형이 있으면 그 OS에서 먼저 쓴다', () => {
  const c = { ...CATALOGS.ko };
  CATALOGS.ko['zz.test.paste'] = '붙여 넣기';
  CATALOGS.ko['zz.test.paste~mac'] = '⌘V로 붙여 넣기';
  try {
    assert.equal(translate('ko', 'zz.test.paste', undefined, 'macos'), '⌘V로 붙여 넣기');
    assert.equal(translate('ko', 'zz.test.paste', undefined, 'windows'), '붙여 넣기');
    assert.equal(translate('en', 'zz.test.paste', undefined, 'mac'), '⌘V로 붙여 넣기', '영어가 없으면 한국어 변형');
    assert.equal(translate('ko', 'zz.test.paste'), '붙여 넣기', 'OS를 안 주면 기본');
  } finally {
    for (const k of Object.keys(CATALOGS.ko)) if (!(k in c)) delete CATALOGS.ko[k];
  }
});

test('합친 카탈로그가 영역 파일과 같다(node scripts/i18n-build.mjs를 돌렸는지)', async () => {
  const { buildCatalogs, serialize } = await import('../../../../../scripts/i18n-build.mjs');
  const { out, errors } = buildCatalogs();
  assert.deepEqual(errors, []);
  for (const l of LANGS) assert.equal(serialize(out[l]), serialize(CATALOGS[l]), `${l}.json을 다시 합치세요`);
});
