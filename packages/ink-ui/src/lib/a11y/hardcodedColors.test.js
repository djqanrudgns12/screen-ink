// 0.4 하드코딩 색 정적 검사:
// packages/ink-ui/src/**/*.svelte 및 *.css 에서 디자인 토큰(tokens.css) 대신
// 직접 작성한 색 리터럴(#hex · rgb()/rgba() · hsl()/hsla())을 찾아 차단한다.
//
// 허용 목록:
// - src/lib/tokens.css: 디자인 토큰 원본 정의 파일
// - src/preview/**: 개발용 가짜 클라이언트 미리보기 하네스(배경 목업·샘플 HEX 포함, 실제 앱 번들 미포함)
// - 주석(HTML <!-- -->, CSS /* */, JS // · /* */)
// - SVG 내부 참조(url(#...)) 및 앵커(href="#...")
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const UI_SRC_DIR = resolve(THIS_DIR, '..', '..');

const HEX_COLOR_RE = /(?<!\{)#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{4}|[0-9a-fA-F]{3})(?![0-9a-zA-Z_-])/g;
const FUNC_COLOR_RE = /\b(?:rgba?|hsla?)\s*\([^)]*\)/gi;

/**
 * 허용 파일 여부를 판정한다.
 * @param {string} relPath UI_SRC_DIR 기준 POSIX 상대 경로
 */
export function isAllowedColorFile(relPath) {
  if (relPath === 'lib/tokens.css') return true;
  if (relPath.startsWith('preview/')) return true;
  return false;
}

/**
 * 줄 번호를 보존하기 위해 매치 구간을 공백/개행으로 치환한다.
 * @param {string} chunk
 */
function blankPreservingLines(chunk) {
  return chunk.replace(/[^\n]/g, ' ');
}

/**
 * 주석과 비색상 `#` 참조(`url(#...)`, `href="#..."`)를 공백 처리한다.
 * @param {string} src
 */
export function stripCommentsAndSvgRefs(src) {
  let out = src
    .replace(/<!--[\s\S]*?-->/g, blankPreservingLines)
    .replace(/\/\*[\s\S]*?\*\//g, blankPreservingLines);

  // <script> 블록 안 한 줄 주석(// ...) 제거 (URL 프로토콜 http://, https:// 는 제외)
  out = out.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, (block) =>
    block.replace(/(^|[^:\\])\/\/[^\n]*/g, (m, prefix) => prefix + ' '.repeat(m.length - prefix.length)),
  );

  // SVG url(#id) 및 href="#id" 앵커 참조 제외
  out = out
    .replace(/url\(\s*['"]?#[^)'"]+['"]?\s*\)/gi, blankPreservingLines)
    .replace(/\b(?:href|xlink:href)\s*=\s*['"]#[^'"]+['"]/gi, blankPreservingLines);

  return out;
}

/**
 * 단일 소스 텍스트에서 하드코딩 색 리터럴을 추출한다.
 * @param {string} src
 * @returns {Array<{ line: number, token: string, snippet: string }>}
 */
export function findHardcodedColors(src) {
  const cleaned = stripCommentsAndSvgRefs(src);
  const lines = cleaned.split('\n');
  /** @type {Array<{ line: number, token: string, snippet: string }>} */
  const hits = [];

  for (let idx = 0; idx < lines.length; idx += 1) {
    const lineText = lines[idx];
    for (const re of [HEX_COLOR_RE, FUNC_COLOR_RE]) {
      re.lastIndex = 0;
      let match;
      while ((match = re.exec(lineText)) !== null) {
        hits.push({
          line: idx + 1,
          token: match[0],
          snippet: lineText.trim().slice(0, 90),
        });
      }
    }
  }
  return hits;
}

/**
 * @param {string} dir
 * @param {string[]} [acc]
 */
function collectStyleAndComponentFiles(dir, acc = []) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      collectStyleAndComponentFiles(full, acc);
    } else if (name.endsWith('.svelte') || name.endsWith('.css')) {
      acc.push(full);
    }
  }
  return acc.sort();
}

test('하드코딩 색 검사기가 주석·토큰·SVG 참조는 허용하고 #hex·rgb()·hsl() 리터럴은 적발한다', () => {
  const cleanSample = `
    <script>
      // #123456 은 주석이라 통과
      const ring = { color: 'var(--ink-pen-W)' };
    </script>
    <!-- #FFFFFF 주석 -->
    <svg><rect fill="url(#grad0)" /></svg>
    <style>
      /* rgba(0, 0, 0, 0.5) CSS 주석 */
      .ok {
        color: var(--ink-text);
        background: color-mix(in srgb, var(--ink-ok) 15%, transparent);
      }
    </style>
  `;
  assert.deepEqual(findHardcodedColors(cleanSample), []);

  const dirtySample = `
    <script>
      const c = '#FF9F0A';
    </script>
    <div style="border-color: hsl(24, 100%, 50%)"></div>
    <style>
      .bad {
        color: #f7f8fa;
        background: rgba(16, 17, 20, 0.88);
        box-shadow: 0 0 0 1px rgb(0, 0, 0);
      }
    </style>
  `;
  const found = findHardcodedColors(dirtySample);
  assert.deepEqual(
    found.map((f) => f.token),
    ['#FF9F0A', 'hsl(24, 100%, 50%)', '#f7f8fa', 'rgba(16, 17, 20, 0.88)', 'rgb(0, 0, 0)'],
  );
});

test('packages/ink-ui/src/**/*.svelte·*.css 에 토큰 밖 하드코딩 색 리터럴이 0건이다', () => {
  const files = collectStyleAndComponentFiles(UI_SRC_DIR);
  assert.ok(files.length > 0, '검사 대상 .svelte/.css 파일이 없습니다');

  /** @type {string[]} */
  const violations = [];

  for (const full of files) {
    const rel = relative(UI_SRC_DIR, full).replace(/\\/g, '/');
    if (isAllowedColorFile(rel)) continue;
    const src = readFileSync(full, 'utf8');
    const hits = findHardcodedColors(src);
    for (const h of hits) {
      violations.push(`${rel}:${h.line} [${h.token}] ${h.snippet}`);
    }
  }

  assert.deepEqual(
    violations,
    [],
    `토큰 대신 하드코딩된 색 리터럴이 발견됐습니다:\n${violations.join('\n')}`,
  );
});
