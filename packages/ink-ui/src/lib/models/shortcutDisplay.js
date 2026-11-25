// S02 단축키 목록·겹침 배너 표시 규칙 (순수 JS)
import { keycaps } from '@screen-ink/client';

const F_KEY_RE = /^F([1-9]|1[0-9]|2[0-4])$/i;
const TRAILING_PUNCT_RE = /[\s'"’”)\]]+$/;

/**
 * 주격 조사('이'/'가')를 고른다.
 * 한글 음절로 끝나면 받침 유무로 '이'/'가'를 판정하고,
 * 영문·숫자 등 비한글 끝은 기존 표기("Snipaste가")처럼 '가'를 쓴다.
 * @param {string} word
 * @returns {'이' | '가'}
 */
export function subjectParticle(word) {
  const cleaned = String(word ?? '').replace(TRAILING_PUNCT_RE, '');
  if (!cleaned) return '가';
  const code = cleaned.charCodeAt(cleaned.length - 1);
  if (code >= 0xac00 && code <= 0xd7a3) {
    return (code - 0xac00) % 28 !== 0 ? '이' : '가';
  }
  return '가';
}

/**
 * 키 조합을 문장 안 읽기 표기로 바꾼다.
 * Windows: "Alt Shift D", macOS: "⌥⇧D" (10-design-system.md §8).
 * @param {string} combo
 * @param {'windows' | 'macos'} [platform='windows']
 * @returns {string}
 */
export function formatComboReadable(combo, platform = 'windows') {
  const caps = keycaps(combo, platform);
  if (caps.length === 0) return '';
  return platform === 'macos' ? caps.join('') : caps.join(' ');
}

/**
 * 겹침 배너(S02)의 강조 앱 이름(lead)과 본문(message)을 만든다.
 * - 겹친 조합들의 주 키가 모두 F키면 F04처럼 "F1·F3 키 4개를" (1개면 "F1 키를")
 * - 그 밖에는 조합 읽기 표기 최대 2개 + "외 n개를"
 * @param {{ count: number, app?: string, combos?: string[] } | null | undefined} conflicts
 * @param {'windows' | 'macos'} [platform='windows']
 * @returns {{ lead: string, message: string } | null}
 */
export function formatConflictBanner(conflicts, platform = 'windows') {
  if (!conflicts || conflicts.count <= 0) return null;

  const count = conflicts.count;
  const app = conflicts.app || '';
  if (!app) {
    return {
      lead: '',
      message: `단축키 ${count}개가 겹치고 있어요`,
    };
  }

  const particle = subjectParticle(app);
  const combos = (conflicts.combos ?? []).filter(Boolean);
  const mainKeys = combos.map((c) => c.split('+').at(-1) || '').filter(Boolean);
  const allFKeys = mainKeys.length > 0 && mainKeys.every((k) => F_KEY_RE.test(k));

  if (allFKeys) {
    const uniqF = [...new Set(mainKeys)];
    const shown = (uniqF.length <= 3 ? uniqF : uniqF.slice(0, 2)).join('·');
    const keyPhrase = count > 1 ? `${shown} 키 ${count}개를` : `${shown} 키를`;
    return {
      lead: app,
      message: `${particle} ${keyPhrase} 먼저 쓰고 있어요`,
    };
  }

  if (combos.length > 0) {
    const uniqCombos = [...new Set(combos)];
    const readable = uniqCombos.map((c) => formatComboReadable(c, platform)).filter(Boolean);
    const shown = readable.slice(0, 2).join(' · ');
    const total = Math.max(count, readable.length);
    const shownCount = Math.min(2, readable.length);
    const extra = total - shownCount;
    const keyPhrase = extra > 0 ? `${shown} 외 ${extra}개를` : `${shown} 키를`;
    return {
      lead: app,
      message: `${particle} ${keyPhrase} 먼저 쓰고 있어요`,
    };
  }

  return {
    lead: app,
    message: `${particle} 키 ${count}개를 먼저 쓰고 있어요`,
  };
}

/**
 * S02 단축키 한 줄의 점·보조 문구·오류·주의 표시 상태를 계산한다.
 * - 평소에는 conflict 줄만 주황 점('attn'), warn 줄은 점·이유 없이 한 줄.
 * - warn 이유는 hover/focus 때 hoverSub(--ink-sub), 키 입력 중엔 note(--ink-sub).
 * - conflict 이유는 맨 위 배너가 설명하는 앱(bannerApp)과 같은 원인이면 줄 아래에 쓰지 않고 점만.
 * @param {{ status?: string, reason?: string, owner?: string }} row
 * @param {{
 *   isRecording?: boolean,
 *   feedback?: { error?: string, suggestions?: string[] } | null,
 *   bannerApp?: string,
 * }} [ctx]
 */
export function getShortcutRowDisplay(row, ctx = {}) {
  const isRecording = Boolean(ctx.isRecording);
  const feedback = ctx.feedback ?? null;
  const bannerApp = ctx.bannerApp ?? '';
  const hasError = Boolean(feedback?.error);

  const dotStatus = !isRecording && row?.status === 'conflict' ? 'attn' : '';
  const sub = isRecording ? '키를 누르세요 · Esc 취소' : '';
  const hoverSub = !isRecording && !hasError && row?.status === 'warn' ? row?.reason || '' : '';
  const note = isRecording && !hasError && row?.status === 'warn' ? row?.reason || '' : '';

  const coveredByBanner = Boolean(bannerApp && row?.owner && row.owner === bannerApp);
  const warning =
    !isRecording && !hasError && row?.status === 'conflict' && !coveredByBanner
      ? row?.reason || ''
      : '';

  return {
    dotStatus,
    sub,
    hoverSub,
    note,
    error: feedback?.error || '',
    warning,
    suggestions: feedback?.suggestions || [],
  };
}
