// 글꼴 고르기 순수 모델(R2-TXT-1~4, 6 · Tidy식 묶음·굵기 알약·검색·최근 3개·키보드·팝오버 배치).
// 개인정보 원칙(PRD §2·R-LOG-1): 시스템 글꼴 이름은 화면 표시용으로만 쓰고 로그/외부에 남기지 않는다.

export const DEFAULT_FONT_FAMILY = 'Freesentation';
export const DEFAULT_FONT_WEIGHT = 400;
export const FONT_SETTING_MAX_BYTES = 200;
export const RECENT_FONT_MAX = 3;
export const SEARCH_THRESHOLD = 10;
export const FONT_POPOVER_WIDTH = 264;
export const FONT_POPOVER_HEIGHT = 320;
export const FONT_POPOVER_MIN_HEIGHT = 180;

/** @typedef {'all' | 'gothic' | 'serif' | 'hand' | 'round' | 'display' | 'mono' | 'mine'} FontTabId */
/** @typedef {'gothic' | 'serif' | 'hand' | 'round' | 'display' | 'mono' | 'mine'} FontCategory */

/** @type {Readonly<Record<number, string>>} */
export const WEIGHT_LABELS = Object.freeze({
  100: 'T',
  200: 'EL',
  300: 'L',
  400: 'R',
  500: 'M',
  600: 'SB',
  700: 'B',
  800: 'EB',
  900: 'Bk',
});

/** @type {ReadonlyArray<{ name: string, weight: number, chip: string, pattern: RegExp }>} */
const WEIGHT_SUFFIXES = Object.freeze([
  { name: 'ExtraLight', weight: 200, chip: 'EL', pattern: /(?:^|[\s\-_])(?:Extra\s*Light|Ultra\s*Light)$/i },
  { name: 'ExtraBold', weight: 800, chip: 'EB', pattern: /(?:^|[\s\-_])(?:Extra\s*Bold|Ultra\s*Bold)$/i },
  { name: 'SemiBold', weight: 600, chip: 'SB', pattern: /(?:^|[\s\-_])(?:Semi\s*Bold|Demi\s*Bold)$/i },
  { name: 'Thin', weight: 100, chip: 'T', pattern: /(?:^|[\s\-_])(?:Thin|Hairline)$/i },
  { name: 'Light', weight: 300, chip: 'L', pattern: /(?:^|[\s\-_])Light$/i },
  { name: 'Regular', weight: 400, chip: 'R', pattern: /(?:^|[\s\-_])(?:Regular|Normal|Roman|Book)$/i },
  { name: 'Medium', weight: 500, chip: 'M', pattern: /(?:^|[\s\-_])Medium$/i },
  { name: 'Bold', weight: 700, chip: 'B', pattern: /(?:^|[\s\-_])Bold$/i },
  { name: 'Black', weight: 900, chip: 'Bk', pattern: /(?:^|[\s\-_])(?:Black|Heavy)$/i },
]);

/**
 * @typedef {{
 *   family: string,
 *   weight: number,
 *   chip: string,
 *   label: string,
 *   cssFamily: string,
 * }} FontVariantItem
 *
 * @typedef {{
 *   id: string,
 *   name: string,
 *   category: FontCategory,
 *   source: 'app' | 'system',
 *   defaultWeight: number,
 *   variants: readonly FontVariantItem[],
 * }} FontFamilyItem
 */

/** 앱에 넣은 글꼴(tokens.css · fonts.css 기준). @type {ReadonlyArray<{ name: string, category: FontCategory, weights: number[], defaultWeight: number }>} */
export const BUNDLED_FONTS = Object.freeze([
  Object.freeze({
    name: 'Freesentation',
    category: 'gothic',
    weights: [400, 500, 600],
    defaultWeight: 400,
  }),
  Object.freeze({
    name: 'Paperlogy',
    category: 'display',
    weights: [500, 600],
    defaultWeight: 500,
  }),
  Object.freeze({
    name: 'IBM Plex Mono',
    category: 'mono',
    weights: [400, 500],
    defaultWeight: 400,
  }),
]);

/** @type {readonly FontTabId[]} */
export const FONT_CATEGORIES = Object.freeze(['all', 'gothic', 'serif', 'hand', 'round', 'display', 'mono', 'mine']);

const BUNDLED_NAMES = new Set(BUNDLED_FONTS.map((f) => f.name.toLowerCase()));

/**
 * CSS 인젝션 문자 제거 + 숨김 OS 폰트(`.`) 필터.
 * @param {unknown} name
 * @returns {string}
 */
export function sanitizeFontName(name) {
  if (typeof name !== 'string') return '';
  let out = '';
  for (const ch of name) {
    const code = ch.codePointAt(0) ?? 0;
    if (code < 0x20 || code === 0x7f) continue;
    if (ch === '"' || ch === "'" || ch === '\\' || ch === ';' || ch === '{' || ch === '}' || ch === '<' || ch === '>') {
      continue;
    }
    out += ch;
  }
  const clean = out.replace(/\s+/g, ' ').trim();
  if (!clean || clean.startsWith('.')) return '';
  return clean;
}

/**
 * 캔버스·DOM용 안전한 `font-family` 폴백 스택.
 * Paperlogy는 2,780자 제한이므로 반드시 Freesentation을 바로 뒤에 둔다.
 * @param {string} family
 * @returns {string}
 */
export function buildFontFamilyStack(family) {
  const safe = sanitizeFontName(family) || DEFAULT_FONT_FAMILY;
  if (safe === 'Paperlogy') {
    return '"Paperlogy", "Freesentation", -apple-system, BlinkMacSystemFont, "Apple SD Gothic Neo", "Segoe UI", "Malgun Gothic", sans-serif';
  }
  if (safe === 'IBM Plex Mono') {
    return '"IBM Plex Mono", "Freesentation", monospace';
  }
  if (safe === 'Freesentation') {
    return '"Freesentation", -apple-system, BlinkMacSystemFont, "Apple SD Gothic Neo", "Segoe UI", "Malgun Gothic", sans-serif';
  }
  return `"${safe}", "Freesentation", -apple-system, BlinkMacSystemFont, "Apple SD Gothic Neo", "Segoe UI", "Malgun Gothic", sans-serif`;
}

/**
 * 2D 캔버스 `ctx.font` / `document.fonts.load`용 문자열.
 * @param {number} fontSize
 * @param {string} family
 * @param {number} [weight=400]
 * @returns {string}
 */
export function formatCanvasFont(fontSize, family, weight = DEFAULT_FONT_WEIGHT) {
  const fs = Number.isFinite(fontSize) && fontSize > 0 ? fontSize : 24;
  const w = normalizeFontWeight(weight);
  return `${w} ${fs}px ${buildFontFamilyStack(family)}`;
}

/**
 * @param {unknown} weight
 * @returns {number}
 */
export function normalizeFontWeight(weight) {
  const n = Number(weight);
  if (!Number.isFinite(n) || n < 100 || n > 950) return DEFAULT_FONT_WEIGHT;
  return Math.round(n / 100) * 100;
}

/**
 * @param {string} family
 * @param {number} [weight=400]
 * @returns {string}
 */
export function formatFontValue(family, weight = DEFAULT_FONT_WEIGHT) {
  const clean = sanitizeFontName(family) || DEFAULT_FONT_FAMILY;
  const w = normalizeFontWeight(weight);
  return w === DEFAULT_FONT_WEIGHT ? clean : `${clean}:${w}`;
}

/**
 * `"Freesentation:600"` 또는 `"Paperlogy"` 파싱.
 * @param {unknown} raw
 * @returns {{ family: string, weight: number }}
 */
export function parseFontValue(raw) {
  if (typeof raw !== 'string') return { family: DEFAULT_FONT_FAMILY, weight: DEFAULT_FONT_WEIGHT };
  const trimmed = raw.trim();
  if (!trimmed) return { family: DEFAULT_FONT_FAMILY, weight: DEFAULT_FONT_WEIGHT };
  const idx = trimmed.lastIndexOf(':');
  if (idx > 0) {
    const maybeWeight = Number(trimmed.slice(idx + 1));
    if (Number.isFinite(maybeWeight) && maybeWeight >= 100 && maybeWeight <= 950) {
      const fam = sanitizeFontName(trimmed.slice(0, idx));
      if (fam) return { family: fam, weight: normalizeFontWeight(maybeWeight) };
    }
  }
  return { family: sanitizeFontName(trimmed) || DEFAULT_FONT_FAMILY, weight: DEFAULT_FONT_WEIGHT };
}

/**
 * 최근 사용 목록 갱신(중복 제거, 맨 앞, 최대 3개).
 * @param {ReadonlyArray<string>} recent
 * @param {string} family
 * @param {number} [weight]
 * @returns {string[]}
 */
export function pushRecentFont(recent, family, weight = DEFAULT_FONT_WEIGHT) {
  const clean = sanitizeFontName(family);
  if (!clean) return Array.isArray(recent) ? recent.slice(0, RECENT_FONT_MAX) : [];
  const val = formatFontValue(clean, weight);
  const out = [val];
  if (Array.isArray(recent)) {
    for (const item of recent) {
      const p = parseFontValue(item);
      const norm = formatFontValue(p.family, p.weight);
      if (p.family.toLowerCase() !== clean.toLowerCase() && norm !== val && out.length < RECENT_FONT_MAX) {
        out.push(norm);
      }
    }
  }
  return out.slice(0, RECENT_FONT_MAX);
}

/**
 * 설정 `annotate.font`(200자 제한) 파싱.
 * 빈 문자열 · 평문 `"Paperlogy"` · `"Paperlogy:600"` · JSON `{"f":...,"w":...,"r":[...]}` 모두 지원.
 * @param {unknown} raw
 * @returns {{ family: string, weight: number, recent: string[] }}
 */
export function parseFontSetting(raw) {
  const empty = { family: DEFAULT_FONT_FAMILY, weight: DEFAULT_FONT_WEIGHT, recent: [] };
  if (typeof raw !== 'string') {
    if (raw && typeof raw === 'object') {
      const rec = /** @type {Record<string, any>} */ (raw);
      const family = sanitizeFontName(rec.f ?? rec.family) || DEFAULT_FONT_FAMILY;
      const weight = normalizeFontWeight(rec.w ?? rec.weight);
      const recent = normalizeRecentList(rec.r ?? rec.recent);
      return { family, weight, recent };
    }
    return empty;
  }
  const trimmed = raw.trim();
  if (!trimmed) return empty;
  if (trimmed.startsWith('{')) {
    try {
      const obj = JSON.parse(trimmed);
      if (obj && typeof obj === 'object') {
        const family = sanitizeFontName(obj.f ?? obj.family) || DEFAULT_FONT_FAMILY;
        const weight = normalizeFontWeight(obj.w ?? obj.weight);
        const recent = normalizeRecentList(obj.r ?? obj.recent);
        return { family, weight, recent };
      }
    } catch {
      return empty;
    }
  }
  const parsed = parseFontValue(trimmed);
  return { family: parsed.family, weight: parsed.weight, recent: [] };
}

/** @param {unknown} list @returns {string[]} */
function normalizeRecentList(list) {
  if (!Array.isArray(list)) return [];
  /** @type {string[]} */
  const out = [];
  const seen = new Set();
  for (const item of list) {
    if (typeof item !== 'string' || !item.trim()) continue;
    const p = parseFontValue(item);
    const key = p.family.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(formatFontValue(p.family, p.weight));
    if (out.length >= RECENT_FONT_MAX) break;
  }
  return out;
}

/**
 * `annotate.font` 저장 문자열 생성(Rust `Text(200)` 200바이트 상한 보장).
 * @param {{ family: string, weight?: number, recent?: ReadonlyArray<string> }} state
 * @returns {string}
 */
export function serializeFontSetting(state) {
  const family = sanitizeFontName(state?.family) || DEFAULT_FONT_FAMILY;
  const weight = normalizeFontWeight(state?.weight);
  const recent = normalizeRecentList(state?.recent);
  let trimmedRecent = recent.slice(0, RECENT_FONT_MAX);
  while (trimmedRecent.length >= 0) {
    const payload = trimmedRecent.length > 0 ? { f: family, w: weight, r: trimmedRecent } : { f: family, w: weight };
    const json = JSON.stringify(payload);
    if (json.length <= FONT_SETTING_MAX_BYTES) return json;
    if (trimmedRecent.length === 0) break;
    trimmedRecent = trimmedRecent.slice(0, -1);
  }
  return JSON.stringify({ f: family.slice(0, 140), w: weight }).slice(0, FONT_SETTING_MAX_BYTES);
}

/**
 * 글꼴 이름 기반 묶음 분류.
 * @param {string} name
 * @returns {FontCategory}
 */
export function classifyFontCategory(name) {
  const lower = name.toLowerCase();
  if (/(mono|code|consolas|courier|menlo|monaco|fira\s*code|jetbrains|hack|inconsolata|d2coding|고정폭)/.test(lower)) {
    return 'mono';
  }
  if (/(myungjo|myeongjo|batang|serif|times|georgia|garamond|palatino|baskerville|gungsuh|eulyoo|명조|바탕|궁서|단정해|을유|마루\s*부리|부리)/.test(lower)) {
    return 'serif';
  }
  if (/(hand|script|brush|pen|kyobo|chalk|cursive|comic|손글씨|손멋|교보|펜|붓|필기|나눔손)/.test(lower)) {
    return 'hand';
  }
  if (/(round|rounded|gulim|cookie|jua|dongle|둥근|굴림|동글|주아|쿠키런|말랑|보글)/.test(lower)) {
    return 'round';
  }
  if (/(display|poster|title|black\s*han|jalnan|dohyeon|euljiro|ganpan|paperlogy|headline|impact|제목|간판|잘난|도현|을지로|페이퍼로지|헤드라인)/.test(lower)) {
    return 'display';
  }
  if (/(gothic|sans|pretendard|noto|freesentation|malgun|apple\s*sd|segoe|helvetica|arial|inter|roboto|suit|wanted|nanum\s*gothic|nanum\s*square|고딕|산스|돋움|나눔고딕|나눔스퀘어|맑은|프리젠테이션)/.test(lower)) {
    return 'gothic';
  }
  return 'mine';
}

/**
 * 시스템 글꼴 이름에서 굵기 접미사를 분리해 같은 집끼리 묶을 수 있도록 한다.
 * @param {string} name
 * @returns {{ baseName: string, weight: number, chip: string } | null}
 */
function splitWeightSuffix(name) {
  for (const rule of WEIGHT_SUFFIXES) {
    if (rule.pattern.test(name)) {
      const base = name.replace(rule.pattern, '').trim();
      if (base.length >= 2) {
        return { baseName: base, weight: rule.weight, chip: rule.chip };
      }
    }
  }
  return null;
}

/**
 * 번들 + 이 컴퓨터 글꼴 목록으로 카탈로그를 만든다.
 * 없는 굵기는 지어내지 않고 실제로 들어온 변형만 칩으로 붙인다.
 * @param {ReadonlyArray<string>} [systemFonts=[]]
 * @returns {{ families: FontFamilyItem[], byId: Map<string, FontFamilyItem>, totalCount: number }}
 */
export function buildFontCatalog(systemFonts = []) {
  /** @type {FontFamilyItem[]} */
  const families = [];
  /** @type {Map<string, FontFamilyItem>} */
  const byId = new Map();

  for (const b of BUNDLED_FONTS) {
    const variants = b.weights.map((w) => ({
      family: b.name,
      weight: w,
      chip: WEIGHT_LABELS[w] ?? String(w),
      label: `${b.name} ${WEIGHT_LABELS[w] ?? w}`,
      cssFamily: buildFontFamilyStack(b.name),
    }));
    /** @type {FontFamilyItem} */
    const item = {
      id: `app:${b.name.toLowerCase()}`,
      name: b.name,
      category: b.category,
      source: 'app',
      defaultWeight: b.defaultWeight,
      variants,
    };
    families.push(item);
    byId.set(b.name.toLowerCase(), item);
  }

  /** @type {Map<string, { name: string, category: FontCategory, variants: FontVariantItem[] }>} */
  const grouped = new Map();
  const seenExact = new Set();

  if (Array.isArray(systemFonts)) {
    for (const raw of systemFonts) {
      const clean = sanitizeFontName(raw);
      if (!clean) continue;
      const lower = clean.toLowerCase();
      if (BUNDLED_NAMES.has(lower) || seenExact.has(lower)) continue;
      seenExact.add(lower);

      const split = splitWeightSuffix(clean);
      const baseName = split ? split.baseName : clean;
      const baseKey = baseName.toLowerCase();
      if (BUNDLED_NAMES.has(baseKey)) continue;

      const weight = split ? split.weight : DEFAULT_FONT_WEIGHT;
      const chip = split ? split.chip : '';
      let group = grouped.get(baseKey);
      if (!group) {
        group = {
          name: baseName,
          category: classifyFontCategory(baseName),
          variants: [],
        };
        grouped.set(baseKey, group);
      }
      if (!group.variants.some((v) => v.weight === weight && v.family.toLowerCase() === lower)) {
        group.variants.push({
          family: clean,
          weight,
          chip: chip || WEIGHT_LABELS[weight] || 'R',
          label: clean,
          cssFamily: buildFontFamilyStack(clean),
        });
      }
    }
  }

  const sortedSystem = [...grouped.values()].sort((a, b) => a.name.localeCompare(b.name, 'ko'));
  for (const g of sortedSystem) {
    g.variants.sort((a, b) => a.weight - b.weight);
    const has400 = g.variants.find((v) => v.weight === 400);
    const def = has400 ?? g.variants[0];
    /** @type {FontFamilyItem} */
    const item = {
      id: `sys:${g.name.toLowerCase()}`,
      name: g.name,
      category: g.category,
      source: 'system',
      defaultWeight: def.weight,
      variants: g.variants,
    };
    families.push(item);
    byId.set(g.name.toLowerCase(), item);
    for (const v of g.variants) {
      byId.set(v.family.toLowerCase(), item);
    }
  }

  return { families, byId, totalCount: families.length };
}

// ── 한글 초성 검색 지원 ──
const CHOSEONG = [
  'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ',
  'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

/** @param {string} s */
function extractChoseong(s) {
  let out = '';
  for (const ch of s) {
    const code = ch.codePointAt(0) ?? 0;
    if (code >= 0xac00 && code <= 0xd7a3) {
      out += CHOSEONG[Math.floor((code - 0xac00) / 588)];
    } else if (!/\s/.test(ch)) {
      out += ch.toLowerCase();
    }
  }
  return out;
}

/** @param {string} s */
function isChoseongOnly(s) {
  const trimmed = s.replace(/\s+/g, '');
  return trimmed.length > 0 && [...trimmed].every((c) => CHOSEONG.includes(c));
}

/**
 * @param {FontFamilyItem} item
 * @param {string} query
 */
export function matchesFontQuery(item, query) {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  const compactQ = q.replace(/\s+/g, '');
  const haystacks = [item.name, ...item.variants.map((v) => v.family)];
  for (const h of haystacks) {
    const lower = h.toLowerCase();
    if (lower.includes(q) || lower.replace(/\s+/g, '').includes(compactQ)) return true;
    if (isChoseongOnly(compactQ) && extractChoseong(h).includes(compactQ)) return true;
  }
  return false;
}

/**
 * 카탈로그를 탭 · 검색 · 최근 3개로 필터링한다.
 * @param {{ families: FontFamilyItem[], byId: Map<string, FontFamilyItem>, totalCount: number }} catalog
 * @param {{ category?: FontTabId, query?: string, recent?: ReadonlyArray<string> }} [opts]
 */
export function filterFontCatalog(catalog, { category = 'all', query = '', recent = [] } = {}) {
  const hasSystem = catalog.families.some((f) => f.source === 'system');
  const presentCats = new Set(catalog.families.map((f) => f.category));
  /** @type {FontTabId[]} */
  const tabs = ['all'];
  for (const cat of ['gothic', 'serif', 'hand', 'round', 'display', 'mono']) {
    if (presentCats.has(/** @type {FontCategory} */ (cat))) {
      tabs.push(/** @type {FontTabId} */ (cat));
    }
  }
  if (hasSystem) tabs.push('mine');

  const trimmedQuery = typeof query === 'string' ? query.trim() : '';
  const activeTab = tabs.includes(category) ? category : 'all';

  // 최근 3개(전체 탭 & 검색어 없을 때만)
  /** @type {Array<{ key: string, item: FontFamilyItem, variantWeight: number }>} */
  const recentEntries = [];
  if (activeTab === 'all' && !trimmedQuery && Array.isArray(recent) && recent.length > 0) {
    for (const raw of recent.slice(0, RECENT_FONT_MAX)) {
      const parsed = parseFontValue(raw);
      const found = catalog.byId.get(parsed.family.toLowerCase());
      if (!found) continue;
      if (recentEntries.some((r) => r.item.id === found.id)) continue;
      recentEntries.push({
        key: `recent:${found.id}`,
        item: found,
        variantWeight: parsed.weight,
      });
    }
  }

  const matched = catalog.families.filter((item) => {
    if (trimmedQuery) return matchesFontQuery(item, trimmedQuery);
    if (activeTab === 'all') return true;
    if (activeTab === 'mine') return item.source === 'system';
    return item.category === activeTab;
  });

  const appItems = matched.filter((f) => f.source === 'app');
  const systemItems = matched.filter((f) => f.source === 'system');

  /** @type {Array<{ id: string, titleKey: string, rows: Array<{ key: string, item: FontFamilyItem, variantWeight?: number, isRecent?: boolean }> }>} */
  const sections = [];

  if (recentEntries.length > 0) {
    sections.push({
      id: 'recent',
      titleKey: 'font.section.recent',
      rows: recentEntries.map((r) => ({
        key: r.key,
        item: r.item,
        variantWeight: r.variantWeight,
        isRecent: true,
      })),
    });
  }

  if (appItems.length > 0) {
    sections.push({
      id: 'app',
      titleKey: 'font.section.app',
      rows: appItems.map((item) => ({ key: `app:${item.id}`, item })),
    });
  }

  if (systemItems.length > 0) {
    sections.push({
      id: 'mine',
      titleKey: 'font.section.mine',
      rows: systemItems.map((item) => ({ key: `sys:${item.id}`, item })),
    });
  }

  const flatRows = sections.flatMap((sec) => sec.rows);
  const showSearch = catalog.totalCount > SEARCH_THRESHOLD || trimmedQuery.length > 0;

  return {
    tabs,
    activeTab,
    sections,
    flatRows,
    showSearch,
    hasSystem,
    totalCount: catalog.totalCount,
  };
}

/**
 * 트리거 버튼용 짧은 이름(예: `Freesentation`, `Freesentation SB`, `Paperlogy M`).
 * @param {string} family
 * @param {number} [weight=400]
 * @returns {string}
 */
export function compactFontLabel(family, weight = DEFAULT_FONT_WEIGHT) {
  const clean = sanitizeFontName(family) || DEFAULT_FONT_FAMILY;
  const w = normalizeFontWeight(weight);
  const chip = WEIGHT_LABELS[w];
  if (w === DEFAULT_FONT_WEIGHT || !chip) return clean;
  return `${clean} ${chip}`;
}

/**
 * 목록 행 키보드 이동(↑↓·Home·End).
 * @param {number} currentIdx
 * @param {string} key
 * @param {number} count
 * @returns {number}
 */
export function stepFontRowIndex(currentIdx, key, count) {
  if (count <= 0) return -1;
  if (key === 'Home') return 0;
  if (key === 'End') return count - 1;
  if (key === 'ArrowDown') {
    return currentIdx < 0 ? 0 : Math.min(count - 1, currentIdx + 1);
  }
  if (key === 'ArrowUp') {
    return currentIdx < 0 ? count - 1 : Math.max(0, currentIdx - 1);
  }
  return currentIdx;
}

/**
 * 다중 굵기 칩 이동(←→). -1은 행 본문, 0..count-1은 칩 인덱스.
 * @param {number} currentChipIdx
 * @param {string} key
 * @param {number} count
 * @returns {number}
 */
export function stepWeightChipIndex(currentChipIdx, key, count) {
  if (count <= 1) return -1;
  if (key === 'ArrowRight') {
    if (currentChipIdx < 0) return 0;
    return Math.min(count - 1, currentChipIdx + 1);
  }
  if (key === 'ArrowLeft') {
    if (currentChipIdx <= 0) return -1;
    return currentChipIdx - 1;
  }
  return currentChipIdx;
}

/**
 * 팝오버가 열린 상태에서 바로 글자를 치면 검색창으로 넘길지 판정.
 * @param {{ key?: string, ctrlKey?: boolean, metaKey?: boolean, altKey?: boolean, isComposing?: boolean }} ev
 * @returns {boolean}
 */
export function shouldStartTypeSearch(ev) {
  if (!ev || ev.isComposing || ev.ctrlKey || ev.metaKey || ev.altKey) return false;
  if (typeof ev.key !== 'string' || ev.key.length !== 1) return false;
  return !/\s/.test(ev.key);
}

/**
 * 글꼴 팝오버 배치(트리거 바로 위 우선, 공간이 모자라면 아래 + 높이 압축, 화면 안 클램프).
 * @param {{ left: number, top: number, width: number, height: number }} anchorRect
 * @param {{ w: number, h: number }} viewport
 * @param {{ width?: number, height?: number, minHeight?: number, gap?: number, edge?: number }} [opts]
 * @returns {{ left: number, top: number, width: number, maxHeight: number, side: 'above' | 'below' }}
 */
export function placeFontPopover(anchorRect, viewport, opts = {}) {
  const width = opts.width ?? FONT_POPOVER_WIDTH;
  const height = opts.height ?? FONT_POPOVER_HEIGHT;
  const minHeight = opts.minHeight ?? FONT_POPOVER_MIN_HEIGHT;
  const gap = opts.gap ?? 6;
  const edge = opts.edge ?? 8;

  const vw = Math.max(width + edge * 2, viewport.w || 1280);
  const vh = Math.max(minHeight + edge * 2, viewport.h || 720);

  const availAbove = Math.max(0, anchorRect.top - gap - edge);
  const availBelow = Math.max(0, vh - edge - (anchorRect.top + anchorRect.height + gap));

  /** @type {'above' | 'below'} */
  let side = 'above';
  let maxHeight = height;

  if (availAbove >= height) {
    side = 'above';
    maxHeight = height;
  } else if (availAbove >= minHeight && availAbove >= availBelow) {
    side = 'above';
    maxHeight = availAbove;
  } else if (availBelow >= height) {
    side = 'below';
    maxHeight = height;
  } else if (availBelow >= availAbove) {
    side = 'below';
    maxHeight = Math.max(Math.min(height, availBelow), Math.min(minHeight, vh - edge * 2));
  } else {
    side = 'above';
    maxHeight = Math.max(Math.min(height, availAbove), Math.min(minHeight, vh - edge * 2));
  }

  const rawLeft = anchorRect.left;
  const maxLeft = Math.max(edge, vw - edge - width);
  const left = Math.round(Math.min(maxLeft, Math.max(edge, rawLeft)));

  const top =
    side === 'above'
      ? Math.max(edge, Math.round(anchorRect.top - gap - maxHeight))
      : Math.min(Math.max(edge, vh - edge - maxHeight), Math.round(anchorRect.top + anchorRect.height + gap));

  return { left, top, width, maxHeight: Math.round(maxHeight), side };
}
