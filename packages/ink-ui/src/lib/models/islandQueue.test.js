import test from 'node:test';
import assert from 'node:assert/strict';
import {
  createIslandState,
  pushIsland,
  dismissIsland,
  clearModeIsland,
  setIslandHover,
  setHelpOpen,
  shouldShowHintLine,
  getAutoDismissMs,
} from './islandQueue.js';

test('shows only one island at a time and replaces previous transient island', () => {
  let s = createIslandState();
  s = pushIsland(s, { kind: 'result', text: '복사했어요' }, { now: 1000 });
  assert.equal(s.current.text, '복사했어요');
  assert.equal(s.expiresAt, 2200);

  s = pushIsland(s, { kind: 'info', text: '저장했어요' }, { now: 1500 });
  assert.equal(s.current.text, '저장했어요');
  assert.equal(s.expiresAt, 2700);
});

test('auto-dismiss durations match spec including reduced motion and hover pause', () => {
  assert.equal(getAutoDismissMs({ kind: 'result' }), 1200);
  assert.equal(getAutoDismissMs({ kind: 'result' }, { reducedMotion: true }), 2000);
  assert.equal(getAutoDismissMs({ kind: 'problem' }), 6000);
  assert.equal(getAutoDismissMs({ kind: 'off' }), 2000);
  assert.equal(getAutoDismissMs({ kind: 'mode' }), null);

  let s = createIslandState();
  s = pushIsland(s, { kind: 'result', text: '복사했어요' }, { now: 100 });
  s = setIslandHover(s, true, { now: 500 });
  assert.equal(s.hovering, true);
  assert.equal(s.expiresAt, null);

  s = setIslandHover(s, false, { now: 2000 });
  assert.equal(s.hovering, false);
  assert.equal(s.expiresAt, 3200);
});

test('result overlays active mode for 0.8s and restores mode on dismiss', () => {
  let s = createIslandState();
  const mode = { kind: 'mode', modeKey: 'region', text: '범위 캡처', hints: [{ key: 'Esc', label: '취소' }] };
  s = pushIsland(s, mode, { now: 1000 });
  assert.equal(s.current, mode);
  assert.equal(s.expiresAt, null);

  s = pushIsland(s, { kind: 'result', text: '색 복사' }, { now: 1500 });
  assert.equal(s.current.text, '색 복사');
  assert.equal(s.expiresAt, 2300);

  s = dismissIsland(s);
  assert.equal(s.current, mode);
  assert.equal(s.expiresAt, null);

  s = clearModeIsland(s);
  assert.equal(s.current, null);
  assert.equal(s.activeMode, null);
});

test('shows second hint line only for first 3 mode uses or when ? help is open', () => {
  let s = createIslandState();
  const mode = { kind: 'mode', modeKey: 'region', text: '범위 캡처', hints: [{ key: 'Tab', label: '창' }] };

  for (let i = 1; i <= 3; i++) {
    s = pushIsland(s, mode, { now: i * 100 });
    assert.equal(shouldShowHintLine(s), true, `use ${i} should show hints`);
    s = clearModeIsland(s);
  }

  // 4th use hides hints by default
  s = pushIsland(s, mode, { now: 400 });
  assert.equal(shouldShowHintLine(s), false);

  // Opening '?' shows hints again
  s = setHelpOpen(s, true);
  assert.equal(shouldShowHintLine(s), true);
  s = setHelpOpen(s, false);
  assert.equal(shouldShowHintLine(s), false);
});
