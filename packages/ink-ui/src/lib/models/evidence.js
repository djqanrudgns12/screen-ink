// 숨은 "인식 근거 보기"(08 §4.5) 순수 모델 — 마디 근거 → 색·글자, 물리 → 이 모니터 CSS.

import { resolveMonitorHighlight } from './captureOverlay.js';

/** 근거 색(08 §4.5: W 파랑 · A 초록 · M 청록 · V 주황). H(자식 창)는 W와 같은 파랑. */
export const EVIDENCE_COLORS = Object.freeze({
  A: '#22C55E',
  V: '#FF9F0A',
  M: '#14B8A6',
  W: '#3B82F6',
});

/**
 * 마디의 근거 글자("W·A·V")에서 테두리 색을 고른다 — 가장 구체적인 근거 순(A > V > M > W·H).
 * @param {string} sources
 * @returns {string}
 */
export function evidenceColor(sources) {
  const has = new Set(String(sources || '').split('·').filter(Boolean));
  if (has.has('A')) return EVIDENCE_COLORS.A;
  if (has.has('V')) return EVIDENCE_COLORS.V;
  if (has.has('M')) return EVIDENCE_COLORS.M;
  return EVIDENCE_COLORS.W;
}

/**
 * 보기의 근거 마디를 이 모니터에 그릴 모양으로(겹치지 않는 마디는 뺌).
 * @param {Array<{ rect: {x:number,y:number,w:number,h:number}, label: string, score: number, sources: string, current: boolean, hidden: boolean }> | null | undefined} evidence
 * @param {{ bounds: { x: number, y: number, w: number, h: number }, scale?: number } | null | undefined} monitor
 */
export function evidenceBoxes(evidence, monitor) {
  if (!Array.isArray(evidence) || !monitor) return [];
  const out = [];
  for (const e of evidence) {
    const css = resolveMonitorHighlight(e.rect, monitor);
    if (!css) continue;
    out.push({
      ...css,
      color: evidenceColor(e.sources),
      tag: `${e.label} ${e.sources || '-'} ${Number.isFinite(e.score) ? e.score.toFixed(2) : ''}`.trim(),
      current: Boolean(e.current),
      hidden: Boolean(e.hidden),
    });
  }
  return out;
}
