// 디자인 토큰(라이트·다크) 색 대비 자동 검사(R2-A11Y-2): 글자/배경 ≥ 4.5:1, 아이콘·UI/배경 ≥ 3.0:1.
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = dirname(fileURLToPath(import.meta.url));
export const DEFAULT_TOKENS_PATH = join(ROOT, '..', 'packages', 'ink-ui', 'src', 'lib', 'tokens.css');

/**
 * #RGB / #RRGGBB / rgb() / rgba() 문자열을 [r, g, b, a] 튜플로 해석한다.
 * @param {string} raw
 * @returns {[number, number, number, number] | null}
 */
export function parseColor(raw) {
  if (!raw) return null;
  const s = raw.trim();
  const hex6 = s.match(/^#([0-9a-f]{6})$/i);
  if (hex6) {
    const n = parseInt(hex6[1], 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255, 1];
  }
  const hex3 = s.match(/^#([0-9a-f]{3})$/i);
  if (hex3) {
    const [r, g, b] = hex3[1].split('').map((c) => parseInt(c + c, 16));
    return [r, g, b, 1];
  }
  const rgb = s.match(/^rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)(?:\s*,\s*([\d.]+))?\s*\)$/i);
  if (rgb) {
    return [
      Number(rgb[1]),
      Number(rgb[2]),
      Number(rgb[3]),
      rgb[4] === undefined ? 1 : Number(rgb[4]),
    ];
  }
  return null;
}

/**
 * 반투명 전경색을 불투명 배경색 위에 알파 합성한다.
 * @param {[number, number, number, number]} fg
 * @param {[number, number, number, number]} bg
 * @returns {[number, number, number, number]}
 */
export function compositeOver(fg, bg) {
  const a = Math.max(0, Math.min(1, fg[3]));
  return [
    fg[0] * a + bg[0] * (1 - a),
    fg[1] * a + bg[1] * (1 - a),
    fg[2] * a + bg[2] * (1 - a),
    1,
  ];
}

/**
 * WCAG 2.1 상대 휘도(0..1)를 계산한다.
 * @param {[number, number, number, number] | [number, number, number]} rgb
 * @returns {number}
 */
export function relativeLuminance(rgb) {
  const linear = (c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * linear(rgb[0]) + 0.7152 * linear(rgb[1]) + 0.0722 * linear(rgb[2]);
}

/**
 * 두 색의 WCAG 2.1 대비비(1..21)를 계산한다.
 * @param {[number, number, number, number] | [number, number, number]} fg
 * @param {[number, number, number, number] | [number, number, number]} bg
 * @returns {number}
 */
export function contrastRatio(fg, bg) {
  const l1 = relativeLuminance(fg);
  const l2 = relativeLuminance(bg);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

/**
 * CSS 블록 본문에서 `--token: value;` 선언을 추출한다.
 * @param {string} block
 * @returns {Record<string, string>}
 */
function extractDeclarations(block) {
  /** @type {Record<string, string>} */
  const vars = {};
  for (const m of block.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/gi)) {
    vars[m[1]] = m[2].trim();
  }
  return vars;
}

/**
 * `var(--other)` 참조를 최종 색 문자열로 푼다(순환 방어 포함).
 * @param {Record<string, string>} vars
 * @param {string} name
 * @returns {string | null}
 */
export function resolveTokenValue(vars, name) {
  let cur = vars[name];
  const seen = new Set([name]);
  while (cur) {
    const ref = cur.match(/^var\(\s*(--[a-z0-9-]+)\s*\)$/i);
    if (!ref) return cur;
    if (seen.has(ref[1])) return null;
    seen.add(ref[1]);
    cur = vars[ref[1]];
  }
  return null;
}

/**
 * tokens.css에서 light / dark / prefers-dark 변수 맵을 파싱한다.
 * @param {string} cssText
 */
export function parseTokenThemes(cssText) {
  const lightMatch = cssText.match(/:root,\s*\[data-theme='light'\]\s*\{([\s\S]*?)\n\}/);
  const darkMatch = cssText.match(/\[data-theme='dark'\]\s*\{([\s\S]*?)\n\}/);
  const mediaDarkMatch = cssText.match(
    /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{\s*:root:not\(\[data-theme='light'\]\)\s*\{([\s\S]*?)\n\s*\}\s*\}/,
  );
  if (!lightMatch || !darkMatch) {
    throw new Error('tokens.css에서 light 또는 dark 토큰 블록을 찾지 못했습니다.');
  }

  const light = extractDeclarations(lightMatch[1]);
  const darkRaw = extractDeclarations(darkMatch[1]);
  const dark = { ...light, ...darkRaw };
  const mediaDarkRaw = mediaDarkMatch ? extractDeclarations(mediaDarkMatch[1]) : {};
  const mediaDark = { ...light, ...mediaDarkRaw };
  return { light, dark, mediaDark, darkRaw, mediaDarkRaw };
}

/**
 * 검증 대상 전경/배경 토큰 쌍.
 * kind: 'text' (≥ 4.5:1) | 'ui' (≥ 3.0:1)
 * @type {ReadonlyArray<{ fg: string, bg: string, under?: string, min: number, kind: 'text' | 'ui' }>}
 */
export const CONTRAST_PAIRS = Object.freeze([
  // 글자 / 배경 (4.5:1)
  ...['--ink-bg', '--ink-surface', '--ink-sidebar', '--ink-raised', '--ink-group-bg', '--ink-window-bg', '--ink-btn-sec-bg', '--ink-key-bg', '--ink-chip-bg', '--ink-input-bg'].map(
    (bg) => ({ fg: '--ink-text', bg, min: 4.5, kind: /** @type {const} */ ('text') }),
  ),
  ...['--ink-bg', '--ink-surface', '--ink-sidebar', '--ink-raised', '--ink-group-bg', '--ink-window-bg'].map(
    (bg) => ({ fg: '--ink-text-2', bg, min: 4.5, kind: /** @type {const} */ ('text') }),
  ),
  ...['--ink-bg', '--ink-surface', '--ink-sidebar', '--ink-raised', '--ink-group-bg', '--ink-window-bg', '--ink-chip-bg', '--ink-key-bg'].map(
    (bg) => ({ fg: '--ink-sub', bg, min: 4.5, kind: /** @type {const} */ ('text') }),
  ),
  { fg: '--ink-on-invert', bg: '--ink-invert', min: 4.5, kind: 'text' },
  { fg: '--ink-nav-on-text', bg: '--ink-nav-on-bg', min: 4.5, kind: 'text' },
  { fg: '--ink-accent-text', bg: '--ink-surface', min: 4.5, kind: 'text' },
  { fg: '--ink-accent-text', bg: '--ink-group-bg', min: 4.5, kind: 'text' },
  { fg: '--ink-accent-text', bg: '--ink-raised', min: 4.5, kind: 'text' },
  { fg: '--ink-accent-text', bg: '--ink-bg', min: 4.5, kind: 'text' },
  { fg: '--ink-accent-text', bg: '--ink-accent-soft', under: '--ink-surface', min: 4.5, kind: 'text' },
  { fg: '--ink-accent-text', bg: '--ink-rec-bg', under: '--ink-group-bg', min: 4.5, kind: 'text' },
  { fg: '--ink-key-on-invert-text', bg: '--ink-key-on-invert-bg', min: 4.5, kind: 'text' },
  { fg: '--ink-key-on-invert-text', bg: '--ink-invert', min: 4.5, kind: 'text' },
  { fg: '--ink-danger-text', bg: '--ink-surface', min: 4.5, kind: 'text' },
  { fg: '--ink-danger-text', bg: '--ink-group-bg', min: 4.5, kind: 'text' },
  { fg: '--ink-danger-text', bg: '--ink-bg', min: 4.5, kind: 'text' },
  { fg: '--ink-island-text', bg: '--ink-island', min: 4.5, kind: 'text' },
  { fg: '--ink-island-text-2', bg: '--ink-island', min: 4.5, kind: 'text' },
  { fg: '--ink-island-sub', bg: '--ink-island', min: 4.5, kind: 'text' },
  { fg: '--ink-island-accent-text', bg: '--ink-island', min: 4.5, kind: 'text' },
  { fg: '--ink-island-text', bg: '--ink-island-muted', min: 4.5, kind: 'text' },
  { fg: '--ink-island-text-2', bg: '--ink-island-muted', min: 4.5, kind: 'text' },
  { fg: '--ink-island-sub', bg: '--ink-island-muted', min: 4.5, kind: 'text' },
  { fg: '--ink-island-text', bg: '--ink-island-key-bg', min: 4.5, kind: 'text' },
  { fg: '--ink-island-btn-text', bg: '--ink-island-btn-bg', min: 4.5, kind: 'text' },

  // 아이콘 · 초점 테두리 · UI 요소 / 배경 (3.0:1)
  ...['--ink-surface', '--ink-group-bg', '--ink-raised', '--ink-bg', '--ink-sidebar'].map(
    (bg) => ({ fg: '--ink-accent-icon', bg, min: 3.0, kind: /** @type {const} */ ('ui') }),
  ),
  ...['--ink-surface', '--ink-group-bg', '--ink-raised', '--ink-bg', '--ink-sidebar'].map(
    (bg) => ({ fg: '--ink-focus-ring', bg, min: 3.0, kind: /** @type {const} */ ('ui') }),
  ),
  { fg: '--ink-attn', bg: '--ink-surface', min: 3.0, kind: 'ui' },
  { fg: '--ink-attn', bg: '--ink-group-bg', min: 3.0, kind: 'ui' },
  { fg: '--ink-attn', bg: '--ink-attn-bg', under: '--ink-surface', min: 3.0, kind: 'ui' },
  { fg: '--ink-ok', bg: '--ink-surface', min: 3.0, kind: 'ui' },
  { fg: '--ink-ok', bg: '--ink-group-bg', min: 3.0, kind: 'ui' },
  { fg: '--ink-ok', bg: '--ink-raised', min: 3.0, kind: 'ui' },
  { fg: '--ink-ok', bg: '--ink-bg', min: 3.0, kind: 'ui' },
  { fg: '--ink-danger', bg: '--ink-surface', min: 3.0, kind: 'ui' },
  { fg: '--ink-danger', bg: '--ink-group-bg', min: 3.0, kind: 'ui' },
  { fg: '--ink-danger', bg: '--ink-bg', min: 3.0, kind: 'ui' },
  { fg: '--ink-muted', bg: '--ink-surface', min: 3.0, kind: 'ui' },
  { fg: '--ink-muted', bg: '--ink-group-bg', min: 3.0, kind: 'ui' },
  { fg: '--ink-muted', bg: '--ink-bg', min: 3.0, kind: 'ui' },
  { fg: '--ink-island-ok', bg: '--ink-island', min: 3.0, kind: 'ui' },
  { fg: '--ink-island-danger', bg: '--ink-island', min: 3.0, kind: 'ui' },
  { fg: '--ink-accent', bg: '--ink-island', min: 3.0, kind: 'ui' },
  { fg: '--ink-switch-on-thumb', bg: '--ink-switch-on-bg', min: 3.0, kind: 'ui' },
]);

/**
 * tokens.css의 라이트·다크 토큰 대비를 계산하고 실패 항목을 모아 돌려준다.
 * @param {string} [cssPath]
 */
export function checkContrast(cssPath = DEFAULT_TOKENS_PATH) {
  const cssText = readFileSync(cssPath, 'utf8');
  const { light, dark, mediaDark, darkRaw, mediaDarkRaw } = parseTokenThemes(cssText);

  /** @type {Array<{ theme: string, fg: string, bg: string, fgHex: string, bgHex: string, ratio: number, min: number, kind: 'text' | 'ui', ok: boolean, error?: string }>} */
  const results = [];
  for (const [themeName, vars] of [
    ['light', light],
    ['dark', dark],
  ]) {
    for (const pair of CONTRAST_PAIRS) {
      const fgRaw = resolveTokenValue(vars, pair.fg);
      const bgRaw = resolveTokenValue(vars, pair.bg);
      const underRaw = pair.under ? resolveTokenValue(vars, pair.under) : null;
      const fgColor = fgRaw ? parseColor(fgRaw) : null;
      let bgColor = bgRaw ? parseColor(bgRaw) : null;

      if (!fgColor || !bgColor) {
        results.push({
          theme: themeName,
          fg: pair.fg,
          bg: pair.bg,
          fgHex: fgRaw ?? 'missing',
          bgHex: bgRaw ?? 'missing',
          ratio: 0,
          min: pair.min,
          kind: pair.kind,
          ok: false,
          error: '해석할 수 없는 색 토큰',
        });
        continue;
      }

      if (bgColor[3] < 1) {
        const underColor = underRaw ? parseColor(underRaw) : parseColor(resolveTokenValue(vars, '--ink-surface') ?? '');
        if (underColor) bgColor = compositeOver(bgColor, underColor);
      }
      const effectiveFg = fgColor[3] < 1 ? compositeOver(fgColor, bgColor) : fgColor;
      const ratio = contrastRatio(effectiveFg, bgColor);
      results.push({
        theme: themeName,
        fg: pair.fg,
        bg: pair.bg,
        fgHex: fgRaw,
        bgHex: bgRaw,
        ratio,
        min: pair.min,
        kind: pair.kind,
        ok: ratio >= pair.min,
      });
    }
  }

  const failures = results.filter((r) => !r.ok);
  return { results, failures, light, dark, mediaDark, darkRaw, mediaDarkRaw };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const { results, failures } = checkContrast();
  if (failures.length > 0) {
    console.error(`대비 검사 실패 (${failures.length}/${results.length}건):`);
    for (const f of failures) {
      console.error(
        `  [${f.theme}] ${f.fg}(${f.fgHex}) / ${f.bg}(${f.bgHex}): ${f.ratio.toFixed(2)}:1 < 기준 ${f.min}:1${f.error ? ` (${f.error})` : ''}`,
      );
    }
    process.exit(1);
  }
  const minText = Math.min(...results.filter((r) => r.kind === 'text').map((r) => r.ratio));
  const minUi = Math.min(...results.filter((r) => r.kind === 'ui').map((r) => r.ratio));
  console.log(
    `대비 검사 통과: ${results.length}쌍 확인 (글자 최소 ${minText.toFixed(2)}:1 ≥ 4.5, 아이콘·UI 최소 ${minUi.toFixed(2)}:1 ≥ 3.0)`,
  );
}
