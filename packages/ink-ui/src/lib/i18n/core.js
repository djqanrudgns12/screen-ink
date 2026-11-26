// 다국어 핵심(R2-I18N) — 순수 함수. 카탈로그 = 같은 폴더 ko.json·en.json(Rust `ink_domain::i18n`도 같은 파일을 읽는다).
// 키는 점 표기(`island.zoom.title`), 값 끼우기는 `{이름}`. 없는 키는 한국어 → 키 그대로 순서로 대신한다.
import ko from './ko.json' with { type: 'json' };
import en from './en.json' with { type: 'json' };

/** @typedef {'ko' | 'en'} Lang */

/** @type {Record<Lang, Record<string, string>>} */
export const CATALOGS = { ko, en };
export const LANGS = /** @type {Lang[]} */ (['ko', 'en']);

/** 설정 값·OS 언어 문자열 → 지원 언어(모르면 한국어). `en-US` → `en`. */
export function normalizeLang(v) {
  const s = String(v ?? '').toLowerCase();
  return s.startsWith('en') ? 'en' : 'ko';
}

/**
 * @param {string} lang
 * @param {string} key
 * @param {Record<string, string | number>} [params]
 */
export function translate(lang, key, params) {
  const l = normalizeLang(lang);
  const s = CATALOGS[l][key] ?? ko[key] ?? key;
  if (!params) return s;
  return s.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m));
}

/** 글 안의 `{이름}` 목록(시험·검사용). */
export function placeholders(s) {
  return [...String(s).matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
}
