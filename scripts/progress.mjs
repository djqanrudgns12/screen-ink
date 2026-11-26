#!/usr/bin/env node
// 진척도: docs/progress/progress.json(원본) → docs/progress/README.md(보고서).
//   npm run progress          보고서를 다시 쓰고 요약을 찍는다
//   npm run progress -- --check   보고서가 원본과 어긋나면 실패(커밋 전 확인용)
// 진척(%) = Σ(작업일 × 상태 값) ÷ Σ작업일. 상태 값은 원본의 scale(완료 1 · 확인 남음 0.8 · 진행 중 0.5 · 안 함 0).

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const SRC = join(root, 'docs/progress/progress.json');
const OUT = join(root, 'docs/progress/README.md');

/** @param {any} data */
export function validate(data) {
  const errors = [];
  const ids = new Set();
  const statuses = Object.keys(data.scale ?? {});
  const platforms = Object.keys(data.platforms ?? {});
  for (const t of data.tracks ?? []) {
    for (const g of t.groups ?? []) {
      for (const it of g.items ?? []) {
        if (ids.has(it.id)) errors.push(`중복 id: ${it.id}`);
        ids.add(it.id);
        if (!statuses.includes(it.status)) errors.push(`${it.id}: 모르는 상태 ${it.status}`);
        if (!platforms.includes(it.platform)) errors.push(`${it.id}: 모르는 플랫폼 ${it.platform}`);
        if (!(typeof it.days === 'number' && it.days > 0)) errors.push(`${it.id}: days는 양수`);
        if (data.levels && it.status !== 'done' && !(it.level in data.levels)) {
          errors.push(`${it.id}: 남은 일은 난이도(level: ${Object.keys(data.levels).join('|')}) 필요`);
        }
      }
    }
  }
  for (const [p, list] of Object.entries(data.next ?? {})) {
    for (const id of list) if (!ids.has(id)) errors.push(`next.${p}: 없는 id ${id}`);
  }
  return errors;
}

/** @param {any} data @param {any[]} items */
function sum(data, items) {
  const days = items.reduce((a, it) => a + it.days, 0);
  const got = items.reduce((a, it) => a + it.days * data.scale[it.status].value, 0);
  const count = Object.fromEntries(Object.keys(data.scale).map((s) => [s, items.filter((it) => it.status === s).length]));
  return { days, got, pct: days ? (got / days) * 100 : 0, count };
}

const all = (/** @type {any} */ t) => t.groups.flatMap((/** @type {any} */ g) => g.items);
const fmtPct = (/** @type {number} */ p) => `${p.toFixed(1)}%`;
const bar = (/** @type {number} */ p) => {
  const n = Math.round(p / 5);
  return '█'.repeat(n) + '░'.repeat(20 - n);
};
const days = (/** @type {number} */ d) => (Math.round(d * 100) / 100).toString();

/** 작업량 칸: 항목 작업일 기준 */
const SIZES = /** @type {const} */ ([
  ['S', '작음(≤0.3일)', (/** @type {number} */ d) => d <= 0.3],
  ['M', '보통(≤0.6일)', (/** @type {number} */ d) => d > 0.3 && d <= 0.6],
  ['L', '큼(>0.6일)', (/** @type {number} */ d) => d > 0.6],
]);

/**
 * 남은 일 — 난이도 × 작업량 표와 난이도별 목록(남은 환산 작업일이 큰 순).
 * @param {any} data
 * @param {string[]} L
 * @param {(p: string) => string} plat
 * @param {(s: string) => string} label
 */
function renderLevels(data, L, plat, label) {
  const left = data.tracks.flatMap(all).filter((/** @type {any} */ it) => it.status !== 'done');
  const remain = (/** @type {any} */ it) => it.days * (1 - data.scale[it.status].value);
  const cell = (/** @type {any[]} */ xs) => (xs.length ? `${xs.length}개 · ${days(xs.reduce((a, it) => a + remain(it), 0))}일` : '—');
  L.push('## 남은 일 — 난이도 × 작업량', '');
  L.push(
    '난이도: ' + Object.entries(data.levels).map(([k, v]) => `**${v}** = ${data.levelMeaning?.[k] ?? ''}`).join(' · ') + '. 작업량 칸 = 항목 작업일, 숫자 = 남은 환산 작업일.',
    '',
  );
  L.push(`| 난이도 | ${SIZES.map((s) => s[1]).join(' | ')} | 합계 |`, `|---|${SIZES.map(() => '---:').join('|')}|---:|`);
  for (const [lv, name] of Object.entries(data.levels)) {
    const row = left.filter((/** @type {any} */ it) => it.level === lv);
    L.push(`| ${name} | ${SIZES.map((s) => cell(row.filter((/** @type {any} */ it) => s[2](it.days)))).join(' | ')} | ${cell(row)} |`);
  }
  L.push(`| **합계** | ${SIZES.map((s) => cell(left.filter((/** @type {any} */ it) => s[2](it.days)))).join(' | ')} | ${cell(left)} |`, '');
  for (const [lv, name] of Object.entries(data.levels)) {
    const row = left.filter((/** @type {any} */ it) => it.level === lv).sort((/** @type {any} */ a, /** @type {any} */ b) => remain(b) - remain(a));
    L.push(`### 난이도 ${name} (${row.length}개, 남은 ${days(row.reduce((/** @type {number} */ a, /** @type {any} */ it) => a + remain(it), 0))}일)`, '');
    for (const it of row) L.push(`- \`${it.id}\` ${it.title} — ${plat(it.platform)} · ${days(it.days)}일 · ${label(it.status)}`);
    L.push('');
  }
}

/** @param {any} data */
export function render(data) {
  const L = [];
  const label = (/** @type {string} */ s) => data.scale[s].label;
  const plat = (/** @type {string} */ p) => data.platforms[p];
  const tracks = data.tracks.map((/** @type {any} */ t) => ({ t, s: sum(data, all(t)) }));
  const everything = sum(data, data.tracks.flatMap(all));
  const byId = new Map(data.tracks.flatMap(all).map((/** @type {any} */ it) => [it.id, it]));

  L.push('# 진척도', '');
  L.push(`> 자동 생성 — 고치지 말고 [progress.json](progress.json)을 고친 뒤 \`npm run progress\`. 기준일 ${data.updated} · ${data.basis}`, '');
  L.push('## 계산 규칙', '');
  L.push('진척 = Σ(작업일 × 상태 값) ÷ Σ작업일. 작업일은 07 마스터플랜의 일수(단계 안 항목은 나눔), 맥·배포 트랙은 추정 일수.', '');
  L.push('| 상태 | 값 | 뜻 |', '|---|---|---|');
  for (const [k, v] of Object.entries(data.scale)) L.push(`| ${v.label} | ${v.value} | ${v.meaning} |`);
  L.push('');

  L.push('## 한눈에', '');
  L.push('| 트랙 | 작업일 | 진척 | | 완료 | 확인 남음 | 진행 중 | 안 함 |', '|---|---:|---:|---|---:|---:|---:|---:|');
  for (const { t, s } of tracks) {
    L.push(`| ${t.name} | ${days(s.days)} | **${fmtPct(s.pct)}** | \`${bar(s.pct)}\` | ${s.count.done} | ${s.count.verify} | ${s.count.partial} | ${s.count.todo} |`);
  }
  const e = everything;
  L.push(`| **전체** | ${days(e.days)} | **${fmtPct(e.pct)}** | \`${bar(e.pct)}\` | ${e.count.done} | ${e.count.verify} | ${e.count.partial} | ${e.count.todo} |`, '');

  L.push('### 플랫폼별', '');
  L.push('| 플랫폼 | 작업일 | 진척 | 남은 작업일(환산) |', '|---|---:|---:|---:|');
  for (const p of Object.keys(data.platforms)) {
    const s = sum(data, data.tracks.flatMap(all).filter((/** @type {any} */ it) => it.platform === p));
    L.push(`| ${plat(p)} | ${days(s.days)} | ${fmtPct(s.pct)} | ${days(s.days - s.got)} |`);
  }
  L.push('');

  L.push('## 지금 할 순서', '');
  for (const [p, list] of Object.entries(data.next ?? {})) {
    L.push(`- **${plat(p)}**: ${list.map((/** @type {string} */ id, /** @type {number} */ i) => `${i + 1}) ${byId.get(id).title} \`${id}\``).join(' → ')}`);
  }
  L.push('');

  if (data.levels) renderLevels(data, L, plat, label);

  for (const { t, s } of tracks) {
    L.push(`## ${t.name} — ${fmtPct(s.pct)}`, '');
    for (const g of t.groups) {
      const gs = sum(data, g.items);
      L.push(`### ${g.name} — ${fmtPct(gs.pct)}`, '');
      L.push('| id | 할 일 | 플랫폼 | 일 | 상태 | 메모 |', '|---|---|---|---:|---|---|');
      for (const it of g.items) {
        const st = it.status === 'done' ? `✅ ${label(it.status)}` : it.status === 'todo' ? `⬜ ${label(it.status)}` : `🟡 ${label(it.status)}`;
        L.push(`| \`${it.id}\` | ${it.title} | ${plat(it.platform)} | ${days(it.days)} | ${st} | ${it.note ?? ''} |`);
      }
      L.push('');
    }
  }

  L.push('## 남은 일 — 플랫폼별', '');
  for (const p of Object.keys(data.platforms)) {
    const left = data.tracks.flatMap((/** @type {any} */ t) => all(t).map((/** @type {any} */ it) => ({ ...it, track: t.name }))).filter((/** @type {any} */ it) => it.platform === p && it.status !== 'done');
    L.push(`### ${plat(p)} (${left.length}개)`, '');
    for (const it of left) L.push(`- [${it.status === 'todo' ? ' ' : '~'}] \`${it.id}\` ${it.title} — ${label(it.status)}${it.note ? ` (${it.note})` : ''}`);
    L.push('');
  }
  return L.join('\n');
}

function main() {
  const data = JSON.parse(readFileSync(SRC, 'utf8'));
  const errors = validate(data);
  if (errors.length) {
    console.error(errors.join('\n'));
    process.exit(1);
  }
  const md = render(data);
  if (process.argv.includes('--check')) {
    const now = readFileSync(OUT, 'utf8');
    if (now.replace(/\r\n/g, '\n') !== md) {
      console.error('docs/progress/README.md가 progress.json과 다릅니다 — npm run progress');
      process.exit(1);
    }
    console.log('진척도 보고서 최신');
    return;
  }
  writeFileSync(OUT, md);
  for (const t of data.tracks) console.log(`${t.name}: ${fmtPct(sum(data, all(t)).pct)}`);
  console.log(`전체: ${fmtPct(sum(data, data.tracks.flatMap(all)).pct)}`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) main();
