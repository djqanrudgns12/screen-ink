// 화면 위 섬(Island) 상태 기계 — S17 / 10 §6 규칙
// - 한 번에 하나만 표시
// - 결과 1.2초 자동 닫힘(줄임 모드 2초, 마우스 올리면 멈춤)
// - 새 알림이 오면 이전 것을 밀어냄; 모드 활성 중 결과는 0.8초 겹쳐 보인 뒤 모드 상태로 돌아감
// - 키 안내 둘째 줄은 해당 모드 처음 3번 또는 '?' 요청 때만 표시

export const ISLAND_DURATIONS = {
  result: 1200,
  resultReduced: 2000,
  resultOverMode: 800,
  info: 1200,
  off: 2000,
  problem: 6000,
  mode: null,
};

export const HINT_MAX_AUTO_SHOW = 3;

/** 섬 늘기·줄기 스프링 규격(10-design-system.md §6). */
export const ISLAND_SPRING = Object.freeze({
  stiffness: 0.22,
  damping: 0.78,
  contentDelayMs: 80,
});

/**
 * @typedef {Object} IslandQueueState
 * @property {any | null} current       지금 화면에 보이는 섬
 * @property {any | null} activeMode    바탕 모드 섬(결과 뒤에 복귀)
 * @property {number | null} expiresAt  자동 닫힘 시각(ms), null이면 계속 보임
 * @property {boolean} hovering         마우스가 올라가 있어 타이머 일시 정지
 * @property {number} seq               변경 시퀀스(애니메이션/진행선 리셋용)
 * @property {Record<string, number>} modeUses 모드별 사용 횟수
 * @property {boolean} helpOpen         '?' 키로 안내 줄 강제 표시
 */

/**
 * @param {{ modeUses?: Record<string, number> }} [init]
 * @returns {IslandQueueState}
 */
export function createIslandState(init = {}) {
  return {
    current: null,
    activeMode: null,
    expiresAt: null,
    hovering: false,
    seq: 0,
    modeUses: { ...(init.modeUses || {}) },
    helpOpen: false,
  };
}

/**
 * 섬 종류와 조건에 따라 자동 닫힘 시간(ms)을 계산.
 * @param {any} island
 * @param {{ hasActiveMode?: boolean, reducedMotion?: boolean }} [opts]
 * @returns {number | null}
 */
export function getAutoDismissMs(island, { hasActiveMode = false, reducedMotion = false } = {}) {
  if (!island) return null;
  if (typeof island.durationMs === 'number') return island.durationMs;
  if (island.kind === 'mode') return null;
  if (island.kind === 'result') {
    if (hasActiveMode) return ISLAND_DURATIONS.resultOverMode;
    return reducedMotion ? ISLAND_DURATIONS.resultReduced : ISLAND_DURATIONS.result;
  }
  if (island.kind === 'problem') return ISLAND_DURATIONS.problem;
  if (island.kind === 'off') return ISLAND_DURATIONS.off;
  return reducedMotion ? ISLAND_DURATIONS.resultReduced : ISLAND_DURATIONS.info;
}

/**
 * 새 섬 이벤트를 넣는다. 한 번에 하나 규칙 + 대체 규칙 적용.
 * @param {IslandQueueState} state
 * @param {any} island
 * @param {{ now?: number, reducedMotion?: boolean }} [opts]
 * @returns {IslandQueueState}
 */
export function pushIsland(state, island, { now = Date.now(), reducedMotion = false } = {}) {
  if (!island) return state;
  if (island.kind === 'end') return clearModeIsland(state);

  const nextSeq = state.seq + 1;
  const modeUses = { ...state.modeUses };

  if (island.kind === 'mode') {
    const modeKey = island.modeKey || island.mode || island.text;
    if (modeKey && (!state.activeMode || (state.activeMode.modeKey || state.activeMode.mode || state.activeMode.text) !== modeKey)) {
      modeUses[modeKey] = (modeUses[modeKey] || 0) + 1;
    }
    return {
      ...state,
      current: island,
      activeMode: island,
      expiresAt: null,
      hovering: false,
      seq: nextSeq,
      modeUses,
    };
  }

  const hasActiveMode = Boolean(state.activeMode);
  const duration = getAutoDismissMs(island, { hasActiveMode, reducedMotion });
  return {
    ...state,
    current: island,
    expiresAt: duration === null ? null : now + duration,
    hovering: false,
    seq: nextSeq,
    modeUses,
  };
}

/**
 * 현재 임시 섬을 닫는다. 활성 모드가 있으면 모드 섬으로 돌아간다.
 * @param {IslandQueueState} state
 * @returns {IslandQueueState}
 */
export function dismissIsland(state) {
  if (!state.current) return state;
  if (state.current.kind === 'mode') {
    return {
      ...state,
      current: null,
      activeMode: null,
      expiresAt: null,
      hovering: false,
      helpOpen: false,
    };
  }
  if (state.activeMode) {
    return {
      ...state,
      current: state.activeMode,
      expiresAt: null,
      hovering: false,
      seq: state.seq + 1,
    };
  }
  return {
    ...state,
    current: null,
    expiresAt: null,
    hovering: false,
  };
}

/**
 * 활성 모드를 끝낸다.
 * @param {IslandQueueState} state
 * @returns {IslandQueueState}
 */
export function clearModeIsland(state) {
  const showingMode = state.current && state.current.kind === 'mode';
  return {
    ...state,
    activeMode: null,
    current: showingMode ? null : state.current,
    expiresAt: showingMode ? null : state.expiresAt,
    hovering: showingMode ? false : state.hovering,
    helpOpen: false,
  };
}

/**
 * 마우스 올림/나감 처리 — 올리면 타이머 정지, 나가면 새 1.2초(또는 원래 지속 시간) 시작.
 * @param {IslandQueueState} state
 * @param {boolean} hovering
 * @param {{ now?: number, reducedMotion?: boolean }} [opts]
 * @returns {IslandQueueState}
 */
export function setIslandHover(state, hovering, { now = Date.now(), reducedMotion = false } = {}) {
  if (!state.current || state.current.kind === 'mode') {
    return { ...state, hovering: false };
  }
  if (hovering) {
    return { ...state, hovering: true, expiresAt: null };
  }
  const duration = getAutoDismissMs(state.current, {
    hasActiveMode: Boolean(state.activeMode),
    reducedMotion,
  });
  return {
    ...state,
    hovering: false,
    expiresAt: duration === null ? null : now + duration,
  };
}

/**
 * '?' 키 토글 상태 갱신.
 * @param {IslandQueueState} state
 * @param {boolean} open
 * @returns {IslandQueueState}
 */
export function setHelpOpen(state, open) {
  return { ...state, helpOpen: Boolean(open) };
}

/**
 * 키 안내 둘째 줄을 보일지 결정: 처음 3번 사용 또는 '?'가 켜진 경우만 true.
 * @param {IslandQueueState} state
 * @param {string} [modeKey]
 * @returns {boolean}
 */
export function shouldShowHintLine(state, modeKey) {
  if (!state || !state.current) return false;
  const hints = state.current.hints;
  if (!Array.isArray(hints) || hints.length === 0) return false;
  if (state.helpOpen) return true;
  const key = modeKey || state.current.modeKey || state.current.mode || state.current.text;
  const uses = state.modeUses[key] || 0;
  return uses <= HINT_MAX_AUTO_SHOW;
}
