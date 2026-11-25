// 테마 결정(system / light / dark) 순수 모델

/**
 * 사용자 설정과 시스템 다크 여부로 실제 적용 테마를 결정한다.
 * @param {'system'|'light'|'dark'|string} pref
 * @param {boolean} [systemPrefersDark=false]
 * @returns {'light'|'dark'}
 */
export function resolveTheme(pref, systemPrefersDark = false) {
  if (pref === 'light' || pref === 'dark') return pref;
  return systemPrefersDark ? 'dark' : 'light';
}

/**
 * 요소에 `data-theme` 속성을 적용하고 해석된 테마를 돌려준다.
 * @param {{ setAttribute?: (k: string, v: string) => void } | null | undefined} target
 * @param {'system'|'light'|'dark'|string} pref
 * @param {boolean} [systemPrefersDark=false]
 * @returns {'light'|'dark'}
 */
export function applyTheme(target, pref, systemPrefersDark = false) {
  const resolved = resolveTheme(pref, systemPrefersDark);
  if (target && typeof target.setAttribute === 'function') {
    target.setAttribute('data-theme', resolved);
  }
  return resolved;
}

/**
 * 시스템 다크 모드 변경을 구독한다.
 * @param {(isDark: boolean) => void} onChange
 * @param {((q: string) => MediaQueryList) | null} [matchMediaFn]
 * @returns {() => void}
 */
export function watchSystemTheme(onChange, matchMediaFn) {
  const mm = matchMediaFn ?? (typeof window !== 'undefined' && typeof window.matchMedia === 'function' ? window.matchMedia.bind(window) : null);
  if (!mm) return () => {};
  const mq = mm('(prefers-color-scheme: dark)');
  const handler = (e) => onChange(Boolean(e.matches));
  if (typeof mq.addEventListener === 'function') {
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }
  return () => {};
}
