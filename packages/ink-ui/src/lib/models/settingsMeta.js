// 설정 키 메타데이터(이름·표시값)와 가져오기 변경 목록·겹침 문구 도우미 (순수 JS)
import { shortenPath } from './template.js';
import { formatComboReadable, subjectParticle } from './shortcutDisplay.js';
import { compactFontLabel, parseFontSetting } from './fontPicker.js';
import { normalizeOs, translate } from '../i18n/core.js';

/** @type {Record<string, string>} 설정 키 → 카탈로그 키 */
export const SETTING_LABEL_KEYS = {
  'capture.result': 'meta.setting.capture.result',
  'capture.cursor': 'meta.setting.capture.cursor',
  'capture.sound': 'meta.setting.capture.sound',
  'capture.element': 'meta.setting.capture.element',
  'capture.magnet': 'meta.setting.capture.magnet',
  'capture.historyMax': 'meta.setting.capture.historyMax',
  'capture.excluded': 'meta.setting.capture.excluded',
  'capture.delay': 'integ.setting.capture.delay',
  'output.folder': 'meta.setting.output.folder',
  'output.template': 'meta.setting.output.template',
  'record.fps': 'meta.setting.record.fps',
  'record.mic': 'meta.setting.record.mic',
  'record.micDevice': 'meta.setting.record.micDevice',
  'record.cursor': 'integ.setting.record.cursor',
  'draw.color': 'meta.setting.draw.color',
  'draw.width': 'meta.setting.draw.width',
  'draw.smoothing': 'meta.setting.draw.smoothing',
  'draw.straighten': 'meta.setting.draw.straighten',
  'draw.shapeOnHold': 'meta.setting.draw.shapeOnHold',
  'draw.shapeApply': 'meta.setting.draw.shapeApply',
  'draw.startZoom': 'meta.setting.draw.startZoom',
  'draw.keep': 'meta.setting.draw.keep',
  'draw.esc': 'meta.setting.draw.esc',
  'draw.rightClickPan': 'meta.setting.draw.rightClickPan',
  'annotate.font': 'integ.setting.annotate.font',
  'annotate.style': 'integ.setting.annotate.style',
  'zoom.follow': 'meta.setting.zoom.follow',
  'zoom.minimap': 'meta.setting.zoom.minimap',
  'general.theme': 'meta.setting.general.theme',
  'general.lang': 'meta.setting.general.lang',
  'general.autostart': 'meta.setting.general.autostart',
  'update.auto': 'updateUi.autoLabel',
  'update.snooze': 'integ.setting.update.snooze',
  'topmost.border': 'meta.setting.topmost.border',
  'topmost.sound': 'meta.setting.topmost.sound',
};

/** 설정 키 → 기본 한국어 쉬운 말 이름 (하위 호환용 export) */
export const SETTING_LABELS = Object.fromEntries(
  Object.entries(SETTING_LABEL_KEYS).map(([path, key]) => [path, translate('ko', key)]),
);

const VALUE_KEY_MAPS = {
  'capture.result': {
    copy: 'page.capture.result.copy',
    save: 'page.capture.result.save',
    both: 'page.capture.result.both',
  },
  'draw.color': {
    R: 'capture.color.R',
    Y: 'capture.color.Y',
    G: 'capture.color.G',
    O: 'capture.color.O',
    B: 'capture.color.B',
    L: 'capture.color.L',
  },
  'draw.smoothing': {
    off: 'page.draw.smoothing.off',
    light: 'page.draw.smoothing.light',
    strong: 'page.draw.smoothing.strong',
  },
  'draw.shapeApply': {
    suggest: 'page.draw.shapeApply.suggest',
    auto: 'page.draw.shapeApply.auto',
  },
  'draw.esc': {
    step: 'page.draw.esc.step',
    exit: 'page.draw.esc.exit',
  },
  'zoom.follow': {
    pointer: 'page.draw.zoomFollow.pointer',
    smooth: 'meta.value.zoomFollow.smoothFull',
    instant: 'meta.value.zoomFollow.instantFull',
  },
  'general.theme': {
    system: 'general.appearance.themeSystem',
    light: 'general.appearance.themeLight',
    dark: 'general.appearance.themeDark',
  },
  'general.lang': {
    ko: 'general.appearance.langKo',
    en: 'general.appearance.langEn',
  },
};

/**
 * 설정 키의 표시 이름을 돌려준다.
 * @param {string} path
 * @param {string} [lang='ko']
 * @returns {string}
 */
export function getSettingLabel(path, lang = 'ko') {
  const key = SETTING_LABEL_KEYS[path];
  return key ? translate(lang, key) : path;
}

/**
 * 설정 값을 사용자가 읽을 짧은 문자열로 바꾼다.
 * @param {string} path
 * @param {any} value
 * @param {string} [lang='ko']
 * @returns {string}
 */
export function formatSettingValue(path, value, lang = 'ko') {
  if (typeof value === 'boolean') return translate(lang, value ? 'common.on' : 'common.off');
  if (path === 'record.micDevice') return value ? String(value) : translate(lang, 'page.capture.micDefault');
  if (path === 'output.folder') return shortenPath(String(value ?? ''), 22);
  if (path === 'record.fps') return `${value}fps`;
  if (path === 'draw.width') return `${value}px`;
  if (path === 'draw.startZoom') return `×${value}`;
  if (path === 'capture.historyMax') return translate(lang, 'meta.value.countItems', { n: value });
  if (path === 'capture.delay') return `${value}s`;
  if (path === 'annotate.font') {
    if (!value) return translate(lang, 'common.empty');
    const parsed = parseFontSetting(value);
    return compactFontLabel(parsed.family, parsed.weight);
  }
  if (path === 'topmost.border') return Number(value) === 0 ? translate(lang, 'common.off') : `${value}px`;
  if (path === 'capture.excluded') {
    return translate(lang, 'meta.value.countItems', { n: Array.isArray(value) ? value.length : 0 });
  }
  const key = VALUE_KEY_MAPS[path]?.[String(value)];
  if (key) return translate(lang, key);
  return value === '' || value === null || value === undefined ? translate(lang, 'common.empty') : String(value);
}

/**
 * `previewImport()`의 `SettingChange` 한 건을 시트 표시용으로 변환한다.
 * @param {{ path: string, before: any, after: any }} change
 * @param {Array<{ id: string, label: string }>} [shortcuts=[]]
 * @param {string} [lang='ko']
 */
export function describeSettingChange(change, shortcuts = [], lang = 'ko') {
  const emptyText = translate(lang, 'common.empty');
  if (change.path.startsWith('shortcuts.')) {
    const id = change.path.slice('shortcuts.'.length);
    const row = shortcuts.find((s) => s.id === id);
    return {
      path: change.path,
      label: row?.label ?? id,
      isShortcut: true,
      beforeCombo: String(change.before ?? ''),
      afterCombo: String(change.after ?? ''),
      beforeText: change.before ? String(change.before) : emptyText,
      afterText: change.after ? String(change.after) : emptyText,
    };
  }
  return {
    path: change.path,
    label: getSettingLabel(change.path, lang),
    isShortcut: false,
    beforeCombo: '',
    afterCombo: '',
    beforeText: formatSettingValue(change.path, change.before, lang),
    afterText: formatSettingValue(change.path, change.after, lang),
  };
}

/**
 * "멈추면 도형으로" 줄의 보조 문구 — 고른 도형 보정 방식(draw.shapeApply)과 OS 되돌리기 키에 맞춘다.
 * @param {'suggest' | 'auto' | string | null | undefined} shapeApply
 * @param {'windows' | 'macos' | string} [platform='windows']
 * @param {string} [lang='ko']
 * @param {string} [undoCombo='']
 * @returns {string}
 */
export function formatShapeOnHoldSub(shapeApply, platform = 'windows', lang = 'ko', undoCombo = '') {
  if (shapeApply !== 'auto') {
    return translate(lang, 'page.draw.shapeOnHoldSubSuggest');
  }
  const resolvedOs = normalizeOs(platform) === 'mac' ? 'macos' : 'windows';
  const fallback = resolvedOs === 'macos' ? 'Cmd+Z' : 'Ctrl+Z';
  const undo = formatComboReadable(undoCombo || fallback, resolvedOs) || (resolvedOs === 'macos' ? '⌘Z' : 'Ctrl Z');
  return translate(lang, 'page.draw.shapeOnHoldSubAuto', { undo });
}

/**
 * 설정 위치(`state.location`) 표시 문자열을 만든다.
 * @param {{ path: string, portable: boolean, readOnly?: boolean, packaged?: boolean } | null | undefined} location
 * @param {number} [maxPathLen=32]
 * @param {'unpackaged' | 'msix-store' | 'msix-direct' | string} [channel]
 * @param {'windows' | 'macos' | string} [platform='windows']
 * @param {string} [lang='ko']
 */
export function formatLocationSub(
  location,
  maxPathLen = 32,
  channel = undefined,
  platform = 'windows',
  lang = 'ko',
) {
  if (!location) return '';
  const isMsix =
    Boolean(location.packaged) ||
    channel === 'msix-store' ||
    channel === 'msix-direct';
  const kind = isMsix
    ? translate(lang, 'meta.storage.msix')
    : location.portable
      ? translate(lang, 'meta.storage.portable')
      : translate(lang, 'meta.storage.installed', undefined, platform);
  const short = shortenPath(location.path ?? '', maxPathLen);
  return short ? `${kind} · ${short}` : kind;
}

/**
 * 첫 실행 시트(S06) 겹침 줄 문구 생성.
 * @param {{ count: number, app?: string, combos: string[] } | null | undefined} conflicts
 * @param {'windows' | 'macos'} [platform='windows']
 * @param {string} [lang='ko']
 */
export function formatFirstRunConflict(conflicts, platform = 'windows', lang = 'ko') {
  if (!conflicts || conflicts.count <= 0) return null;
  const app = conflicts.app || '';
  if (app.includes('PowerToys')) {
    const combo = formatComboReadable('Alt+Shift+D', platform);
    return {
      prefix: '',
      app: 'PowerToys',
      suffix: translate(lang, 'meta.conflict.powertoys', { combo }),
      canHowto: true,
    };
  }
  const combos = (conflicts.combos ?? []).filter(Boolean);
  const mainKeys = combos.map((c) => c.split('+').at(-1) || '').filter(Boolean);
  const allFKeys = mainKeys.length > 0 && mainKeys.every((k) => /^F([1-9]|1[0-9]|2[0-4])$/i.test(k));

  if (app) {
    if (allFKeys) {
      const uniq = [...new Set(mainKeys)].slice(0, 2);
      const keys = uniq.join('·');
      const suffix =
        conflicts.count > 1
          ? translate(lang, 'meta.conflict.appMulti', { keys, n: conflicts.count })
          : translate(lang, 'meta.conflict.appSingle', { key: uniq[0] });
      return { prefix: '', app, suffix, canHowto: true };
    }
    if (combos.length > 0) {
      const uniq = [...new Set(combos)].map((c) => formatComboReadable(c, platform)).filter(Boolean);
      const shown = uniq.slice(0, 2).join(' · ');
      const extra = Math.max(conflicts.count, uniq.length) - Math.min(2, uniq.length);
      const suffix =
        extra > 0
          ? translate(lang, 'meta.conflict.appOverflow', { keys: shown, extra })
          : translate(lang, 'meta.conflict.appSingle', { key: shown });
      return { prefix: '', app, suffix, canHowto: true };
    }
    return {
      prefix: '',
      app,
      suffix: translate(lang, 'meta.conflict.appCountOnly', { n: conflicts.count }),
      canHowto: true,
    };
  }

  return {
    prefix: '',
    app: '',
    suffix: translate(lang, 'meta.conflict.generic', { n: conflicts.count }),
    canHowto: false,
  };
}

export { subjectParticle };
