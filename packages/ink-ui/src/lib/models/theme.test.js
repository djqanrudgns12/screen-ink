import test from 'node:test';
import assert from 'node:assert/strict';
import { resolveTheme, applyTheme, watchSystemTheme } from './theme.js';

test('resolveTheme respects explicit light/dark and falls back to system', () => {
  assert.equal(resolveTheme('light', true), 'light');
  assert.equal(resolveTheme('dark', false), 'dark');
  assert.equal(resolveTheme('system', false), 'light');
  assert.equal(resolveTheme('system', true), 'dark');
  assert.equal(resolveTheme(undefined, true), 'dark');
});

test('applyTheme sets data-theme on target element', () => {
  const attrs = {};
  const el = { setAttribute: (k, v) => { attrs[k] = v; } };
  assert.equal(applyTheme(el, 'system', true), 'dark');
  assert.equal(attrs['data-theme'], 'dark');
  assert.equal(applyTheme(el, 'light', true), 'light');
  assert.equal(attrs['data-theme'], 'light');
});

test('watchSystemTheme subscribes and unsubscribes cleanly', () => {
  const listeners = new Set();
  const fakeMatchMedia = () => ({
    matches: false,
    addEventListener: (_ev, fn) => listeners.add(fn),
    removeEventListener: (_ev, fn) => listeners.delete(fn),
  });
  const seen = [];
  const off = watchSystemTheme((v) => seen.push(v), fakeMatchMedia);
  for (const fn of listeners) fn({ matches: true });
  assert.deepEqual(seen, [true]);
  off();
  assert.equal(listeners.size, 0);
});
