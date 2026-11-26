// 다국어 카탈로그 합치기(R2-I18N): packages/ink-ui/src/lib/i18n/areas/<영역>.<언어>.json(원본, 사람이 고침)
// → 같은 폴더 위 ko.json·en.json(합친 결과, 손으로 고치지 않음 — Rust include_str!와 화면이 이것을 읽는다).
// 여러 사람·에이전트가 동시에 영역 파일만 고치고 이 스크립트를 돌리면 서로 덮어쓰지 않는다.
//   node scripts/i18n-build.mjs          합쳐서 쓴다
//   node scripts/i18n-build.mjs --check  합친 결과가 저장된 것과 같은지만 본다(다르면 실패)
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', 'packages', 'ink-ui', 'src', 'lib', 'i18n');
const LANGS = ['ko', 'en'];

/** @returns {{ out: Record<string, Record<string, string>>, errors: string[] }} */
export function buildCatalogs(root = ROOT) {
  const errors = [];
  const out = Object.fromEntries(LANGS.map((l) => [l, {}]));
  const owner = Object.fromEntries(LANGS.map((l) => [l, {}]));
  const files = readdirSync(join(root, 'areas')).filter((f) => f.endsWith('.json')).sort();
  const areas = new Set(files.map((f) => f.replace(/\.(ko|en)\.json$/, '')));
  for (const area of areas) {
    for (const lang of LANGS) {
      const f = `${area}.${lang}.json`;
      if (!files.includes(f)) {
        errors.push(`${f} 없음(${area} 영역은 두 언어 다 있어야 해요)`);
        continue;
      }
      let data;
      try {
        data = JSON.parse(readFileSync(join(root, 'areas', f), 'utf8'));
      } catch (e) {
        errors.push(`${f}: JSON 형식 오류 ${e.message}`);
        continue;
      }
      for (const [k, v] of Object.entries(data)) {
        if (typeof v !== 'string') errors.push(`${f}: ${k} 값이 글이 아님`);
        if (k in out[lang]) errors.push(`${lang}: ${k}가 ${owner[lang][k]}와 ${f} 두 곳에 있음`);
        out[lang][k] = v;
        owner[lang][k] = f;
      }
    }
  }
  for (const lang of LANGS) out[lang] = Object.fromEntries(Object.entries(out[lang]).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
  return { out, errors };
}

export function serialize(obj) {
  return `${JSON.stringify(obj, null, 2)}\n`;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const check = process.argv.includes('--check');
  const { out, errors } = buildCatalogs();
  if (errors.length) {
    console.error(errors.join('\n'));
    process.exit(1);
  }
  let stale = false;
  for (const lang of LANGS) {
    const path = join(ROOT, `${lang}.json`);
    const want = serialize(out[lang]);
    const have = readFileSync(path, 'utf8');
    if (have !== want) {
      if (check) {
        console.error(`${lang}.json이 영역 파일과 달라요 — node scripts/i18n-build.mjs`);
        stale = true;
      } else writeFileSync(path, want);
    }
  }
  if (stale) process.exit(1);
  console.log(`i18n ${check ? '확인' : '합침'}: ${Object.keys(out.ko).length}키`);
}
