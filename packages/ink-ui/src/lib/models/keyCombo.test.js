import test from 'node:test';
import assert from 'node:assert/strict';
import { eventToCombo, isEscapeOnly, isClearOnly } from './keyCombo.js';

test('returns null when only modifier keys are pressed', () => {
  assert.equal(eventToCombo({ key: 'Control', code: 'ControlLeft', ctrlKey: true }), null);
  assert.equal(eventToCombo({ key: 'Alt', code: 'AltLeft', altKey: true }), null);
  assert.equal(eventToCombo({ key: 'Shift', code: 'ShiftRight', shiftKey: true }), null);
  assert.equal(eventToCombo({ key: 'Meta', code: 'MetaLeft', metaKey: true }), null);
});

test('orders modifiers as Ctrl, Alt, Shift, Win on windows and Ctrl, Option, Shift, Cmd on macos', () => {
  const e = { key: 'd', code: 'KeyD', ctrlKey: true, altKey: true, shiftKey: true, metaKey: true };
  assert.equal(eventToCombo(e, 'windows'), 'Ctrl+Alt+Shift+Win+D');
  assert.equal(eventToCombo(e, 'macos'), 'Ctrl+Option+Shift+Cmd+D');
});

test('uses physical code for letters and digits even under Korean IME', () => {
  assert.equal(
    eventToCombo({ key: 'ㅁ', code: 'KeyA', ctrlKey: true }, 'windows'),
    'Ctrl+A',
  );
  assert.equal(
    eventToCombo({ key: 'Process', code: 'KeyD', altKey: true, shiftKey: true }, 'windows'),
    'Alt+Shift+D',
  );
  assert.equal(
    eventToCombo({ key: '!', code: 'Digit1', altKey: true, shiftKey: true }, 'macos'),
    'Option+Shift+1',
  );
});

test('maps Backquote, Escape, F1-F24, arrows, Enter, Tab, Space', () => {
  assert.equal(eventToCombo({ key: '₩', code: 'Backquote', altKey: true }, 'windows'), 'Alt+`');
  assert.equal(eventToCombo({ key: 'Escape', code: 'Escape' }), 'Esc');
  assert.equal(eventToCombo({ key: 'F1', code: 'F1' }), 'F1');
  assert.equal(eventToCombo({ key: 'F24', code: 'F24', shiftKey: true }), 'Shift+F24');
  assert.equal(eventToCombo({ key: 'ArrowUp', code: 'ArrowUp', ctrlKey: true }), 'Ctrl+Up');
  assert.equal(eventToCombo({ key: 'ArrowDown', code: 'ArrowDown' }), 'Down');
  assert.equal(eventToCombo({ key: 'ArrowLeft', code: 'ArrowLeft' }), 'Left');
  assert.equal(eventToCombo({ key: 'ArrowRight', code: 'ArrowRight' }), 'Right');
  assert.equal(eventToCombo({ key: 'Enter', code: 'Enter' }), 'Enter');
  assert.equal(eventToCombo({ key: 'Tab', code: 'Tab' }), 'Tab');
  assert.equal(eventToCombo({ key: ' ', code: 'Space' }), 'Space');
  assert.equal(eventToCombo({ key: '?', code: 'Slash', shiftKey: true }), 'Shift+/');
});

test('falls back to key when code is omitted', () => {
  assert.equal(eventToCombo({ key: 'k', ctrlKey: true }, 'macos'), 'Ctrl+K');
  assert.equal(eventToCombo({ key: 'Process' }), null);
});

test('detects Escape-only and Backspace-only helpers', () => {
  assert.equal(isEscapeOnly({ key: 'Escape', code: 'Escape' }), true);
  assert.equal(isEscapeOnly({ key: 'Escape', code: 'Escape', shiftKey: true }), false);
  assert.equal(isClearOnly({ key: 'Backspace', code: 'Backspace' }), true);
  assert.equal(isClearOnly({ key: 'Backspace', code: 'Backspace', ctrlKey: true }), false);
});
