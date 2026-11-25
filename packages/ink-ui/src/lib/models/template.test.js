import test from 'node:test';
import assert from 'node:assert/strict';
import { validateFilenameTemplate, renderFilenamePreview, shortenPath } from './template.js';

test('validates forbidden Windows filename characters', () => {
  assert.equal(validateFilenameTemplate('수업_{yyyy}{MM}{dd}_{n}'), null);
  assert.equal(validateFilenameTemplate('수업:1'), '\\ / : * ? " < > | 는 쓸 수 없어요');
  assert.equal(validateFilenameTemplate('a/b'), '\\ / : * ? " < > | 는 쓸 수 없어요');
  assert.equal(validateFilenameTemplate(''), '파일 이름을 입력하세요');
});

test('renders filename preview with date tokens and sequence', () => {
  const date = new Date(2026, 10, 25, 9, 5, 3);
  assert.equal(renderFilenamePreview('수업_{yyyy}{MM}{dd}_{n}', date, 1), '수업_20261125_001.png');
  assert.equal(renderFilenamePreview('{HH}{mm}{ss}_{n}', date, 12), '090503_012.png');
});

test('shortens long folder paths from the front', () => {
  assert.equal(shortenPath('C:\\Users\\Teacher\\Pictures\\screen-ink', 24), '…\\Pictures\\screen-ink');
  assert.equal(shortenPath('short'), 'short');
});
