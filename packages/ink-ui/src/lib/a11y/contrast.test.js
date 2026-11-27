import test from 'node:test';
import assert from 'node:assert/strict';
import {
  CONTRAST_PAIRS,
  checkContrast,
  compositeOver,
  contrastRatio,
  parseColor,
  relativeLuminance,
} from '../../../../../scripts/check-contrast.mjs';

test('WCAG 상대 휘도와 대비비 계산이 정확하다', () => {
  const black = parseColor('#000000');
  const white = parseColor('#FFFFFF');
  assert.ok(black && white);
  assert.equal(relativeLuminance(black), 0);
  assert.equal(relativeLuminance(white), 1);
  assert.equal(Math.round(contrastRatio(white, black) * 100) / 100, 21);
  assert.equal(contrastRatio(white, white), 1);

  const blended = compositeOver([0, 0, 0, 0.5], [255, 255, 255, 1]);
  assert.deepEqual(blended.slice(0, 3).map((v) => Math.round(v)), [128, 128, 128]);
});

test('디자인 토큰 라이트·다크의 모든 글자(4.5:1)와 아이콘·UI(3:1) 쌍이 대비 기준을 통과한다', () => {
  assert.ok(CONTRAST_PAIRS.length > 0);
  const { results, failures } = checkContrast();
  assert.ok(results.length >= CONTRAST_PAIRS.length * 2);
  assert.deepEqual(
    failures.map((f) => `${f.theme} ${f.fg}(${f.fgHex})/${f.bg}(${f.bgHex})=${f.ratio.toFixed(2)}<${f.min}`),
    [],
    '대비 기준 미달 토큰 발견',
  );
});

test('[data-theme=dark]와 @media (prefers-color-scheme: dark)의 색 토큰이 일치한다', () => {
  const { darkRaw, mediaDarkRaw } = checkContrast();
  for (const [key, val] of Object.entries(darkRaw)) {
    assert.equal(mediaDarkRaw[key], val, `prefers-color-scheme: dark 의 ${key} 값이 다릅니다`);
  }
});
