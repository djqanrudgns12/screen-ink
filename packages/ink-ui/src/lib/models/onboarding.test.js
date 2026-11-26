import test from 'node:test';
import assert from 'node:assert/strict';
import {
  ONBOARDING_STEP_COUNT,
  canAdvanceStep,
  canJumpToStep,
  getConflictHowtoKind,
  getOnboardingSteps,
  resolveInitialStep,
  summarizeCheckup,
} from './onboarding.js';

test('맥과 Windows는 서로 다른 카드 구성을 가지지만 모두 5장이다', () => {
  const mac = getOnboardingSteps('macos');
  const win = getOnboardingSteps('windows');
  assert.equal(mac.length, ONBOARDING_STEP_COUNT);
  assert.equal(win.length, ONBOARDING_STEP_COUNT);
  assert.deepEqual([...mac], ['welcome', 'permScreen', 'permAx', 'keys', 'ready']);
  assert.deepEqual([...win], ['welcome', 'conflicts', 'autostart', 'keys', 'ready']);
});

test('시작 카드 번호는 풀린 권한을 우선하고 onboarding.step을 0~4로 제한한다', () => {
  assert.equal(
    resolveInitialStep({
      platform: 'macos',
      permissions: { screen: false, accessibility: true, screenRevoked: true, accessibilityRevoked: false },
      settings: { 'onboarding.step': 4 },
    }),
    1,
  );
  assert.equal(
    resolveInitialStep({
      platform: 'macos',
      permissions: { screen: true, accessibility: false, screenRevoked: false, accessibilityRevoked: true },
      settings: { 'onboarding.step': 4 },
    }),
    2,
  );
  assert.equal(
    resolveInitialStep({
      platform: 'macos',
      permissions: { screen: true, accessibility: true, screenRevoked: false, accessibilityRevoked: false },
      settings: { 'onboarding.step': 3 },
    }),
    3,
  );
  assert.equal(resolveInitialStep({ platform: 'windows', settings: { 'onboarding.step': 99 } }), 4);
  assert.equal(resolveInitialStep({ platform: 'windows', settings: { 'onboarding.step': -2 } }), 0);
});

test('맥 권한 카드는 권한이 켜지거나 나중에 할게요를 확인해야 다음으로 간다', () => {
  const off = { screen: false, accessibility: false };
  assert.equal(canAdvanceStep(0, 'macos', off, false), true);
  assert.equal(canAdvanceStep(1, 'macos', off, false), false);
  assert.equal(canAdvanceStep(2, 'macos', off, false), false);
  assert.equal(canAdvanceStep(1, 'macos', { screen: true, accessibility: false }, false), true);
  assert.equal(canAdvanceStep(2, 'macos', { screen: true, accessibility: false }, true), true);
  assert.equal(canAdvanceStep(1, 'windows', undefined, false), true);
});

test('점 표시 점프는 뒤로 가기는 항상 허용하고, 앞서는 필수 카드를 건너뛰지 못한다', () => {
  const onlyScreen = { screen: true, accessibility: false };
  assert.equal(canJumpToStep(0, 2, 'macos', onlyScreen, false), true);
  assert.equal(canJumpToStep(2, 1, 'macos', onlyScreen, false), true);
  assert.equal(canJumpToStep(3, 1, 'macos', onlyScreen, false), false);
  assert.equal(canJumpToStep(4, 1, 'macos', onlyScreen, true), true);
  assert.equal(canJumpToStep(5, 1, 'windows', undefined, false), false);
});

test('겹침 앱 이름에 맞는 끄는 법 모달 종류를 고른다', () => {
  assert.equal(getConflictHowtoKind('Snipaste'), 'snipaste');
  assert.equal(getConflictHowtoKind('Microsoft PowerToys'), 'powertoys');
  assert.equal(getConflictHowtoKind('OtherApp'), 'generic');
});

test('설정 일반 점검 요약은 맥 권한 상태와 Windows 겹침·시작 상태를 계산한다', () => {
  const macRelaunch = summarizeCheckup(
    {
      platform: 'macos',
      permissions: { screen: false, accessibility: true, screenRevoked: false, accessibilityRevoked: false },
    },
    { screenAsked: true },
  );
  assert.equal(macRelaunch.allOk, false);
  assert.equal(macRelaunch.issueCount, 1);
  assert.equal(macRelaunch.items[0].status, 'relaunch');
  assert.equal(macRelaunch.items[1].status, 'ok');

  const winWarn = summarizeCheckup({
    platform: 'windows',
    conflicts: { count: 2, app: 'Snipaste', combos: ['F1', 'F3'] },
    settings: { 'general.autostart': false },
  });
  assert.equal(winWarn.allOk, false);
  assert.equal(winWarn.issueCount, 1);
  assert.equal(winWarn.items[0].status, 'warn');
  assert.equal(winWarn.items[1].status, 'off');
});
