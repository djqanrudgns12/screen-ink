// 화면 위 섬(Island) 상태 기계 — S17 / 10 §6 규칙
// - 한 번에 하나만 표시
// - 결과 1.2초 자동 닫힘(줄임 모드 2초, 마우스 올리면 멈춤)
// - 새 알림이 오면 이전 것을 밀어냄; 모드 활성 중 결과는 0.8초 겹쳐 보인 뒤 모드 상태로 돌아감
// - 키 안내 둘째 줄은 해당 모드 처음 3번 또는 '?' 요청 때만 표시
import { translate } from '../i18n/core.js';

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

/**
 * 인식 사슬(path)과 현재 단계(level)를 섬 표시용 조각 배열로 다듬는다.
 * - level 항목만 active=true(흰색), 나머지는 보조색
 * - 길이가 maxChars 또는 maxItems를 넘으면 앞쪽부터 줄여 '…'를 붙인다(현재 단계는 항상 유지)
 * @param {string[] | null | undefined} path
 * @param {number} level
 * @param {{ maxChars?: number, maxItems?: number }} [opts]
 * @returns {{ label: string, active: boolean, ellipsis?: boolean }[]}
 */
export function formatCapturePath(path, level, { maxChars = 20, maxItems = 4 } = {}) {
  if (!Array.isArray(path) || path.length === 0) return [];
  if (!Number.isInteger(level) || level < 0 || level >= path.length) return [];

  let start = 0;
  let end = path.length;

  const measure = (s, e) => {
    const count = e - s + (s > 0 ? 1 : 0) + (e < path.length ? 1 : 0);
    let chars = 0;
    if (s > 0) chars += 1;
    for (let i = s; i < e; i += 1) {
      chars += String(path[i] ?? '').length;
    }
    if (e < path.length) chars += 1;
    if (count > 1) chars += (count - 1) * 3; // ' › '
    return { count, chars };
  };

  while (start < level) {
    const { count, chars } = measure(start, end);
    if (count <= maxItems && chars <= maxChars) break;
    start += 1;
  }

  while (end > level + 1) {
    const { count, chars } = measure(start, end);
    if (count <= maxItems && chars <= maxChars) break;
    end -= 1;
  }

  /** @type {{ label: string, active: boolean, ellipsis?: boolean }[]} */
  const out = [];
  if (start > 0) {
    out.push({ label: '…', active: false, ellipsis: true });
  }
  for (let i = start; i < end; i += 1) {
    out.push({ label: String(path[i] ?? ''), active: i === level });
  }
  if (end < path.length) {
    out.push({ label: '…', active: false, ellipsis: true });
  }
  return out;
}

/**
 * CaptureView에서 모드 섬에 반영할 필드(경로·크기·상태 태그·도움말)를 뽑는다.
 * @param {import('@screen-ink/client').CaptureView | null | undefined} view
 * @param {string} [lang='ko']
 * @returns {{ path: string[], level: number, value: string, tags: string[], help: boolean } | null}
 */
export function buildCaptureIslandPatch(view, lang = 'ko') {
  if (!view) return null;
  const rect = view.rect;
  const value =
    rect && Number.isFinite(rect.w) && Number.isFinite(rect.h) && rect.w > 0 && rect.h > 0
      ? `${Math.round(rect.w)}×${Math.round(rect.h)}`
      : '';
  const tags = [];
  if (view.magnet === false) tags.push(translate(lang, 'islandUi.tag.snapOff'));
  if (view.cursorIncluded) tags.push(translate(lang, 'islandUi.tag.cursorIncluded'));
  return {
    path: Array.isArray(view.path) ? view.path : [],
    level: typeof view.level === 'number' ? view.level : -1,
    value,
    tags,
    help: Boolean(view.help),
  };
}

/**
 * 활성 모드 섬을 새로 만들지 않고(seq 유지) CaptureView 정보로 그 자리에서 갱신한다.
 * 일시 결과 섬이 위에 떠 있으면 activeMode만 배경에서 갱신한다.
 * @param {IslandQueueState} state
 * @param {import('@screen-ink/client').CaptureView | null | undefined} view
 * @param {string} [lang='ko']
 * @returns {IslandQueueState}
 */
export function applyCaptureView(state, view, lang = 'ko') {
  if (!state || !state.activeMode || !view) return state;
  const patch = buildCaptureIslandPatch(view, lang);
  if (!patch) return state;

  const nextMode = {
    ...state.activeMode,
    path: patch.path,
    level: patch.level,
    value: patch.value,
    tags: patch.tags,
  };
  const showingMode = Boolean(state.current && state.current.kind === 'mode');
  return {
    ...state,
    activeMode: nextMode,
    current: showingMode ? nextMode : state.current,
    helpOpen: patch.help,
  };
}

/**
 * 칩 문구(예: "#FF9F0A 복사")에서 6자리 #RRGGBB 색 코드를 뽑아 대문자로 돌려준다.
 * @param {string | null | undefined} label
 * @returns {string | null}
 */
export function extractHexColor(label) {
  if (typeof label !== 'string') return null;
  const match = label.match(/(?:^|\s)(#[0-9a-fA-F]{6})(?![0-9a-fA-F])/);
  return match ? match[1].toUpperCase() : null;
}

/**
 * 결과 섬 추천 칩 표시 정보(라벨·견본 색·동작)를 정리한다.
 * @param {{ label?: string, action?: string, hex?: string } | null | undefined} chip
 * @returns {{ label: string, hex: string | null, action: string } | null}
 */
export function resolveIslandChip(chip) {
  if (!chip || typeof chip.label !== 'string' || !chip.label.trim()) return null;
  const hex = extractHexColor(chip.hex) ?? extractHexColor(chip.label);
  return {
    label: chip.label,
    hex,
    action: typeof chip.action === 'string' ? chip.action : '',
  };
}

/**
 * 520px 폭 안에 액션 단추가 3개(주·보조·칩) 동시에 들어가거나 실제로 넘칠 때 단추 안 키캡을 생략한다(시안 결정 #4).
 * @param {{ button?: any, secondaryButton?: any, chip?: any, overflow?: boolean }} [opts]
 * @returns {boolean}
 */
export function shouldOmitButtonKeys({
  button = null,
  secondaryButton = null,
  chip = null,
  overflow = false,
} = {}) {
  if (overflow) return true;
  const actionCount = (button ? 1 : 0) + (secondaryButton ? 1 : 0) + (chip ? 1 : 0);
  return actionCount >= 3;
}
