import test from 'node:test';
import assert from 'node:assert/strict';
import fontkit from '@pdf-lib/fontkit';
import { PDFDocument, PDFName } from 'pdf-lib';
import defaults from '../../../../../docs/shortcuts/defaults-v1.json' with { type: 'json' };
import {
  PAGE_SIZES,
  buildShortcutSheetModel,
  buildShortcutSheetPdf,
  exportShortcutSheetPdf,
  extractTextFragments,
  loadPdfFonts,
  suggestedShortcutPdfName,
} from './index.js';
import { assertTrueTypeBytes } from './fonts.js';

function withTimeout(promise, ms = 15000, label = 'async op') {
  let timer;
  const timeoutPromise = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${label} 시간 초과 (${ms}ms)`)), ms);
    timer.unref?.();
  });
  return Promise.race([promise, timeoutPromise]).finally(() => {
    if (timer !== undefined) clearTimeout(timer);
  });
}

function buildShortcutsFromDefaults(osKey, overrides = {}) {
  const out = [];
  for (const [id, slot] of Object.entries(defaults.slots)) {
    const raw = slot[osKey];
    const combo = id in overrides ? overrides[id] : raw ?? '';
    const scope = slot.scope;
    const group =
      scope === 'capture'
        ? 'capture-mode'
        : scope === 'draw'
          ? 'draw-mode'
          : scope === 'zoom'
            ? 'zoom-mode'
            : ['region', 'window', 'record', 'last', 'full', 'active', 'delay'].includes(id)
              ? 'capture'
              : ['draw', 'zoom', 'board', 'topmost'].includes(id)
                ? 'tools'
                : 'quick';
    out.push({
      id,
      scope,
      group,
      label: id,
      combo: combo ?? '',
      later: ['pin', 'pinToggle', 'timer'].includes(id),
      windowsOnly: slot.mac === null,
    });
  }
  return out;
}

test('PDF용 내장 글꼴 5종이 모두 TrueType(TTF)이며 fontkit TTFFont로 열린다', async () => {
  const fonts = await withTimeout(loadPdfFonts(), 15000, 'loadPdfFonts');
  const entries = Object.entries(fonts);
  assert.equal(entries.length, 5);
  for (const [role, bytes] of entries) {
    assertTrueTypeBytes(bytes, role);
    const fk = fontkit.create(bytes);
    assert.equal(fk.constructor.name, 'TTFFont', `${role}가 WOFF2 등 비TTF로 열림: ${fk.constructor.name}`);
    assert.ok(fk.loca, `${role}에 loca 테이블이 없음`);
  }
});

test('PDF 바이트 헤더(%PDF-), A4 가로 크기, 글꼴 임베딩을 검증한다', async () => {
  const shortcuts = buildShortcutsFromDefaults('mac', {
    record: 'Option+R',
    board: 'Option+B',
    'd-spot': 'F',
    delay: '',
  });
  const bytes = await withTimeout(
    buildShortcutSheetPdf({
      shortcuts,
      lang: 'ko',
      os: 'macos',
      date: '2026-11-27',
      version: '0.1.0',
    }),
    15000,
    'buildShortcutSheetPdf(mac/ko)',
  );

  assert.ok(bytes instanceof Uint8Array);
  const header = new TextDecoder().decode(bytes.subarray(0, 5));
  assert.equal(header, '%PDF-');

  const doc = await withTimeout(PDFDocument.load(bytes), 15000, 'PDFDocument.load(mac/ko)');
  assert.equal(doc.getPageCount(), 1);
  const page = doc.getPage(0);
  const { width, height } = page.getSize();
  assert.ok(Math.abs(width - PAGE_SIZES.A4.width) < 0.5, `A4 가로 폭 불일치: ${width}`);
  assert.ok(Math.abs(height - PAGE_SIZES.A4.height) < 0.5, `A4 가로 높이 불일치: ${height}`);

  // 임베딩된 글꼴(FontFile2)이 5종 이상 들어 있는지 확인
  let fontFileCount = 0;
  for (const [, obj] of doc.context.enumerateIndirectObjects()) {
    if (obj?.dict?.get?.(PDFName.of('FontFile2'))) {
      fontFileCount += 1;
    }
  }
  assert.ok(fontFileCount >= 5, `임베딩된 TTF 글꼴 수 부족: ${fontFileCount}`);
});

test('영어 + 미국 지역이면 Letter 가로 용지를 사용한다', async () => {
  const shortcuts = buildShortcutsFromDefaults('win');
  const bytes = await withTimeout(
    buildShortcutSheetPdf({
      shortcuts,
      lang: 'en',
      os: 'windows',
      region: 'US',
      date: '2026-11-27',
      version: '0.1.0',
    }),
    15000,
    'buildShortcutSheetPdf(win/en/US)',
  );

  const doc = await withTimeout(PDFDocument.load(bytes), 15000, 'PDFDocument.load(win/en/US)');
  assert.equal(doc.getPageCount(), 1);
  const { width, height } = doc.getPage(0).getSize();
  assert.ok(Math.abs(width - PAGE_SIZES.LETTER.width) < 0.5);
  assert.ok(Math.abs(height - PAGE_SIZES.LETTER.height) < 0.5);
});

test('레이아웃 모델: 현재 값 반영·비운 칸 제외·OS 분리·영어 판 한글 0건 검증', () => {
  const macShortcuts = buildShortcutsFromDefaults('mac', {
    record: 'Option+R',
    delay: '',
  });
  const winShortcuts = buildShortcutsFromDefaults('win', {
    record: 'Alt+R',
    delay: '',
  });

  // 1) macOS (ko & en)
  for (const lang of ['ko', 'en']) {
    const model = buildShortcutSheetModel({
      shortcuts: macShortcuts,
      lang,
      os: 'macos',
      date: '2026-11-27',
      version: '0.1.0',
    });
    const frags = extractTextFragments(model);
    const joined = frags.join('\n');

    const recordHero = model.hero.find((h) => h.id === 'record');
    assert.deepEqual(recordHero?.tokens, ['⌥', 'R']);
    assert.equal(recordHero?.modified, true);

    // 비운 칸(delay) 제외
    const delayRow = model.pages[0].columns[0]
      .flatMap((sec) => sec.rows ?? [])
      .find((r) => r.id === 'delay');
    assert.equal(delayRow, undefined);

    // 맥 판에 Ctrl / Alt / Win 문자열 없음
    assert.equal(/\b(Ctrl|Alt|Win)\b/.test(joined), false, `macOS(${lang})에 Windows 키 남음:\n${joined}`);
    if (lang === 'en') {
      assert.equal(/[가-힣ㄱ-ㅎㅏ-ㅣ]/.test(joined), false, `영어(mac) 판에 한글 남음:\n${joined}`);
    }
  }

  // 2) Windows (ko & en)
  for (const lang of ['ko', 'en']) {
    const model = buildShortcutSheetModel({
      shortcuts: winShortcuts,
      lang,
      os: 'windows',
      date: '2026-11-27',
      version: '0.1.0',
    });
    const frags = extractTextFragments(model);
    const joined = frags.join('\n');

    const recordHero = model.hero.find((h) => h.id === 'record');
    assert.deepEqual(recordHero?.tokens, ['Alt', 'R']);
    assert.equal(recordHero?.modified, true);

    // Windows 판에 맥 기호 없음
    assert.equal(/[⌘⌥⇧⌃]/.test(joined), false, `Windows(${lang})에 맥 기호 남음:\n${joined}`);
    if (lang === 'en') {
      assert.equal(/[가-힣ㄱ-ㅎㅏ-ㅣ]/.test(joined), false, `영어(win) 판에 한글 남음:\n${joined}`);
    }
  }
});

test('항목이 많아 넘치면 묶음 중간에서 자르지 않고 2쪽으로 보낸다', async () => {
  const base = buildShortcutsFromDefaults('mac');
  const extras = Array.from({ length: 18 }, (_, i) => ({
    id: `custom-${i + 1}`,
    scope: 'custom',
    group: i < 9 ? 'extra-a' : 'extra-b',
    groupTitle: i < 9 ? '추가 묶음 A' : '추가 묶음 B',
    label: `사용자 동작 ${i + 1}`,
    combo: `Option+${(i % 9) + 1}`,
  }));

  const model = buildShortcutSheetModel({
    shortcuts: [...base, ...extras],
    lang: 'ko',
    os: 'macos',
    date: '2026-11-27',
    version: '0.1.0',
  });

  assert.equal(model.pages.length, 2);
  const page1Ids = new Set(model.pages[0].columns.flat().map((s) => s.id));
  const page2Ids = new Set(model.pages[1].columns.flat().map((s) => s.id));
  for (const id of page2Ids) {
    assert.equal(page1Ids.has(id), false, `묶음 ${id}가 1쪽과 2쪽에 나뉘어선 안 됨`);
  }

  const bytes = await withTimeout(
    buildShortcutSheetPdf({
      shortcuts: [...base, ...extras],
      lang: 'ko',
      os: 'macos',
      date: '2026-11-27',
      version: '0.1.0',
    }),
    15000,
    'buildShortcutSheetPdf(2pages)',
  );
  const doc = await withTimeout(PDFDocument.load(bytes), 15000, 'PDFDocument.load(2pages)');
  assert.equal(doc.getPageCount(), 2);
});

test('제안 파일명은 언어와 날짜 규칙을 따른다', () => {
  assert.equal(suggestedShortcutPdfName('ko', '2026-11-27'), '슉쇽-단축키-2026-11-27.pdf');
  assert.equal(suggestedShortcutPdfName('en', '2026-11-27'), 'shookshok-shortcuts-2026-11-27.pdf');
});

test('exportShortcutSheetPdf는 client.savePdf에 PDF 바이트와 언어별 파일명을 전달한다', async () => {
  const calls = [];
  const fakeClient = {
    async savePdf(bytes, name) {
      calls.push({ bytes, name });
      return true;
    },
  };
  const shortcuts = buildShortcutsFromDefaults('win');
  const res = await withTimeout(
    exportShortcutSheetPdf(fakeClient, {
      shortcuts,
      lang: 'en',
      os: 'windows',
      date: '2026-11-27',
      version: '0.1.0',
    }),
    15000,
    'exportShortcutSheetPdf',
  );
  assert.deepEqual(res, { ok: true, name: 'shookshok-shortcuts-2026-11-27.pdf' });
  assert.equal(calls.length, 1);
  assert.equal(calls[0].name, 'shookshok-shortcuts-2026-11-27.pdf');
  assert.ok(calls[0].bytes instanceof Uint8Array);
  assert.equal(new TextDecoder().decode(calls[0].bytes.subarray(0, 5)), '%PDF-');
});
