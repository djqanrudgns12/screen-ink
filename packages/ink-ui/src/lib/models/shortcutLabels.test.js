import test from 'node:test';
import assert from 'node:assert/strict';
import { translate } from '../i18n/core.js';
import { slotLabel, withSlotLabels } from './shortcutLabels.js';
import { searchSettings } from './settingsSearch.js';

const ko = (/** @type {string} */ k) => translate('ko', k);
const en = (/** @type {string} */ k) => translate('en', k);
const RAW = [
  { id: 'draw', label: '판서', combo: 'Alt+1' },
  { id: 'zoom', label: '실시간 확대', combo: 'Alt+2' },
];

test('단축키 이름은 화면 이름(쉬운 말)으로, 모르는 칸은 원래 이름', () => {
  assert.equal(slotLabel(RAW[0], ko), '그리기');
  assert.equal(slotLabel(RAW[1], ko), '확대');
  assert.equal(slotLabel(RAW[1], en), 'Zoom');
  assert.equal(slotLabel({ id: 'nope', label: '그대로' }, ko), '그대로');
});

test('설정 찾기에서 옛 이름으로 찾아도 결과에는 지금 이름만(한국어·영어)', () => {
  for (const [lang, t] of /** @type {const} */ ([['ko', ko], ['en', en]])) {
    const results = searchSettings('실시간 확대', { lang, platform: 'macos', shortcuts: withSlotLabels(RAW, t), translate: t, limit: 8 });
    assert.ok(results.length > 0, `${lang}: 결과 있음`);
    for (const r of results) {
      assert.ok(!/판서|실시간 확대/.test(r.title), `${lang}: 옛 이름 결과 ${r.title}`);
    }
  }
});
