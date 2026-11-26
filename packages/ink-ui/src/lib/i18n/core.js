// 다국어 핵심(R2-I18N) — 순수 함수. 카탈로그 = 같은 폴더 ko.json·en.json(Rust `ink_domain::i18n`도 같은 파일을 읽는다).
// ko.json·en.json은 `areas/<영역>.<언어>.json`을 합친 결과다 — 고칠 때는 영역 파일을 고치고 `node scripts/i18n-build.mjs`.
// 키는 점 표기(`island.zoom.title`), 값 끼우기는 `{이름}`. 없는 키는 한국어 → 키 그대로 순서로 대신한다.
// OS별 글(R2-OSFIT-1): 같은 키에 `~mac`·`~win`을 붙인 변형이 있으면 그 OS에서 먼저 쓴다(`island.paste~mac`).
import ko from './ko.json' with { type: 'json' };
import en from './en.json' with { type: 'json' };

/** @typedef {'ko' | 'en'} Lang */

/** @type {Record<Lang, Record<string, string>>} */
export const CATALOGS = { ko, en };
export const LANGS = /** @type {Lang[]} */ (['ko', 'en']);

/** @typedef {'mac' | 'win'} Os */

/** InkState.platform(`macos`·`windows`) 등 → `mac`·`win`(모르면 `win`). */
export function normalizeOs(v) {
  const s = String(v ?? '').toLowerCase();
  return s.startsWith('mac') || s === 'darwin' ? 'mac' : 'win';
}

/** 설정 값·OS 언어 문자열 → 지원 언어(모르면 한국어). `en-US` → `en`. */
export function normalizeLang(v) {
  const s = String(v ?? '').toLowerCase();
  return s.startsWith('en') ? 'en' : 'ko';
}

/**
 * @param {string} lang
 * @param {string} key
 * @param {Record<string, string | number>} [params]
 * @param {string} [os] `mac`·`win`(또는 `macos`·`windows`) — 주면 `key~os` 변형을 먼저
 */
export function translate(lang, key, params, os) {
  const l = normalizeLang(lang);
  const v = os ? `${key}~${normalizeOs(os)}` : null;
  const s = (v && (CATALOGS[l][v] ?? ko[v])) ?? CATALOGS[l][key] ?? ko[key] ?? key;
  if (!params) return s;
  return s.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m));
}

/** 글 안의 `{이름}` 목록(시험·검사용). */
export function placeholders(s) {
  return [...String(s).matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
}
