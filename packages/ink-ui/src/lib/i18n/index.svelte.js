// 화면용 다국어: `t(key, params)`는 지금 언어(`locale.lang`)를 읽어 Svelte가 언어가 바뀌면 다시 그린다(R-SET-0 바로 반영).
// 언어는 InkState의 설정 `general.lang`에서 `syncLang(state)`로 맞춘다(창마다 한 번 — App·SettingsWindow).
import { normalizeLang, translate } from './core.js';

export { CATALOGS, LANGS, normalizeLang, placeholders, translate } from './core.js';

export const locale = $state({ lang: /** @type {import('./core.js').Lang} */ ('ko') });

/** @param {string} key @param {Record<string, string | number>} [params] */
export function t(key, params) {
  return translate(locale.lang, key, params);
}

/** @param {string} lang */
export function setLang(lang) {
  const l = normalizeLang(lang);
  if (locale.lang !== l) locale.lang = l;
}

/** InkState(또는 설정 표)에서 언어를 맞춘다. */
export function syncLang(state) {
  const v = state?.settings?.['general.lang'];
  if (v) setLang(v);
}
