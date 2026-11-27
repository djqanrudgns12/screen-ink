// 첫 실행 카드 5장(R2-ONB) 및 일반 탭 점검 묶음(R2-ONB-6) 순수 모델

export const ONBOARDING_STEP_COUNT = 5;

/** @type {readonly string[]} */
export const MAC_STEPS = ['welcome', 'permScreen', 'permAx', 'keys', 'ready'];

/** @type {readonly string[]} */
export const WIN_STEPS = ['welcome', 'conflicts', 'autostart', 'keys', 'ready'];

/**
 * OS별 5장 카드 id 배열을 돌려준다.
 * @param {'macos' | 'windows' | string | undefined} platform
 */
export function getOnboardingSteps(platform) {
  return platform === 'macos' ? MAC_STEPS : WIN_STEPS;
}

/**
 * 첫 실행을 열 때 시작할 카드 번호(0~4)를 결정한다(R2-ONB-3, R2-ONB-7).
 * - 맥에서 권한이 풀렸으면(revoked) 해당 권한 카드부터 다시 띄운다.
 * - 그 외에는 저장된 `onboarding.step`(0~4)을 안전하게 0~4 정수로 잘라 쓴다.
 * @param {any} state
 * @returns {number}
 */
export function resolveInitialStep(state) {
  const platform = state?.platform ?? 'windows';
  const perms = state?.permissions;
  if (platform === 'macos' && perms) {
    if (perms.screenRevoked && !perms.screen) return 1;
    if (perms.accessibilityRevoked && !perms.accessibility) return 2;
  }
  const raw = Number(state?.settings?.['onboarding.step'] ?? 0);
  if (!Number.isFinite(raw)) return 0;
  return Math.max(0, Math.min(ONBOARDING_STEP_COUNT - 1, Math.trunc(raw)));
}

/**
 * 현재 카드에서 [다음]으로 넘어갈 수 있는지 판정한다(R2-ONB-4).
 * - 맥 1번(화면 기록)·2번(손쉬운 사용)은 권한이 켜졌거나 "나중에 할게요"를 확인했을 때만 통과.
 * @param {number} stepIndex
 * @param {'macos' | 'windows' | string | undefined} platform
 * @param {{ screen?: boolean, accessibility?: boolean } | null | undefined} permissions
 * @param {boolean} [skipped=false]
 * @returns {boolean}
 */
export function canAdvanceStep(stepIndex, platform, permissions, skipped = false) {
  if (platform !== 'macos') return true;
  if (stepIndex === 1) return Boolean(skipped || permissions?.screen);
  if (stepIndex === 2) return Boolean(skipped || permissions?.accessibility);
  return true;
}

/**
 * 아래 점 표시 클릭 등으로 특정 카드 번호로 점프할 수 있는지 확인한다.
 * - 뒤로 가는 것은 항상 허용, 앞으로 가는 것은 앞선 필수 카드들이 통과 가능할 때만 허용.
 * @param {number} targetIndex
 * @param {number} currentIndex
 * @param {'macos' | 'windows' | string | undefined} platform
 * @param {{ screen?: boolean, accessibility?: boolean } | null | undefined} permissions
 * @param {boolean} [skipped=false]
 * @returns {boolean}
 */
export function canJumpToStep(targetIndex, currentIndex, platform, permissions, skipped = false) {
  if (!Number.isInteger(targetIndex) || targetIndex < 0 || targetIndex >= ONBOARDING_STEP_COUNT) {
    return false;
  }
  if (targetIndex <= currentIndex) return true;
  for (let i = 0; i < targetIndex; i += 1) {
    if (!canAdvanceStep(i, platform, permissions, skipped)) return false;
  }
  return true;
}

/**
 * 겹침 앱 이름에 따른 [끄는 법] 모달 종류를 고른다(R2-FIX-2).
 * @param {string | undefined | null} appName
 * @returns {'snipaste' | 'powertoys' | 'generic'}
 */
export function getConflictHowtoKind(appName) {
  const name = String(appName ?? '').toLowerCase();
  if (name.includes('snipaste')) return 'snipaste';
  if (name.includes('powertoys')) return 'powertoys';
  return 'generic';
}

/**
 * 설정 › 일반 › 점검 묶음의 항목과 한 줄 요약 상태를 만든다(R2-ONB-6).
 * @param {any} state
 * @param {{ screenAsked?: boolean }} [opts]
 */
/**
 * 녹화 마이크 권한 항목(맥·Windows 공통). 녹화 마이크를 켠 사람에게만 "확인 필요"로 센다.
 * status: ok(허용됨) · ask(아직 안 물음 → [허용 요청]) · off(꺼짐 → [열기]) · unsupported(이 OS에서 못 씀)
 * @param {any} state
 */
export function summarizeMicItem(state) {
  const raw = state?.microphone;
  /** @type {'ok' | 'ask' | 'off' | 'unsupported'} */
  const status = raw === 'granted' ? 'ok' : raw === 'notDetermined' ? 'ask' : raw === 'denied' ? 'off' : 'unsupported';
  const wanted = Boolean(state?.settings?.['record.mic']);
  return { id: 'mic', status, wanted, action: 'perm.microphone', isIssue: wanted && (status === 'ask' || status === 'off') };
}

export function summarizeCheckup(state, { screenAsked = false } = {}) {
  const platform = state?.platform ?? 'windows';
  const mic = summarizeMicItem(state);
  const micIssue = mic.isIssue ? 1 : 0;
  if (platform === 'macos') {
    const perms = state?.permissions ?? {
      screen: true,
      accessibility: true,
      screenRevoked: false,
      accessibilityRevoked: false,
    };
    /** @type {'ok' | 'revoked' | 'relaunch' | 'missing'} */
    const screenStatus = perms.screen
      ? 'ok'
      : perms.screenRevoked
        ? 'revoked'
        : screenAsked
          ? 'relaunch'
          : 'missing';
    /** @type {'ok' | 'revoked' | 'missing'} */
    const axStatus = perms.accessibility
      ? 'ok'
      : perms.accessibilityRevoked
        ? 'revoked'
        : 'missing';

    const items = [
      {
        id: 'screen',
        status: screenStatus,
        action: 'perm.screen',
        canRelaunch: !perms.screen,
      },
      {
        id: 'accessibility',
        status: axStatus,
        action: 'perm.accessibility',
        canRelaunch: false,
      },
    ];
    const issueCount = items.filter((item) => item.status !== 'ok').length + micIssue;
    return {
      platform: 'macos',
      items,
      mic,
      issueCount,
      allOk: issueCount === 0,
    };
  }

  const conflicts = state?.conflicts ?? { count: 0, app: undefined, combos: [] };
  const conflictCount = Number(conflicts.count ?? 0);
  const autostart = Boolean(state?.settings?.['general.autostart']);
  const autostartNotice = String(state?.autostartNotice ?? '');
  const items = [
    {
      id: 'conflicts',
      status: conflictCount > 0 ? 'warn' : 'ok',
      count: conflictCount,
      app: conflicts.app || '',
      combos: Array.isArray(conflicts.combos) ? conflicts.combos : [],
    },
    {
      id: 'autostart',
      status: autostartNotice ? 'warn' : autostart ? 'ok' : 'off',
      enabled: autostart,
      notice: autostartNotice,
    },
  ];
  const issueCount = (conflictCount > 0 ? 1 : 0) + (autostartNotice ? 1 : 0) + micIssue;
  return {
    platform: 'windows',
    items,
    mic,
    issueCount,
    allOk: issueCount === 0,
  };
}

/**
 * 첫 실행 ⑤ 연습 카드의 캡처 완료 감지 상태(R2-ONB-9)를 만든다.
 * @param {number} [initialPins=0]
 */
export function createPracticeTracker(initialPins = 0) {
  return {
    sawCapture: false,
    completed: false,
    pins: Number(initialPins) || 0,
  };
}

/**
 * 클라이언트 구독 이벤트(`state` · `island` · `afterimage`)로 연습 캡처 완료를 판정한다.
 * - 캡처 진입(`mode === 'capture'`) 뒤 `afterimage` 또는 결과 섬(`island.kind === 'result'`)이 오거나
 *   핀 개수(`pins`)가 늘어나면 완료로 본다.
 * - 캡처 진입 뒤 아무 결과 없이 `idle` 등으로 돌아오면 취소로 보고 대기 상태를 푼다.
 * @param {{ sawCapture: boolean, completed: boolean, pins: number }} tracker
 * @param {{ type: string, state?: any, island?: any }} event
 */
export function stepPracticeTracker(tracker, event) {
  if (!tracker || tracker.completed || !event) return tracker;

  if (event.type === 'afterimage') {
    return { ...tracker, sawCapture: false, completed: true };
  }

  if (event.type === 'island') {
    if (tracker.sawCapture && event.island?.kind === 'result') {
      return { ...tracker, sawCapture: false, completed: true };
    }
    return tracker;
  }

  if (event.type === 'state' && event.state) {
    const nextPins = Number(event.state.pins ?? tracker.pins) || 0;
    const modeName = event.state.mode?.mode;
    if (modeName === 'capture') {
      return { ...tracker, sawCapture: true, pins: nextPins };
    }
    if (tracker.sawCapture) {
      const pinned = nextPins > tracker.pins;
      return { ...tracker, sawCapture: false, completed: pinned, pins: nextPins };
    }
    return { ...tracker, pins: nextPins };
  }

  return tracker;
}
