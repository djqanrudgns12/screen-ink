import { test } from 'node:test';
import assert from 'node:assert/strict';
import { computeTopIslandRect, rectsOverlap } from './popoverPlacement.js';
import {
  BUNDLED_FONTS,
  DEFAULT_FONT_FAMILY,
  DEFAULT_FONT_WEIGHT,
  FONT_POPOVER_HEIGHT,
  FONT_POPOVER_WIDTH,
  FONT_SETTING_MAX_BYTES,
  RECENT_FONT_MAX,
  buildFontCatalog,
  buildFontFamilyStack,
  classifyFontCategory,
  compactFontLabel,
  filterFontCatalog,
  formatCanvasFont,
  formatFontValue,
  parseFontSetting,
  parseFontValue,
  placeFontPopover,
  pushRecentFont,
  sanitizeFontName,
  serializeFontSetting,
  shouldStartTypeSearch,
  stepFontRowIndex,
  stepWeightChipIndex,
} from './fontPicker.js';

test('번들 글꼴 3종과 안전한 CSS 스택·CSS 인젝션 방어', () => {
  assert.deepEqual(BUNDLED_FONTS.map((f) => f.name), ['Freesentation', 'Paperlogy', 'IBM Plex Mono']);
  assert.equal(sanitizeFontName('  Apple SD Gothic Neo  '), 'Apple SD Gothic Neo');
  assert.equal(sanitizeFontName('.AppleSystemUIFont'), '');
  assert.equal(sanitizeFontName('Bad";color:red;{}'), 'Badcolor:red');

  const paperStack = buildFontFamilyStack('Paperlogy');
  assert.match(paperStack, /^"Paperlogy", "Freesentation",/, 'Paperlogy는 Freesentation 폴백 필수');
  assert.equal(formatCanvasFont(24, 'Freesentation', 600).startsWith('600 24px "Freesentation"'), true);
});

test('시스템 글꼴 묶음 분류·굵기 접미사 그룹화·없는 굵기는 지어내지 않음', () => {
  assert.equal(classifyFontCategory('D2Coding'), 'mono');
  assert.equal(classifyFontCategory('RIDIBatang'), 'serif');
  assert.equal(classifyFontCategory('Nanum Pen Script'), 'hand');
  assert.equal(classifyFontCategory('BM Jua'), 'round');
  assert.equal(classifyFontCategory('Black Han Sans'), 'display');
  assert.equal(classifyFontCategory('Pretendard'), 'gothic');
  assert.equal(classifyFontCategory('SomeUnknownFont'), 'mine');

  const cat = buildFontCatalog([
    '.SF NS',
    'Freesentation',
    'Pretendard Light',
    'Pretendard Medium',
    'Pretendard SemiBold',
    'Pretendard Bold',
    'Apple SD Gothic Neo',
    'D2Coding',
  ]);

  assert.equal(cat.byId.has('.sf ns'), false, '숨김 폰트는 제외');
  const pret = cat.byId.get('pretendard');
  assert.ok(pret, '굵기 접미사가 하나의 패밀리로 묶임');
  assert.deepEqual(
    pret.variants.map((v) => v.chip),
    ['L', 'M', 'SB', 'B'],
    '실제 있는 굵기만 알약으로 생성',
  );
  const apple = cat.byId.get('apple sd gothic neo');
  assert.equal(apple?.variants.length, 1, '단일 패밀리에 임의 굵기를 추가하지 않음');
});

test('탭 필터·검색(영문·한글·초성)·최근 3개 유지', () => {
  const cat = buildFontCatalog([
    'Apple SD Gothic Neo',
    'D2Coding',
    'RIDIBatang',
    '맑은 고딕',
    '나눔손글씨 펜',
    '둥근모꼴',
    'Helvetica',
    'Arial',
    'Menlo',
  ]);

  const all = filterFontCatalog(cat, {
    category: 'all',
    query: '',
    recent: ['Paperlogy:600', 'D2Coding', 'Freesentation:500', 'Arial'],
  });
  assert.equal(all.showSearch, true, '글꼴이 많을 때 검색 표시');
  assert.ok(all.tabs.includes('mine'), '시스템 글꼴이 있으면 내 글꼴 탭 표시');
  const recentSec = all.sections.find((s) => s.id === 'recent');
  assert.equal(recentSec?.rows.length, RECENT_FONT_MAX, '최근 사용은 최대 3개');

  const monoOnly = filterFontCatalog(cat, { category: 'mono', recent: ['Paperlogy:600'] });
  assert.equal(monoOnly.sections.some((s) => s.id === 'recent'), false, '특정 탭에서는 최근 묶음 숨김');
  assert.ok(monoOnly.flatRows.some((r) => r.item.name === 'IBM Plex Mono'));
  assert.ok(monoOnly.flatRows.some((r) => r.item.name === 'D2Coding'));

  const mineOnly = filterFontCatalog(cat, { category: 'mine' });
  assert.ok(mineOnly.flatRows.every((r) => r.item.source === 'system'));

  const searchKo = filterFontCatalog(cat, { query: 'ㅁㅇㄱㄷ' });
  assert.ok(searchKo.flatRows.some((r) => r.item.name === '맑은 고딕'), '한글 초성 검색');

  const searchEn = filterFontCatalog(cat, { query: 'd2c' });
  assert.equal(searchEn.flatRows[0]?.item.name, 'D2Coding');

  const smallCat = buildFontCatalog([]);
  const smallFiltered = filterFontCatalog(smallCat, {});
  assert.equal(smallFiltered.showSearch, false, '앱 글꼴 3종뿐이면 검색 숨김');
  assert.equal(smallFiltered.tabs.includes('mine'), false, '시스템 글꼴 없으면 내 글꼴 탭 숨김');
});

test('annotate.font 직렬화·파싱(200바이트 상한) 및 최근 3개 갱신', () => {
  const empty = parseFontSetting('');
  assert.deepEqual(empty, { family: DEFAULT_FONT_FAMILY, weight: DEFAULT_FONT_WEIGHT, recent: [] });

  assert.deepEqual(parseFontSetting('Paperlogy:600'), {
    family: 'Paperlogy',
    weight: 600,
    recent: [],
  });

  let recent = [];
  recent = pushRecentFont(recent, 'Freesentation', 400);
  recent = pushRecentFont(recent, 'Paperlogy', 600);
  recent = pushRecentFont(recent, 'IBM Plex Mono', 500);
  recent = pushRecentFont(recent, 'Apple SD Gothic Neo', 400);
  assert.equal(recent.length, 3);
  assert.equal(recent[0], 'Apple SD Gothic Neo');

  const saved = serializeFontSetting({
    family: 'Very Long Custom System Font Family Name '.repeat(4),
    weight: 600,
    recent: ['Another Very Long Font Family Name '.repeat(3), 'Paperlogy:600', 'Freesentation'],
  });
  assert.ok(saved.length <= FONT_SETTING_MAX_BYTES, `200바이트 이하 유지: ${saved.length}`);
  const parsed = parseFontSetting(saved);
  assert.equal(parsed.weight, 600);
  assert.equal(formatFontValue('Freesentation', 400), 'Freesentation');
  assert.equal(formatFontValue('Freesentation', 600), 'Freesentation:600');
  assert.deepEqual(parseFontValue('Paperlogy:500'), { family: 'Paperlogy', weight: 500 });
  assert.equal(compactFontLabel('Freesentation', 400), 'Freesentation');
  assert.equal(compactFontLabel('Freesentation', 600), 'Freesentation SB');
});

test('키보드 탐색(↑↓·←→·Home/End·글자 치면 검색) 및 팝오버 위 우선 배치·높이 압축', () => {
  assert.equal(stepFontRowIndex(-1, 'ArrowDown', 5), 0);
  assert.equal(stepFontRowIndex(0, 'ArrowDown', 5), 1);
  assert.equal(stepFontRowIndex(4, 'ArrowDown', 5), 4);
  assert.equal(stepFontRowIndex(2, 'ArrowUp', 5), 1);
  assert.equal(stepFontRowIndex(-1, 'ArrowUp', 5), 4);
  assert.equal(stepFontRowIndex(2, 'Home', 5), 0);
  assert.equal(stepFontRowIndex(2, 'End', 5), 4);

  assert.equal(stepWeightChipIndex(-1, 'ArrowRight', 3), 0);
  assert.equal(stepWeightChipIndex(0, 'ArrowRight', 3), 1);
  assert.equal(stepWeightChipIndex(1, 'ArrowLeft', 3), 0);
  assert.equal(stepWeightChipIndex(0, 'ArrowLeft', 3), -1);

  assert.equal(shouldStartTypeSearch({ key: 'p' }), true);
  assert.equal(shouldStartTypeSearch({ key: ' ', ctrlKey: false }), false);
  assert.equal(shouldStartTypeSearch({ key: 'a', metaKey: true }), false);
  assert.equal(shouldStartTypeSearch({ key: 'ArrowDown' }), false);

  // 위 공간 충분 → 바로 위(264×320)
  const above = placeFontPopover({ left: 400, top: 500, width: 150, height: 28 }, { w: 1280, h: 720 });
  assert.equal(above.side, 'above');
  assert.equal(above.width, FONT_POPOVER_WIDTH);
  assert.equal(above.maxHeight, FONT_POPOVER_HEIGHT);
  assert.equal(above.top, 500 - 6 - FONT_POPOVER_HEIGHT);

  // 위 공간이 작고 아래가 넓으면 아래로
  const below = placeFontPopover({ left: 1200, top: 40, width: 150, height: 28 }, { w: 1280, h: 720 });
  assert.equal(below.side, 'below');
  assert.ok(below.left + below.width <= 1280 - 8, '오른쪽 경계 클램프');

  // 섬 바깥(왼쪽)에서 위 220px(minHeight 180 이상, 아래보다 넓음) → 위 유지 + 높이 압축
  const squeezed = placeFontPopover({ left: 40, top: 234, width: 150, height: 28 }, { w: 1280, h: 420 });
  assert.equal(squeezed.side, 'above');
  assert.equal(squeezed.maxHeight, 220);

  // G19: 화면 위쪽 범위(트리거 top=348, 가운데)에서 상단 섬(560×88)과 겹치면 아래로 뒤집어 연다
  const island820 = computeTopIslandRect({ w: 1280, h: 820 });
  const flipBelow = placeFontPopover({ left: 467, top: 348, width: 260, height: 30 }, { w: 1280, h: 820 });
  assert.equal(flipBelow.side, 'below');
  assert.equal(flipBelow.maxHeight, FONT_POPOVER_HEIGHT);
  assert.equal(flipBelow.top, 348 + 30 + 6);
  assert.equal(rectsOverlap({ ...flipBelow, height: flipBelow.maxHeight }, island820), false);

  // 양쪽 모자람 + 상단 섬 겹침 구역 → 아래로 밀고 섬·화면 경계 모두 회피
  const island420 = computeTopIslandRect({ w: 1280, h: 420 });
  const bothTight = placeFontPopover({ left: 200, top: 234, width: 150, height: 28 }, { w: 1280, h: 420 });
  assert.equal(bothTight.side, 'below');
  assert.equal(bothTight.clamped, true);
  assert.equal(rectsOverlap({ ...bothTight, height: bothTight.maxHeight }, island420), false);
  assert.ok(bothTight.top + bothTight.maxHeight <= 420 - 8);

  // 두 모니터 경계(오른쪽 모니터 1920..3840)에서 경계 밖으로 나가지 않음
  const seam = placeFontPopover(
    { left: 1915, top: 500, width: 150, height: 28 },
    { left: 1920, top: 0, w: 1920, h: 1080 },
  );
  assert.equal(seam.left, 1920 + 8);
});
