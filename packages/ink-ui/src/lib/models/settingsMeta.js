// 설정 키 메타데이터(이름·표시값)와 가져오기 변경 목록·겹침 문구 도우미 (순수 JS)
import { shortenPath } from './template.js';
import { formatComboReadable, subjectParticle } from './shortcutDisplay.js';

/** 설정 키 → S03~S05 화면 줄 이름 */
export const SETTING_LABELS = {
  'capture.result': 'Enter · 더블클릭 하면',
  'capture.cursor': '마우스 커서도 찍기',
  'capture.sound': '찰칵 소리',
  'capture.element': 'UI 요소까지 찾기',
  'capture.magnet': '모서리 자석',
  'capture.historyMax': '캡처 기록',
  'capture.excluded': '찾지 않을 앱',
  'output.folder': '저장 폴더',
  'output.template': '파일 이름',
  'record.fps': '부드러움',
  'record.mic': '마이크 소리',
  'record.micDevice': '마이크 장치',
  'draw.color': '처음 색',
  'draw.width': '굵기',
  'draw.smoothing': '떨림 보정',
  'draw.straighten': '거의 곧은 선은 곧게',
  'draw.shapeOnHold': '멈추면 도형으로',
  'draw.startZoom': '첫 휠 배율',
  'draw.keep': '나갈 때 보관',
  'draw.esc': 'Esc 키',
  'draw.rightClickPan': '확대 중 오른쪽 클릭',
  'zoom.follow': '커서 따라가기',
  'zoom.minimap': '위치 지도 보이기',
  'general.theme': '테마',
  'general.lang': '언어',
  'general.autostart': '로그인하면 실행',
  'topmost.border': '고정한 창 테두리',
  'topmost.sound': '고정할 때 소리',
};

const VALUE_MAPS = {
  'capture.result': { copy: '복사', save: '저장', both: '둘 다' },
  'draw.color': { R: '빨강', Y: '노랑', G: '초록', O: '주황', B: '파랑', L: '연두' },
  'draw.smoothing': { off: '끔', light: '약', strong: '강' },
  'draw.esc': { step: '한 단계씩', exit: '바로 나가기' },
  'zoom.follow': { smooth: '부드럽게', instant: '바로' },
  'general.theme': { system: '시스템 따라', light: '라이트', dark: '다크' },
  'general.lang': { ko: '한국어', en: 'English' },
};

/**
 * 설정 값을 사용자가 읽을 짧은 한글 문자열로 바꾼다.
 * @param {string} path
 * @param {any} value
 * @returns {string}
 */
export function formatSettingValue(path, value) {
  if (typeof value === 'boolean') return value ? '켬' : '끔';
  if (path === 'record.micDevice') return value ? String(value) : '기본 장치';
  if (path === 'output.folder') return shortenPath(String(value ?? ''), 22);
  if (path === 'record.fps') return `${value}fps`;
  if (path === 'draw.width') return `${value}px`;
  if (path === 'draw.startZoom') return `×${value}`;
  if (path === 'capture.historyMax') return `${value}개`;
  if (path === 'topmost.border') return Number(value) === 0 ? '끔' : `${value}px`;
  if (path === 'capture.excluded') return Array.isArray(value) ? `${value.length}개` : '0개';
  const mapped = VALUE_MAPS[path]?.[String(value)];
  if (mapped) return mapped;
  return value === '' || value === null || value === undefined ? '비움' : String(value);
}

/**
 * `previewImport()`의 `SettingChange` 한 건을 시트 표시용으로 변환한다.
 * @param {{ path: string, before: any, after: any }} change
 * @param {Array<{ id: string, label: string }>} [shortcuts=[]]
 */
export function describeSettingChange(change, shortcuts = []) {
  if (change.path.startsWith('shortcuts.')) {
    const id = change.path.slice('shortcuts.'.length);
    const row = shortcuts.find((s) => s.id === id);
    return {
      path: change.path,
      label: row?.label ?? id,
      isShortcut: true,
      beforeCombo: String(change.before ?? ''),
      afterCombo: String(change.after ?? ''),
      beforeText: change.before ? String(change.before) : '비움',
      afterText: change.after ? String(change.after) : '비움',
    };
  }
  return {
    path: change.path,
    label: SETTING_LABELS[change.path] ?? change.path,
    isShortcut: false,
    beforeCombo: '',
    afterCombo: '',
    beforeText: formatSettingValue(change.path, change.before),
    afterText: formatSettingValue(change.path, change.after),
  };
}

/**
 * 설정 위치(`state.location`) 표시 문자열을 만든다.
 * @param {{ path: string, portable: boolean, readOnly?: boolean } | null | undefined} location
 * @param {number} [maxPathLen=32]
 */
export function formatLocationSub(location, maxPathLen = 32) {
  if (!location) return '';
  const kind = location.portable ? '포터블' : '설치';
  const short = shortenPath(location.path ?? '', maxPathLen);
  return short ? `${kind} · ${short}` : kind;
}

/**
 * 첫 실행 시트(S06) 겹침 줄 문구 생성.
 * @param {{ count: number, app?: string, combos: string[] } | null | undefined} conflicts
 * @param {'windows' | 'macos'} [platform='windows']
 */
export function formatFirstRunConflict(conflicts, platform = 'windows') {
  if (!conflicts || conflicts.count <= 0) return null;
  const app = conflicts.app || '';
  if (app.includes('PowerToys')) {
    return {
      prefix: '',
      app: 'PowerToys',
      suffix: "에서 '항상 위'를 끄면 Alt+Shift+D를 써요",
      canHowto: true,
    };
  }
  const combos = (conflicts.combos ?? []).filter(Boolean);
  const mainKeys = combos.map((c) => c.split('+').at(-1) || '').filter(Boolean);
  const allFKeys = mainKeys.length > 0 && mainKeys.every((k) => /^F([1-9]|1[0-9]|2[0-4])$/i.test(k));

  let prefix = '';
  if (allFKeys) {
    const uniq = [...new Set(mainKeys)].slice(0, 2);
    prefix = conflicts.count > 1 ? `${uniq.join('·')} 키 ${conflicts.count}개는 ` : `${uniq[0]} 키는 `;
  } else if (combos.length > 0) {
    const uniq = [...new Set(combos)].map((c) => formatComboReadable(c, platform)).filter(Boolean);
    const shown = uniq.slice(0, 2).join(' · ');
    const extra = Math.max(conflicts.count, uniq.length) - Math.min(2, uniq.length);
    prefix = extra > 0 ? `${shown} 외 ${extra}개는 ` : `${shown} 키는 `;
  } else {
    prefix = `키 ${conflicts.count}개는 `;
  }

  if (app) {
    return {
      prefix,
      app,
      suffix: `${subjectParticle(app)} 쓰고 있어요`,
      canHowto: true,
    };
  }
  return {
    prefix: '',
    app: '',
    suffix: `단축키 ${conflicts.count}개가 겹치고 있어요`,
    canHowto: false,
  };
}
