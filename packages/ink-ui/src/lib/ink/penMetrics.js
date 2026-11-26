// M5 펜 지연 계측(A §3.6, R-DRAW-5) — 입력 사건 시각(event.timeStamp)부터 그 표본을 그린 직후
// performance.now()까지의 소프트웨어 지연을 모아 { n, p50, p95, max }로 요약한다. DOM 의존 0.

/** @typedef {{ n: number, p50: number, p95: number, max: number }} PenLatencySummary */

/** 표본을 이만큼까지만 보관(오래된 것부터 덮어씀). */
export const PEN_METRICS_MAX_SAMPLES = 20000;

/**
 * 가장 가까운 순위 백분위(0.1 ms 반올림).
 * @param {ArrayLike<number>} values
 * @returns {PenLatencySummary}
 */
export function summarizeLatency(values) {
  const n = values.length;
  if (!n) return { n: 0, p50: 0, p95: 0, max: 0 };
  const sorted = Array.from(values).sort((a, b) => a - b);
  const pick = (/** @type {number} */ q) => sorted[Math.min(n - 1, Math.max(0, Math.ceil(q * n) - 1))];
  const r = (/** @type {number} */ v) => Math.round(v * 10) / 10;
  return { n, p50: r(pick(0.5)), p95: r(pick(0.95)), max: r(sorted[n - 1]) };
}

/**
 * 펜 지연 측정기. mark(사건 시각) → 그린 뒤 painted(지금) 한 번에 대기 중 표본의 지연을 기록.
 * @param {{ maxSamples?: number }} [opts]
 */
export function createPenLatencyMeter(opts = {}) {
  const cap = opts.maxSamples ?? PEN_METRICS_MAX_SAMPLES;
  /** @type {number[]} */
  let samples = [];
  let head = 0;
  /** @type {number[]} */
  let pending = [];
  /** @param {number} ms */
  const record = (ms) => {
    if (!(ms >= 0) || !Number.isFinite(ms)) return;
    if (samples.length < cap) samples.push(ms);
    else {
      samples[head] = ms;
      head = (head + 1) % cap;
    }
  };
  return {
    /** @param {number} eventTimeStamp */
    mark(eventTimeStamp) {
      pending.push(eventTimeStamp);
    },
    /** 그린 직후 부른다. @param {number} now */
    painted(now) {
      if (!pending.length) return 0;
      for (const t of pending) record(now - t);
      const k = pending.length;
      pending = [];
      return k;
    },
    record,
    get count() {
      return samples.length;
    },
    get pendingCount() {
      return pending.length;
    },
    /** @returns {PenLatencySummary} */
    summary() {
      return summarizeLatency(samples);
    },
    reset() {
      samples = [];
      pending = [];
      head = 0;
    },
  };
}
