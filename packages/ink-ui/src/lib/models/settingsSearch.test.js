import test from 'node:test';
import assert from 'node:assert/strict';
import { searchSettings } from './settingsSearch.js';

const TR_KO = {
  'settings.tab.general': '일반',
  'settings.tab.capture': '캡처·녹화',
  'settings.tab.draw': '그리기·확대',
  'settings.tab.shortcuts': '단축키',
  'settings.tab.help': '사용 설명서',
  'settings.tab.info': '정보',
};

const TR_EN = {
  'settings.tab.general': 'General',
  'settings.tab.capture': 'Capture & recording',
  'settings.tab.draw': 'Draw & zoom',
  'settings.tab.shortcuts': 'Shortcuts',
  'settings.tab.help': 'User guide',
  'settings.tab.info': 'About',
};

test('쉬운 말과 옛 말("판서", "모서리 자석", "클립보드") 모두 해당 설정을 찾는다', () => {
  const drawOld = searchSettings('판서', { lang: 'ko', translate: (k) => TR_KO[k] });
  assert.ok(drawOld.some((r) => r.tab === 'draw'), '판서로 그리기·확대 탭 항목 검색');

  const magnetOld = searchSettings('모서리 자석', { lang: 'ko', translate: (k) => TR_KO[k] });
  assert.equal(magnetOld[0]?.id, 'capture.magnet');

  const clipboardOld = searchSettings('클립보드', { lang: 'ko', translate: (k) => TR_KO[k] });
  assert.equal(clipboardOld[0]?.id, 'capture.result');
});

test('영어 낱말로도 설정을 찾고 영어 모드에서는 영어 제목을 돌려준다', () => {
  const theme = searchSettings('theme', { lang: 'en', translate: (k) => TR_EN[k] });
  assert.equal(theme[0]?.id, 'general.theme');
  assert.equal(theme[0]?.title, 'Theme');
  assert.equal(theme[0]?.tabLabel, 'General');

  const reset = searchSettings('reset', { lang: 'ko', translate: (k) => TR_KO[k] });
  assert.equal(reset[0]?.id, 'general.reset');
});

test('OS 전용 항목은 해당 OS에서만 결과에 나온다', () => {
  const macPerm = searchSettings('화면 기록', { platform: 'macos', translate: (k) => TR_KO[k] });
  assert.ok(macPerm.some((r) => r.id === 'general.checkup'));

  const winPerm = searchSettings('화면 기록', { platform: 'windows', translate: (k) => TR_KO[k] });
  assert.equal(winPerm.some((r) => r.id === 'general.checkup'), false);

  const macTopmost = searchSettings('창 항상 위', { platform: 'macos', translate: (k) => TR_KO[k] });
  assert.equal(macTopmost.some((r) => r.id === 'general.topmost'), false);

  const winTopmost = searchSettings('창 항상 위', { platform: 'windows', translate: (k) => TR_KO[k] });
  assert.ok(winTopmost.some((r) => r.id === 'general.topmost'));
});

test('단축키 이름과 키 조합으로 단축키 탭 줄을 찾을 수 있다', () => {
  const shortcuts = [
    { id: 'region', label: '영역 찍기', combo: 'F1', scope: 'global' },
    { id: 'draw', label: '화면에 그리기', combo: 'Alt+1', scope: 'global' },
  ];
  const res = searchSettings('F1', { shortcuts, translate: (k) => TR_KO[k] });
  assert.equal(res[0]?.id, 'shortcut:region');
  assert.equal(res[0]?.tab, 'shortcuts');
  assert.equal(res[0]?.combo, 'F1');
});

test('빈 검색어나 공백만 있으면 빈 배열을 돌려준다', () => {
  assert.deepEqual(searchSettings(''), []);
  assert.deepEqual(searchSettings('   '), []);
});
