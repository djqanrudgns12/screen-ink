// 설정 찾기(R2-SET-1) — 쉬운 말·옛 말·영어 낱말·단축키 조합으로 설정 항목과 설명서를 찾는다.

/**
 * @typedef {{
 *   id: string,
 *   tab: 'shortcuts' | 'capture' | 'draw' | 'general' | 'help' | 'info',
 *   tabKey: string,
 *   titleKo: string,
 *   titleEn: string,
 *   matchTexts: string[],
 *   keywords: string[],
 *   platformOnly?: 'macos' | 'windows',
 * }} SettingsSearchEntry
 */

/** @type {SettingsSearchEntry[]} */
export const SETTINGS_SEARCH_ITEMS = [
  // ── 일반 › 점검 ──
  {
    id: 'general.checkup',
    tab: 'general',
    tabKey: 'settings.tab.general',
    titleKo: '권한 점검',
    titleEn: 'Permission check',
    matchTexts: ['권한 점검', 'Permission check', '화면 기록', '손쉬운 사용'],
    keywords: ['권한', '화면 기록', '손쉬운 사용', '점검', '다시 시작', 'permission', 'screen recording', 'accessibility', 'checkup', 'mac'],
    platformOnly: 'macos',
  },
  {
    id: 'general.checkup',
    tab: 'general',
    tabKey: 'settings.tab.general',
    titleKo: '단축키·시작 점검',
    titleEn: 'Shortcuts & startup check',
    matchTexts: ['단축키·시작 점검', 'Shortcuts & startup check', '단축키 겹침'],
    keywords: ['점검', '단축키 겹침', 'snipaste', 'powertoys', 'conflict', 'checkup', 'startup'],
    platformOnly: 'windows',
  },
  // ── 일반 › 모양·시작·창·데이터 ──
  {
    id: 'general.theme',
    tab: 'general',
    tabKey: 'settings.tab.general',
    titleKo: '테마',
    titleEn: 'Theme',
    matchTexts: ['테마', 'Theme'],
    keywords: ['모양', '밝게', '어둡게', '기기 설정', '라이트', '다크', '시스템 따라', 'theme', 'light', 'dark', 'system', 'appearance'],
  },
  {
    id: 'general.lang',
    tab: 'general',
    tabKey: 'settings.tab.general',
    titleKo: '언어',
    titleEn: 'Language',
    matchTexts: ['언어', 'Language'],
    keywords: ['한국어', '영어', '다국어', 'language', 'english', 'korean', 'locale'],
  },
  {
    id: 'general.autostart',
    tab: 'general',
    tabKey: 'settings.tab.general',
    titleKo: '로그인하면 실행',
    titleEn: 'Launch at sign-in',
    matchTexts: ['로그인하면 실행', 'Launch at sign-in'],
    keywords: ['시작', '자동 실행', '부팅', '켤 때 같이 켜기', '메뉴 막대', '작업 표시줄', '트레이', 'autostart', 'startup', 'login', 'tray'],
  },
  {
    id: 'general.firstRun',
    tab: 'general',
    tabKey: 'settings.tab.general',
    titleKo: '첫 실행 안내',
    titleEn: 'First-run guide',
    matchTexts: ['첫 실행 안내', 'First-run guide'],
    keywords: ['처음', '카드', '온보딩', '다시 보기', 'onboarding', 'first run', 'welcome'],
  },
  {
    id: 'general.topmost',
    tab: 'general',
    tabKey: 'settings.tab.general',
    titleKo: '창을 맨 위에',
    titleEn: 'Keep on top',
    matchTexts: ['고정한 창 테두리', 'Pinned window border', '고정한 창', 'Pinned windows'],
    keywords: ['창 항상 위', '창을 맨 위에', '핀', '고정', '테두리', '소리', 'topmost', 'always on top', 'pin', 'border'],
    platformOnly: 'windows',
  },
  {
    id: 'general.location',
    tab: 'general',
    tabKey: 'settings.tab.general',
    titleKo: '설정 위치',
    titleEn: 'Settings file',
    matchTexts: ['설정 위치', 'Settings file'],
    keywords: ['설정 파일', '폴더', 'finder', '탐색기', '경로', '포터블', 'location', 'settings.json', 'reveal', 'folder'],
  },
  {
    id: 'general.transfer',
    tab: 'general',
    tabKey: 'settings.tab.general',
    titleKo: '설정 옮기기',
    titleEn: 'Transfer settings',
    matchTexts: ['설정 옮기기', 'Transfer settings'],
    keywords: ['내보내기', '가져오기', '백업', 'export', 'import', 'backup', 'transfer'],
  },
  {
    id: 'general.logs',
    tab: 'general',
    tabKey: 'settings.tab.general',
    titleKo: '로그',
    titleEn: 'Logs',
    matchTexts: ['로그', 'Logs'],
    keywords: ['기록', '오류', '폴더 열기', '7일', 'logs', 'diagnostics', 'debug'],
  },
  {
    id: 'general.reset',
    tab: 'general',
    tabKey: 'settings.tab.general',
    titleKo: '모두 처음으로',
    titleEn: 'Reset all settings',
    matchTexts: ['모두 처음으로', 'Reset all settings'],
    keywords: ['초기화', '기본값', '되돌리기', '리셋', 'reset', 'default', 'restore'],
  },

  // ── 캡처·녹화 ──
  {
    id: 'capture.result',
    tab: 'capture',
    tabKey: 'settings.tab.capture',
    titleKo: 'Enter · 더블클릭 하면',
    titleEn: 'On Enter or double-click',
    matchTexts: ['Enter · 더블클릭 하면', 'On Enter or double-click'],
    keywords: ['복사', '저장', '둘 다', '클립보드', '결과', 'clipboard', 'copy', 'save', 'capture'],
  },
  {
    id: 'capture.cursor',
    tab: 'capture',
    tabKey: 'settings.tab.capture',
    titleKo: '마우스 커서도 찍기',
    titleEn: 'Include mouse cursor',
    matchTexts: ['마우스 커서도 찍기', 'Include mouse cursor'],
    keywords: ['커서', '포인터', '화살표', 'cursor', 'pointer', 'mouse'],
  },
  {
    id: 'capture.sound',
    tab: 'capture',
    tabKey: 'settings.tab.capture',
    titleKo: '찰칵 소리',
    titleEn: 'Shutter sound',
    matchTexts: ['찰칵 소리', 'Shutter sound'],
    keywords: ['소리', '효과음', '무음', '셔터', 'sound', 'audio', 'shutter'],
  },
  {
    id: 'capture.folder',
    tab: 'capture',
    tabKey: 'settings.tab.capture',
    titleKo: '저장 폴더',
    titleEn: 'Save folder',
    matchTexts: ['저장 폴더', 'Save folder'],
    keywords: ['경로', '폴더', '바꾸기', '열기', '사진', 'folder', 'output', 'directory', 'save'],
  },
  {
    id: 'capture.template',
    tab: 'capture',
    tabKey: 'settings.tab.capture',
    titleKo: '파일 이름',
    titleEn: 'Filename format',
    matchTexts: ['파일 이름', 'Filename format'],
    keywords: ['이름 규칙', '날짜', '번호', '템플릿', 'filename', 'template', 'naming'],
  },
  {
    id: 'capture.element',
    tab: 'capture',
    tabKey: 'settings.tab.capture',
    titleKo: 'UI 요소까지 찾기',
    titleEn: 'Detect UI elements',
    matchTexts: ['UI 요소까지 찾기', 'Detect UI elements'],
    keywords: ['스마트 인식', '자동 인식', '요소', '버튼', '표', '창 단위', 'element', 'detect', 'smart'],
  },
  {
    id: 'capture.magnet',
    tab: 'capture',
    tabKey: 'settings.tab.capture',
    titleKo: '자동 맞춤',
    titleEn: 'Snap to edges',
    matchTexts: ['모서리 자석', '자동 맞춤', 'Snap to edges'],
    keywords: ['모서리 자석', '자석', '맞춤', '스냅', '경계', 'magnet', 'snap', 'edges'],
  },
  {
    id: 'capture.excluded',
    tab: 'capture',
    tabKey: 'settings.tab.capture',
    titleKo: '찾지 않을 앱',
    titleEn: 'Excluded apps',
    matchTexts: ['찾지 않을 앱', 'Excluded apps'],
    keywords: ['제외', '예외', '느림', 'exe', '앱', 'exclude', 'ignore', 'app'],
  },
  {
    id: 'capture.fps',
    tab: 'capture',
    tabKey: 'settings.tab.capture',
    titleKo: '녹화 부드러움',
    titleEn: 'Recording smoothness',
    matchTexts: ['부드러움', 'Smoothness', '녹화 부드러움'],
    keywords: ['녹화', '프레임', '30 fps', '60 fps', '영상', 'record', 'fps', 'video'],
  },
  {
    id: 'capture.mic',
    tab: 'capture',
    tabKey: 'settings.tab.capture',
    titleKo: '마이크 소리',
    titleEn: 'Microphone',
    matchTexts: ['마이크 소리', 'Microphone'],
    keywords: ['녹화', '음성', '소리', '장치', 'mic', 'microphone', 'audio', 'voice'],
  },
  {
    id: 'capture.history',
    tab: 'capture',
    tabKey: 'settings.tab.capture',
    titleKo: '캡처 기록',
    titleEn: 'Capture history',
    matchTexts: ['캡처 기록', 'Capture history'],
    keywords: ['최근', '방금', '직전', '지우기', 'history', 'recent', 'clear'],
  },

  // ── 그리기·확대 ──
  {
    id: 'draw.color',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: '처음 색',
    titleEn: 'Default color',
    matchTexts: ['처음 색', 'Default color'],
    keywords: ['판서', '그리기', '펜 색', '빨강', '노랑', '파랑', 'color', 'pen', 'draw'],
  },
  {
    id: 'draw.width',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: '굵기',
    titleEn: 'Stroke width',
    matchTexts: ['굵기', 'Stroke width'],
    keywords: ['판서', '그리기', '두께', '선', '슬라이더', 'width', 'thickness', 'stroke', 'size'],
  },
  {
    id: 'draw.smoothing',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: '떨림 보정',
    titleEn: 'Stroke smoothing',
    matchTexts: ['떨림 보정', 'Stroke smoothing'],
    keywords: ['판서', '그리기', '손떨림', '부드럽게', '보정', 'smooth', 'stabilize'],
  },
  {
    id: 'draw.straighten',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: '거의 곧은 선은 곧게',
    titleEn: 'Straighten near-straight lines',
    matchTexts: ['거의 곧은 선은 곧게', 'Straighten near-straight lines'],
    keywords: ['직선', '수평', '수직', '판서', '그리기', 'straight', 'line'],
  },
  {
    id: 'draw.shapeOnHold',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: '멈추면 도형으로',
    titleEn: 'Snap to shape on hold',
    matchTexts: ['멈추면 도형으로', '도형 보정', 'Snap to shape on hold'],
    keywords: ['도형', '원', '사각형', '화살표', '보정', '판서', '그리기', 'shape', 'auto'],
  },
  {
    id: 'draw.startZoom',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: '첫 휠 배율',
    titleEn: 'First wheel zoom',
    matchTexts: ['첫 휠 배율', 'First wheel zoom'],
    keywords: ['판서', '그리기', '확대', '휠', '배율', 'zoom', 'wheel', 'scale'],
  },
  {
    id: 'draw.keep',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: '나갈 때 보관',
    titleEn: 'Keep drawing on exit',
    matchTexts: ['나갈 때 보관', 'Keep drawing on exit'],
    keywords: ['판서', '그리기', '보관', '복원', '다시 열기', 'keep', 'restore'],
  },
  {
    id: 'draw.esc',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: 'Esc 키',
    titleEn: 'Esc key behavior',
    matchTexts: ['Esc 키', 'Esc key behavior'],
    keywords: ['판서', '그리기', '나가기', '한 단계씩', 'esc', 'exit'],
  },
  {
    id: 'draw.rightClickPan',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: '확대 중 오른쪽 클릭',
    titleEn: 'Right-click while zoomed',
    matchTexts: ['확대 중 오른쪽 클릭', 'Right-click while zoomed'],
    keywords: ['손 모드', '이동', '판서', '그리기', '우클릭', 'pan', 'right click'],
  },
  {
    id: 'draw.boardPreview',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: '칠판 미리 보기',
    titleEn: 'Whiteboard preview',
    matchTexts: ['칠판 미리 보기', 'Whiteboard preview'],
    keywords: ['칠판', '흰색', '검은색', '화이트보드', '판서', 'whiteboard', 'chalkboard', 'board'],
  },
  {
    id: 'zoom.follow',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: '커서 따라가기',
    titleEn: 'Cursor follow',
    matchTexts: ['커서 따라가기', 'Cursor follow'],
    keywords: ['실시간 확대', '확대', '줌', '따라가기', '포인터', 'zoom', 'follow', 'cursor'],
  },
  {
    id: 'zoom.minimap',
    tab: 'draw',
    tabKey: 'settings.tab.draw',
    titleKo: '위치 지도 보이기',
    titleEn: 'Show minimap',
    matchTexts: ['위치 지도 보이기', 'Show minimap'],
    keywords: ['실시간 확대', '확대', '미니맵', '지도', 'minimap', 'zoom', 'map'],
  },

  // ── 사용 설명서 ──
  {
    id: 'help:capture',
    tab: 'help',
    tabKey: 'settings.tab.help',
    titleKo: '사용 설명서 · 찍기 3종',
    titleEn: 'User guide · Three capture modes',
    matchTexts: ['찍기 3종', 'Three capture modes'],
    keywords: ['사용 설명서', '가이드', '도움말', '찍기', '캡처', 'help', 'guide', 'capture'],
  },
  {
    id: 'help:draw',
    tab: 'help',
    tabKey: 'settings.tab.help',
    titleKo: '사용 설명서 · 화면에 그리기',
    titleEn: 'User guide · Draw on screen',
    matchTexts: ['화면에 그리기', 'Draw on screen'],
    keywords: ['사용 설명서', '판서', '그리기', '도움말', 'help', 'draw'],
  },
  {
    id: 'help:troubleshoot',
    tab: 'help',
    tabKey: 'settings.tab.help',
    titleKo: '사용 설명서 · 문제 해결',
    titleEn: 'User guide · Troubleshooting',
    matchTexts: ['문제 해결', 'Troubleshooting'],
    keywords: ['문제 해결', '권한', '겹침', '안 됨', '오류', 'troubleshoot', 'fix', 'help'],
  },

  // ── 정보 ──
  {
    id: 'info.about',
    tab: 'info',
    tabKey: 'settings.tab.info',
    titleKo: '정보 · 버전 · 오픈소스 고지',
    titleEn: 'About · Version · Open-source',
    matchTexts: ['버전', 'Version', '오픈소스 고지', 'Open-source notices'],
    keywords: ['정보', '버전', '만든 이', '찰떡쌤', '라이선스', '글꼴', 'about', 'version', 'license', 'credits'],
  },
];

function normalizeText(v) {
  return String(v ?? '').toLowerCase().trim();
}

/**
 * 한 항목이 토큰 목록에 얼마나 잘 맞는지 점수를 계산한다(0 = 맞지 않음).
 * @param {string} title
 * @param {string[]} haystackItems
 * @param {string[]} tokens
 * @returns {number}
 */
function scoreEntry(title, haystackItems, tokens) {
  const normTitle = normalizeText(title);
  const combined = haystackItems.map(normalizeText).filter(Boolean);
  let total = 0;

  for (const token of tokens) {
    if (normTitle === token) {
      total += 120;
    } else if (normTitle.startsWith(token)) {
      total += 90;
    } else if (normTitle.includes(token)) {
      total += 60;
    } else {
      const kwMatch = combined.find((item) => item === token || item.startsWith(token) || item.includes(token));
      if (!kwMatch) return 0;
      total += kwMatch === token ? 50 : kwMatch.startsWith(token) ? 35 : 20;
    }
  }
  return total;
}

/**
 * 설정 창 검색 결과 목록을 계산한다(R2-SET-1).
 * @param {string} query
 * @param {{
 *   lang?: string,
 *   platform?: 'windows' | 'macos' | string,
 *   shortcuts?: Array<{ id: string, label: string, combo?: string, scope?: string }>,
 *   translate?: (key: string) => string,
 *   limit?: number,
 * }} [opts]
 */
export function searchSettings(query, opts = {}) {
  const clean = normalizeText(query);
  if (!clean) return [];
  const tokens = clean.split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return [];

  const lang = String(opts.lang ?? 'ko').toLowerCase().startsWith('en') ? 'en' : 'ko';
  const platform = opts.platform === 'macos' ? 'macos' : 'windows';
  const tr = typeof opts.translate === 'function' ? opts.translate : (k) => k;
  const limit = Number(opts.limit ?? 8);

  const results = [];

  for (const item of SETTINGS_SEARCH_ITEMS) {
    if (item.platformOnly && item.platformOnly !== platform) continue;
    const title = lang === 'en' ? item.titleEn : item.titleKo;
    const tabLabel = tr(item.tabKey);
    const haystack = [item.titleKo, item.titleEn, tabLabel, ...item.matchTexts, ...item.keywords];
    const score = scoreEntry(title, haystack, tokens);
    if (score > 0) {
      results.push({
        id: item.id,
        tab: item.tab,
        tabLabel: tabLabel === item.tabKey ? item.tab : tabLabel,
        title,
        matchTexts: item.matchTexts,
        combo: '',
        score,
      });
    }
  }

  for (const row of opts.shortcuts ?? []) {
    if (!row?.id || !row?.label) continue;
    const tabLabel = tr('settings.tab.shortcuts');
    const haystack = [row.label, row.id, row.combo ?? '', '단축키', 'shortcut', 'key'];
    const score = scoreEntry(row.label, haystack, tokens);
    if (score > 0) {
      results.push({
        id: `shortcut:${row.id}`,
        tab: 'shortcuts',
        tabLabel: tabLabel === 'settings.tab.shortcuts' ? '단축키' : tabLabel,
        title: row.label,
        matchTexts: [row.label],
        combo: row.combo ?? '',
        scope: row.scope ?? 'global',
        score,
      });
    }
  }

  results.sort((a, b) => b.score - a.score || a.title.localeCompare(b.title));
  return results.slice(0, Math.max(1, limit));
}
