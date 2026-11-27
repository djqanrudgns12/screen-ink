// 접근성 정적 검사(R2-A11Y-1):
// 1. 모든 .svelte 파일의 <button> / <Dialog.Close> 에 이름표(aria-label / aria-labelledby 또는 가시 글자)가 있는지 검증
// 2. tokens.css 전역 초점 안전망과 관리 대상 컴포넌트의 :focus-visible / ink-focus 적용 검증
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const UI_LIB_DIR = resolve(THIS_DIR, '..');
const TOKENS_PATH = join(UI_LIB_DIR, 'tokens.css');

/**
 * Svelte 마크업에서 HTML 주석·`<script>`·`<style>` 을 제거한 마크업 조각을 얻는다.
 * @param {string} src
 * @returns {string}
 */
function extractMarkup(src) {
  return src
    .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, (m) => m.replace(/[^\n]/g, ' '))
    .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, (m) => m.replace(/[^\n]/g, ' '))
    .replace(/<!--[\s\S]*?-->/g, (m) => m.replace(/[^\n]/g, ' '));
}

/**
 * 속성 문자열 내부의 따옴표와 `{ ... }` 표현식 중첩을 고려해 여는 태그 끝 `>` 인덱스를 찾는다.
 * (화살표 함수 `=>` 의 `>` 에서 태그가 조기 종료되는 것을 막는다.)
 * @param {string} text
 * @param {number} startAfterTagName
 * @returns {{ closeBracket: number, selfClosing: boolean } | null}
 */
function findTagEnd(text, startAfterTagName) {
  let i = startAfterTagName;
  let braceDepth = 0;
  /** @type {string | null} */
  let quote = null;
  while (i < text.length) {
    const ch = text[i];
    if (quote) {
      if (ch === '\\') {
        i += 2;
        continue;
      }
      if (ch === quote) quote = null;
      i += 1;
      continue;
    }
    if (ch === '"' || ch === "'" || ch === '`') {
      quote = ch;
      i += 1;
      continue;
    }
    if (ch === '{') {
      braceDepth += 1;
      i += 1;
      continue;
    }
    if (ch === '}') {
      braceDepth = Math.max(0, braceDepth - 1);
      i += 1;
      continue;
    }
    if (ch === '>' && braceDepth === 0) {
      const prev = i > 0 ? text[i - 1] : '';
      return { closeBracket: i, selfClosing: prev === '/' };
    }
    i += 1;
  }
  return null;
}

/**
 * 마크업 문자열에서 지정한 태그(`<button>`, `<Dialog.Close>`)를 모두 추출한다.
 * @param {string} src
 * @returns {Array<{ tag: string, attrs: string, body: string, line: number }>}
 */
export function extractInteractiveButtons(src) {
  const markup = extractMarkup(src);
  /** @type {Array<{ tag: string, attrs: string, body: string, line: number }>} */
  const out = [];
  const openRe = /<(button|Dialog\.Close)(?=[\s/>])/g;
  let match;
  while ((match = openRe.exec(markup)) !== null) {
    const tag = match[1];
    const startIdx = match.index;
    const tagEnd = findTagEnd(markup, startIdx + 1 + tag.length);
    if (!tagEnd) continue;
    const attrs = markup.slice(startIdx + 1 + tag.length, tagEnd.selfClosing ? tagEnd.closeBracket - 1 : tagEnd.closeBracket);
    let body = '';
    if (!tagEnd.selfClosing) {
      const closeTag = `</${tag}>`;
      const closeIdx = markup.indexOf(closeTag, tagEnd.closeBracket + 1);
      if (closeIdx === -1) continue;
      body = markup.slice(tagEnd.closeBracket + 1, closeIdx);
    }
    const line = markup.slice(0, startIdx).split('\n').length;
    out.push({ tag, attrs, body, line });
  }
  return out;
}

/**
 * 버튼에 스크린 리더 이름표(aria-label / aria-labelledby) 또는 의미 있는 가시 글자가 있는지 판정한다.
 * 아이콘/장식 요소만 있고 이름표가 없으면 false를 반환한다.
 * @param {{ attrs: string, body: string }} btn
 * @returns {boolean}
 */
export function hasAccessibleButtonName(btn) {
  if (/\baria-label\s*=|\baria-labelledby\s*=/.test(btn.attrs)) {
    return true;
  }
  const visible = btn.body
    .replace(/<svg\b[\s\S]*?<\/svg>/gi, ' ')
    .replace(/<(?:Icon|AnnotateGlyph|AppIcon|Keycap)\b[^>]*\/?>/g, ' ')
    .replace(/<span\b[^>]*aria-hidden\s*=\s*["']true["'][^>]*>[\s\S]*?<\/span>/gi, ' ')
    .replace(/<span\b[^>]*class="[^"]*(?:dot|swatch|rainbow|icon|line|cell|caret|ring)[^"]*"[^>]*>[\s\S]*?<\/span>/gi, ' ')
    .replace(/<[^>]+>/g, ' ')
    .replace(/\{[#/:@][^}]*\}/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

  if (!visible) return false;
  if (/^[×✕✕+\-•·▾▴▸◂]$/.test(visible)) return false;
  return true;
}

/**
 * @param {string} dir
 * @param {string[]} [acc]
 */
function collectSvelteFiles(dir, acc = []) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      collectSvelteFiles(full, acc);
    } else if (name.endsWith('.svelte')) {
      acc.push(full);
    }
  }
  return acc.sort();
}

test('접근성 이름표 정적 검사기가 아이콘 전용 단추 누락을 정확히 잡아낸다', () => {
  const bad = extractInteractiveButtons('<button type="button" onclick={() => x()}><Icon name="close" /></button>');
  assert.equal(bad.length, 1);
  assert.equal(hasAccessibleButtonName(bad[0]), false);

  const badSymbol = extractInteractiveButtons('<button type="button">×</button>');
  assert.equal(hasAccessibleButtonName(badSymbol[0]), false);

  const goodAria = extractInteractiveButtons('<button type="button" aria-label={t("close")} onclick={() => x()}><Icon name="close" /></button>');
  assert.equal(hasAccessibleButtonName(goodAria[0]), true);

  const goodText = extractInteractiveButtons('<button type="button"><Icon name="save" /><span>저장</span></button>');
  assert.equal(hasAccessibleButtonName(goodText[0]), true);
});

test('모든 .svelte 화면·부품의 버튼과 닫기 단추에 이름표(aria-label 또는 보이는 글)가 있다', () => {
  const files = collectSvelteFiles(UI_LIB_DIR);
  assert.ok(files.length > 0);
  const violations = [];

  for (const full of files) {
    const src = readFileSync(full, 'utf8');
    const rel = relative(UI_LIB_DIR, full).replace(/\\/g, '/');
    const buttons = extractInteractiveButtons(src);
    for (const btn of buttons) {
      if (!hasAccessibleButtonName(btn)) {
        violations.push(`${rel}:${btn.line} <${btn.tag}> 이름표 없음`);
      }
    }
  }

  assert.deepEqual(violations, [], `접근성 이름표 빠진 단추:\n${violations.join('\n')}`);
});

test('전역 초점 테두리(:focus-visible) 안전망과 주요 컴포넌트 초점 규칙이 정의되어 있다', () => {
  const tokens = readFileSync(TOKENS_PATH, 'utf8');
  assert.match(tokens, /\.ink-focus:focus-visible\s*\{/);
  assert.match(tokens, /:where\([^)]*button[^)]*\):focus-visible\s*\{/);

  const managed = [
    'components/ColorWidthPanel.svelte',
    'components/FontPicker.svelte',
    'components/CaptureToolbar.svelte',
    'components/DrawToolbar.svelte',
    'screens/FirstRunSheet.svelte',
    'screens/ShortcutsPage.svelte',
  ];

  for (const rel of managed) {
    const src = readFileSync(join(UI_LIB_DIR, rel), 'utf8');
    const buttons = extractInteractiveButtons(src);
    const hasFileFocusCss = /:focus-visible/.test(src);
    for (const btn of buttons) {
      const hasInkFocus = /\bink-focus\b/.test(btn.attrs);
      assert.ok(
        hasInkFocus || hasFileFocusCss,
        `${rel}:${btn.line} 버튼에 .ink-focus 또는 :focus-visible 스타일이 없습니다`,
      );
    }
  }
});
