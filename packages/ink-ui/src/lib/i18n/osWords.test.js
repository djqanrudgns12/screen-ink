// OS 표기 자동 검사(R2-OSFIT-3): 맥 판에 Ctrl·작업 표시줄 등, Windows 판에 ⌘·메뉴 막대 등이 섞이면 실패.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { CATALOGS, LANGS, translate } from './core.js';

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const UI_LIB_DIR = resolve(THIS_DIR, '..');

/** Windows 전용 표기 — `~mac` 변형 및 기본 키(예외 제외)에 나오면 실패. */
const WIN_ONLY_TERMS = Object.freeze([
  { name: 'Ctrl', re: /\bCtrl\b/i },
  { name: 'Alt', re: /\bAlt\b/i },
  { name: 'Win', re: /\bWin\b/ },
  { name: '.exe', re: /\.exe\b/i },
  { name: '작업 표시줄', re: /작업\s*표시줄/ },
  { name: '탐색기', re: /탐색기/ },
  { name: 'Taskbar', re: /\bTaskbar\b/i },
  { name: 'File Explorer', re: /\b(?:File\s+)?Explorer\b/i },
]);

/** macOS 전용 표기 — `~win` 변형 및 기본 키(예외 제외)에 나오면 실패. */
const MAC_ONLY_TERMS = Object.freeze([
  { name: '⌘', re: /⌘/ },
  { name: '⌥', re: /⌥/ },
  { name: '⌃', re: /⌃/ },
  { name: 'Cmd', re: /\bCmd\b/i },
  { name: 'Option', re: /\bOption\b/ },
  { name: '메뉴 막대', re: /메뉴\s*막대/ },
  { name: 'Finder', re: /\bFinder\b/ },
  { name: '시스템 설정', re: /시스템\s*설정/ },
  { name: 'Menu bar', re: /\bMenu\s+bar\b/i },
  { name: 'System Settings', re: /\bSystem\s+Settings\b/i },
]);

/**
 * 기본 키(변형 없는 글)에 OS 낱말이 허용되는 예외 목록(키 → 까닭 한 줄).
 * 두 OS 모두 그 낱말이 맞거나, OS 전용 분기에서만 직접 호출되는 키만 적는다.
 * @type {Readonly<Record<string, string>>}
 */
const BASE_KEY_ALLOWLIST = Object.freeze({
  'shortcut.rule.macOptionChar': '맥 전용 단축키 규칙(Rust shortcuts.rs의 Platform::Mac 분기에서만 tr()로 호출)',
});

/**
 * 화면 소스에서 OS 키·용어 리터럴을 허용하는 정당한 파일 목록(src/lib 기준 상대 경로 → 까닭 한 줄).
 * 단축키 표기 함수·키 표 데이터·비표시 검색 동의어 사전 등만 적는다.
 * @type {Readonly<Record<string, string>>}
 */
const SOURCE_ALLOWLIST = Object.freeze({
  'models/settingsSearch.js': '설정 검색용 비표시 동의어 사전(화면에 출력되지 않고 검색어 매칭에만 사용)',
  'pdf/layoutModel.js': 'PDF 키캡 변환용 OS별 키 표 데이터(MAC_KEY_MAP·WIN_KEY_MAP)',
  'pdf/renderPdf.js': 'PDF 글꼴 분기용 기호 문자 집합(SYMBOL_CHARS)',
});

/** 화면 소스(비주석)에서 금지할 OS 키·용어 패턴. */
const SOURCE_FORBIDDEN_PATTERNS = Object.freeze([
  {
    name: '하드코딩된 OS 키 조합(Ctrl+/Alt+/Cmd+/Option+/Win+)',
    re: /['"`](?:[^'"`\n]*\b)?(?:Ctrl|Alt|Cmd|Option|Win)\+/g,
  },
  {
    name: '하드코딩된 공백 구분 OS 키 조합(Ctrl Z / Alt Shift 1 등)',
    re: /['"`](?:Ctrl|Alt|Cmd|Option|Win)\s+(?:Shift\s+)?[A-Z0-9,./]/g,
  },
  {
    name: '하드코딩된 macOS 수식키 기호(⌘/⌥/⌃)',
    re: /[⌘⌥⌃]/g,
  },
  {
    name: '하드코딩된 OS 화면 용어(작업 표시줄/메뉴 막대/시스템 설정/탐색기/Finder/Taskbar 등)',
    re: /작업\s*표시줄|메뉴\s*막대|시스템\s*설정|탐색기|\bTaskbar\b|\bFile\s+Explorer\b|\bMenu\s+bar\b|\bSystem\s+Settings\b|\bFinder\b/gi,
  },
]);

/**
 * @param {string} text
 * @param {ReadonlyArray<{ name: string, re: RegExp }>} terms
 * @returns {string[]}
 */
function findTermHits(text, terms) {
  const hits = [];
  for (const term of terms) {
    if (term.re.test(text)) hits.push(term.name);
  }
  return hits;
}

/**
 * JS·HTML 주석(`//`, `/* *\/`, `<!-- -->`)을 줄 번호를 유지한 채 공백으로 치환한다.
 * 문자열 리터럴 안의 내용은 건드리지 않는다.
 * @param {string} code
 * @returns {string}
 */
function stripCommentsPreserveLines(code) {
  let out = '';
  let i = 0;
  const n = code.length;
  while (i < n) {
    const c = code[i];
    const next = code[i + 1];
    if (c === '<' && code.slice(i, i + 4) === '<!--') {
      const end = code.indexOf('-->', i + 4);
      if (end === -1) break;
      out += code.slice(i, end + 3).replace(/[^\n]/g, ' ');
      i = end + 3;
      continue;
    }
    if (c === '/' && next === '/' && (i === 0 || code[i - 1] !== ':')) {
      const end = code.indexOf('\n', i + 2);
      if (end === -1) break;
      out += ' '.repeat(end - i);
      i = end;
      continue;
    }
    if (c === '/' && next === '*') {
      const end = code.indexOf('*/', i + 2);
      if (end === -1) break;
      out += code.slice(i, end + 2).replace(/[^\n]/g, ' ');
      i = end + 2;
      continue;
    }
    if (c === "'" || c === '"' || c === '`') {
      const quote = c;
      out += c;
      i += 1;
      while (i < n) {
        const sc = code[i];
        // 작은따옴표·큰따옴표 문자열은 줄바꿈을 넘길 수 없음(정규식 리터럴 안의 따옴표로 주석 파싱이 밀리는 것 방지)
        if (sc === '\n' && quote !== '`') {
          out += sc;
          i += 1;
          break;
        }
        out += sc;
        if (sc === '\\' && i + 1 < n) {
          out += code[i + 1];
          i += 2;
          continue;
        }
        if (sc === quote) {
          i += 1;
          break;
        }
        i += 1;
      }
      continue;
    }
    out += c;
    i += 1;
  }
  return out;
}

/**
 * `packages/ink-ui/src/lib` 아래 검사 대상 화면 소스 파일 목록(시험·icons/·i18n/ 제외).
 * @param {string} dir
 * @param {string[]} [acc]
 * @returns {string[]}
 */
function collectSourceFiles(dir, acc = []) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      if (name === 'icons' || name === 'i18n' || name === 'preview') continue;
      collectSourceFiles(full, acc);
    } else if (/\.(svelte|js)$/.test(name) && !/\.(test|spec)\.js$/.test(name)) {
      acc.push(full);
    }
  }
  return acc.sort();
}

test('카탈로그 ~mac 글에 Windows 전용 표기가 없다', () => {
  const violations = [];
  for (const lang of LANGS) {
    for (const [key, val] of Object.entries(CATALOGS[lang])) {
      if (!key.endsWith('~mac')) continue;
      const hits = findTermHits(val, WIN_ONLY_TERMS);
      if (hits.length > 0) {
        violations.push(`${lang}:${key} [${hits.join(', ')}] → ${JSON.stringify(val)}`);
      }
    }
  }
  assert.deepEqual(violations, [], `~mac 카탈로그에 Windows 표기 섞임:\n${violations.join('\n')}`);
});

test('카탈로그 ~win 글에 macOS 전용 표기가 없다', () => {
  const violations = [];
  for (const lang of LANGS) {
    for (const [key, val] of Object.entries(CATALOGS[lang])) {
      if (!key.endsWith('~win')) continue;
      const hits = findTermHits(val, MAC_ONLY_TERMS);
      if (hits.length > 0) {
        violations.push(`${lang}:${key} [${hits.join(', ')}] → ${JSON.stringify(val)}`);
      }
    }
  }
  assert.deepEqual(violations, [], `~win 카탈로그에 macOS 표기 섞임:\n${violations.join('\n')}`);
});

test('카탈로그 기본 키(변형 없는 글)에 OS 전용 표기가 없다(예외 목록 제외)', () => {
  const allTerms = [...WIN_ONLY_TERMS, ...MAC_ONLY_TERMS];
  const violations = [];
  for (const lang of LANGS) {
    for (const [key, val] of Object.entries(CATALOGS[lang])) {
      if (key.includes('~')) continue;
      if (Object.prototype.hasOwnProperty.call(BASE_KEY_ALLOWLIST, key)) continue;
      const hits = findTermHits(val, allTerms);
      if (hits.length > 0) {
        violations.push(`${lang}:${key} [${hits.join(', ')}] → ${JSON.stringify(val)}`);
      }
    }
  }
  assert.deepEqual(
    violations,
    [],
    `기본 키에 OS 전용 표기가 남아 있음(~mac/~win 분리 또는 중립 문구 필요):\n${violations.join('\n')}`,
  );
});

test('모든 기본 키를 mac/win으로 해석했을 때 반대 OS 표기가 나오지 않는다', () => {
  const violations = [];
  const baseKeys = Object.keys(CATALOGS.ko).filter((k) => !k.includes('~'));
  for (const lang of LANGS) {
    for (const key of baseKeys) {
      if (Object.prototype.hasOwnProperty.call(BASE_KEY_ALLOWLIST, key)) continue;
      const macText = translate(lang, key, undefined, 'mac');
      const winHitsOnMac = findTermHits(macText, WIN_ONLY_TERMS);
      if (winHitsOnMac.length > 0) {
        violations.push(`mac(${lang}):${key} [${winHitsOnMac.join(', ')}] → ${JSON.stringify(macText)}`);
      }
      const winText = translate(lang, key, undefined, 'win');
      const macHitsOnWin = findTermHits(winText, MAC_ONLY_TERMS);
      if (macHitsOnWin.length > 0) {
        violations.push(`win(${lang}):${key} [${macHitsOnWin.join(', ')}] → ${JSON.stringify(winText)}`);
      }
    }
  }
  assert.deepEqual(violations, [], `OS별 번역 결과에 반대 OS 표기 섞임:\n${violations.join('\n')}`);
});

test('화면 소스에 OS 키·용어가 직접 박혀 있지 않다(정당한 예외 목록 제외)', () => {
  const files = collectSourceFiles(UI_LIB_DIR);
  assert.ok(files.length > 0, '검사할 화면 소스 파일이 있어야 함');

  const violations = [];
  for (const full of files) {
    const rel = relative(UI_LIB_DIR, full).replace(/\\/g, '/');
    if (Object.prototype.hasOwnProperty.call(SOURCE_ALLOWLIST, rel)) continue;

    const raw = readFileSync(full, 'utf8');
    const stripped = stripCommentsPreserveLines(raw);
    const rawLines = raw.split('\n');
    const lines = stripped.split('\n');

    for (let i = 0; i < lines.length; i += 1) {
      const line = lines[i];
      for (const pat of SOURCE_FORBIDDEN_PATTERNS) {
        pat.re.lastIndex = 0;
        const hits = [...line.matchAll(pat.re)].map((m) => m[0]);
        if (hits.length > 0) {
          violations.push(`${rel}:${i + 1} [${hits.join(', ')}] (${pat.name}) → ${rawLines[i].trim()}`);
        }
      }
    }
  }
  assert.deepEqual(
    violations,
    [],
    `화면 소스에 OS 키·용어가 직접 박혀 있음(defaults-v1/shortcutDisplay/i18n 경로 사용 필요):\n${violations.join('\n')}`,
  );
});

test('예외 목록의 모든 항목이 실제로 존재하고 까닭이 적혀 있다', () => {
  for (const [key, reason] of Object.entries(BASE_KEY_ALLOWLIST)) {
    assert.ok(key in CATALOGS.ko, `BASE_KEY_ALLOWLIST에 없는 키: ${key}`);
    assert.ok(typeof reason === 'string' && reason.trim().length > 0, `${key} 예외 까닭 누락`);
  }
  for (const [rel, reason] of Object.entries(SOURCE_ALLOWLIST)) {
    const full = join(UI_LIB_DIR, rel);
    assert.ok(statSync(full).isFile(), `SOURCE_ALLOWLIST에 없는 파일: ${rel}`);
    assert.ok(typeof reason === 'string' && reason.trim().length > 0, `${rel} 예외 까닭 누락`);
  }
});
