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
  formatCapturePath,
  buildCaptureIslandPatch,
  applyCaptureView,
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

test('kind "end" clears active mode island while keeping any transient result island visible', () => {
  let s = createIslandState();
  const mode = {
    kind: 'mode',
    mode: 'region',
    variant: 'region',
    text: '범위 캡처',
    hints: [{ key: 'Esc', label: '취소' }],
  };
  s = pushIsland(s, mode, { now: 100 });
  assert.equal(s.current?.text, '범위 캡처');

  // Cancelling capture emits kind: 'end' -> clears mode island
  s = pushIsland(s, { kind: 'end', text: '' }, { now: 200 });
  assert.equal(s.current, null);
  assert.equal(s.activeMode, null);

  // If a result island is already showing over a mode, kind: 'end' drops activeMode without killing result
  s = pushIsland(s, mode, { now: 300 });
  s = pushIsland(s, { kind: 'result', text: '복사했어요' }, { now: 400 });
  s = pushIsland(s, { kind: 'end', text: '' }, { now: 450 });
  assert.equal(s.activeMode, null);
  assert.equal(s.current?.text, '복사했어요');

  // Dismissing that result now clears completely instead of restoring ended mode
  s = dismissIsland(s);
  assert.equal(s.current, null);
});

test('formats capture recognition path with active level and trims from front when too long', () => {
  assert.deepEqual(formatCapturePath(['창', '표', '2행'], 2), [
    { label: '창', active: false },
    { label: '표', active: false },
    { label: '2행', active: true },
  ]);

  assert.deepEqual(formatCapturePath(['화면', '창', '표', '2행'], 1), [
    { label: '화면', active: false },
    { label: '창', active: true },
    { label: '표', active: false },
    { label: '2행', active: false },
  ]);

  // Long chain trims front segments with leading ellipsis while keeping active tail
  const trimmedFront = formatCapturePath(
    ['가상 화면', '모니터 1', '브라우저 창', '문서 영역', '데이터 표', '2행'],
    5,
    { maxChars: 20, maxItems: 4 },
  );
  assert.equal(trimmedFront[0].label, '…');
  assert.equal(trimmedFront[0].ellipsis, true);
  assert.equal(trimmedFront.at(-1).label, '2행');
  assert.equal(trimmedFront.at(-1).active, true);
  assert.ok(trimmedFront.length <= 4);

  // When active level is at index 0 of a long chain, keeps index 0 and trims trailing segments
  const trimmedBack = formatCapturePath(
    ['가상 화면', '모니터 1', '브라우저 창', '문서 영역', '데이터 표', '2행'],
    0,
    { maxChars: 20, maxItems: 4 },
  );
  assert.equal(trimmedBack[0].label, '가상 화면');
  assert.equal(trimmedBack[0].active, true);
  assert.equal(trimmedBack.at(-1).label, '…');

  // Drag / custom rect (level = -1) or empty path yields no breadcrumb
  assert.deepEqual(formatCapturePath(['화면', '창'], -1), []);
  assert.deepEqual(formatCapturePath([], 0), []);
});

test('updates active mode island in-place from CaptureView without incrementing seq or flickering', () => {
  let s = createIslandState();
  const mode = {
    kind: 'mode',
    mode: 'region',
    variant: 'region',
    text: '범위 캡처',
    hints: [{ key: '휠', label: '넓히기·좁히기' }],
  };

  // Simulate 4th use so hint line is hidden by default
  for (let i = 0; i < 4; i += 1) {
    s = pushIsland(s, mode, { now: i * 100 });
    if (i < 3) s = clearModeIsland(s);
  }
  assert.equal(shouldShowHintLine(s), false);
  const seqBefore = s.seq;
  const usesBefore = s.modeUses.region;

  // Apply capture view -> updates path/level/value/tags/help in place without touching seq or modeUses
  const patch = buildCaptureIslandPatch({
    session: 1,
    phase: 'hover',
    rect: { x: 100, y: 200, w: 718, h: 40 },
    path: ['화면', '창', '표', '2행'],
    level: 3,
    guides: [],
    magnet: false,
    cursorIncluded: true,
    help: true,
  });
  assert.deepEqual(patch, {
    path: ['화면', '창', '표', '2행'],
    level: 3,
    value: '718×40',
    tags: ['자석 끔', '커서 포함'],
    help: true,
  });

  s = applyCaptureView(s, {
    session: 1,
    phase: 'hover',
    rect: { x: 100, y: 200, w: 718, h: 40 },
    path: ['화면', '창', '표', '2행'],
    level: 3,
    guides: [],
    magnet: false,
    cursorIncluded: true,
    help: true,
  });
  assert.equal(s.seq, seqBefore);
  assert.equal(s.modeUses.region, usesBefore);
  assert.equal(s.current.value, '718×40');
  assert.deepEqual(s.current.tags, ['자석 끔', '커서 포함']);
  assert.equal(s.current.level, 3);
  assert.equal(shouldShowHintLine(s), true);

  // If a transient result island is on top, applyCaptureView updates activeMode in background
  s = pushIsland(s, { kind: 'result', text: '색 복사' }, { now: 500 });
  const seqWithResult = s.seq;
  s = applyCaptureView(s, {
    session: 1,
    phase: 'drag',
    rect: { x: 10, y: 10, w: 300, h: 150 },
    path: ['화면', '창'],
    level: -1,
    guides: [],
    magnet: true,
    cursorIncluded: false,
    help: false,
  });
  assert.equal(s.seq, seqWithResult);
  assert.equal(s.current.text, '색 복사');
  assert.equal(s.activeMode.value, '300×150');
  assert.deepEqual(s.activeMode.tags, []);

  // Dismissing transient result restores the freshly updated mode island
  s = dismissIsland(s);
  assert.equal(s.current.value, '300×150');
  assert.equal(shouldShowHintLine(s), false);
});
