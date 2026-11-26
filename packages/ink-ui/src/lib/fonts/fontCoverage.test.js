import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const I18N_DIR = join(HERE, '..', 'i18n');

// macOS 키 기호 등은 --ink-font-mono 또는 시스템 글꼴 폴백으로 그린다(font-metrics.md §3-2)
const SYSTEM_FALLBACK_CHARS = new Set(['⌥', '⌘', '⇧', '⌃', '⌫', '⏎', '⇥', '⎋', 'ª', 'º']);

test('번들 woff2 파일 5종이 manifest.json과 일치하고 1 MiB 이하이다', () => {
  const manifest = JSON.parse(readFileSync(join(HERE, 'manifest.json'), 'utf8'));
  assert.equal(manifest.files.length, 5);
  assert.ok(manifest.totalBytes <= 1024 * 1024, `총 글꼴 크기가 1 MiB 초과: ${manifest.totalBytes}`);

  let sum = 0;
  for (const entry of manifest.files) {
    const p = join(HERE, entry.file);
    assert.ok(existsSync(p), `글꼴 파일 없음: ${entry.file}`);
    const raw = readFileSync(p);
    assert.equal(raw.length, entry.bytes, `${entry.file} 크기 불일치`);
    const sha = createHash('sha256').update(raw).digest('hex');
    assert.equal(sha, entry.sha256, `${entry.file} sha256 불일치`);
    sum += raw.length;
  }
  assert.equal(sum, manifest.totalBytes);
});

test('Paperlogy 부분 집합에 빈 한글 글리프가 없다', () => {
  const manifest = JSON.parse(readFileSync(join(HERE, 'manifest.json'), 'utf8'));
  for (const entry of manifest.files.filter((f) => f.family === 'Paperlogy')) {
    assert.equal(entry.emptyHangulCount, 0, `${entry.file} 빈 한글 글리프 발견`);
  }
  const paperlogyChars = new Set(Array.from(manifest.codepointsByFamily.Paperlogy ?? ''));
  for (const ch of ['큑', '뷁']) {
    assert.ok(!paperlogyChars.has(ch), `Paperlogy cmap에서 빈 글리프 '${ch}'가 제거되어야 함`);
  }
});

test('카탈로그(ko·en)의 모든 가시 글자가 본문·제목 글꼴 부분 집합에 들어 있다', () => {
  const manifest = JSON.parse(readFileSync(join(HERE, 'manifest.json'), 'utf8'));
  const freesentationChars = new Set(Array.from(manifest.codepointsByFamily.Freesentation ?? ''));
  const paperlogyChars = new Set(Array.from(manifest.codepointsByFamily.Paperlogy ?? ''));

  const ko = JSON.parse(readFileSync(join(I18N_DIR, 'ko.json'), 'utf8'));
  const en = JSON.parse(readFileSync(join(I18N_DIR, 'en.json'), 'utf8'));

  const allValues = [...Object.values(ko), ...Object.values(en), '슉쇽', 'ShookShok'];
  const missingBody = [];
  const missingDisplay = [];

  for (const text of allValues) {
    for (const ch of String(text)) {
      if (/\s/.test(ch) || SYSTEM_FALLBACK_CHARS.has(ch)) continue;
      if (!freesentationChars.has(ch)) missingBody.push(ch);
      // 제목·트레이 글꼴은 빠진 글자만 Freesentation으로 글자 단위 대체한다
      if (!paperlogyChars.has(ch) && !freesentationChars.has(ch)) missingDisplay.push(ch);
    }
  }

  assert.deepEqual([...new Set(missingBody)], [], 'Freesentation 부분 집합에 카탈로그 글자가 빠짐');
  assert.deepEqual([...new Set(missingDisplay)], [], 'Paperlogy+Freesentation에 카탈로그 글자가 빠짐');

  // 현재 ko.json의 모든 한글 음절은 Paperlogy 자체에도 모두 들어 있어야 한다
  const missingPaperlogyHangul = [];
  for (const text of [...Object.values(ko), '슉쇽']) {
    for (const ch of String(text)) {
      const cp = ch.codePointAt(0) ?? 0;
      if (cp >= 0xac00 && cp <= 0xd7a3 && !paperlogyChars.has(ch)) {
        missingPaperlogyHangul.push(ch);
      }
    }
  }
  assert.deepEqual([...new Set(missingPaperlogyHangul)], [], 'Paperlogy에 ko.json 한글이 빠짐');
});

test('SIL OFL 1.1 라이선스 파일 3종이 모두 존재한다', () => {
  for (const name of ['Freesentation-OFL.txt', 'Paperlogy-OFL.txt', 'IBMPlexMono-OFL.txt']) {
    const p = join(HERE, 'LICENSES', name);
    assert.ok(existsSync(p), `라이선스 파일 없음: ${name}`);
    const body = readFileSync(p, 'utf8');
    assert.match(body, /SIL OPEN FONT LICENSE Version 1\.1/, `${name}에 OFL 1.1 원문 없음`);
  }
});
