#!/usr/bin/env node
/**
 * 슉쇽 2차 고도화 화면 스냅숏 시험(R2-QA-1) + 성능 예산 측정(R2-PERF-1).
 *
 * - 가짜 클라이언트(FakeInkClient) 기반 `@screen-ink/ui` Vite 미리보기만 사용한다(실제 OS 화면 캡처 없음).
 * - 기존 맥 캐시 브라우저(chromium-1248 / webkit-2370)를 그대로 사용하며 새 브라우저를 내려받지 않는다.
 * - 단일 작업자(순차)로 실행하며, 종료 시 자체 띄운 Vite 서버와 브라우저를 반드시 정리한다.
 * - PNG 그림은 `.gitignore`된 `output/qa/<날짜>/`에 저장하고, 커밋 대상은 `docs/qa/<날짜>/`의 요약·보고 문서다.
 *
 * 사용 예:
 *   /Users/bodyking/.local/bin/resource-guard run --profile test --wait-seconds 900 -- \
 *     bash -c 'timeout 900 node scripts/qa-snapshots.mjs'
 *   node scripts/qa-snapshots.mjs --only=snapshots
 *   node scripts/qa-snapshots.mjs --only=perf
 */

import { chromium, webkit } from '@playwright/test';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdirSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { pathToFileURL } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const UI_PKG_DIR = join(ROOT, 'packages', 'ink-ui');

// ── CLI 옵션 ──
const argv = process.argv.slice(2);
function getArg(name, fallback) {
  const prefix = `--${name}=`;
  const found = argv.find((a) => a.startsWith(prefix));
  return found ? found.slice(prefix.length) : fallback;
}

const ONLY = getArg('only', 'all'); // all | snapshots | perf
const DATE_STR = getArg('date', '2026-11-27');
const SHOT_DIR = resolve(ROOT, getArg('shot-dir', join('output', 'qa', DATE_STR)));
const JSON_OUT = resolve(ROOT, getArg('json-out', join('output', 'qa', DATE_STR, 'results.json')));
const INCLUDE_WEBKIT_PERF = getArg('webkit-perf', '1') !== '0';

const LANGS = ['ko', 'en'];
const PLATFORMS = ['macos', 'windows'];
const THEMES = ['light', 'dark'];
const VIEWPORT = { width: 1280, height: 820 };

// ── OS 표기 검사 규칙 (packages/ink-ui/src/lib/i18n/osWords.test.js와 동기화) ──
const WIN_ONLY_TERMS = [
  { name: 'Ctrl', source: '\\bCtrl\\b', flags: 'i' },
  { name: 'Alt', source: '\\bAlt\\b', flags: 'i' },
  { name: 'Win', source: '\\bWin\\b', flags: '' },
  { name: '.exe', source: '\\.exe\\b', flags: 'i' },
  { name: '작업 표시줄', source: '작업\\s*표시줄', flags: '' },
  { name: '탐색기', source: '탐색기', flags: '' },
  { name: 'Taskbar', source: '\\bTaskbar\\b', flags: 'i' },
  { name: 'File Explorer', source: '\\b(?:File\\s+)?Explorer\\b', flags: 'i' },
];

const MAC_ONLY_TERMS = [
  { name: '⌘', source: '⌘', flags: '' },
  { name: '⌥', source: '⌥', flags: '' },
  { name: '⌃', source: '⌃', flags: '' },
  { name: 'Cmd', source: '\\bCmd\\b', flags: 'i' },
  { name: 'Option', source: '\\bOption\\b', flags: '' },
  { name: '메뉴 막대', source: '메뉴\\s*막대', flags: '' },
  { name: 'Finder', source: '\\bFinder\\b', flags: '' },
  { name: '시스템 설정', source: '시스템\\s*설정', flags: '' },
  { name: 'Menu bar', source: '\\bMenu\\s+bar\\b', flags: 'i' },
  { name: 'System Settings', source: '\\bSystem\\s+Settings\\b', flags: 'i' },
];

// ── 유틸리티 ──
async function findFreePort() {
  return new Promise((res, rej) => {
    const srv = createServer();
    srv.unref();
    srv.on('error', rej);
    srv.listen(0, '127.0.0.1', () => {
      const addr = srv.address();
      const port = typeof addr === 'object' && addr ? addr.port : 5185;
      srv.close(() => res(port));
    });
  });
}

async function waitForHttp(url, timeoutMs = 25000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const r = await fetch(url);
      if (r.ok) return;
    } catch {
      // 서버 기동 대기
    }
    await new Promise((r) => setTimeout(r, 150));
  }
  throw new Error(`Vite 미리보기 서버 응답 시간 초과: ${url}`);
}

function startViteServer(port) {
  const viteBin = join(ROOT, 'node_modules', '.bin', 'vite');
  const child = spawn(viteBin, ['--host', '127.0.0.1', '--port', String(port), '--strictPort'], {
    cwd: UI_PKG_DIR,
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, BROWSER: 'none' },
  });
  let logs = '';
  child.stdout?.on('data', (d) => {
    logs += d.toString();
  });
  child.stderr?.on('data', (d) => {
    logs += d.toString();
  });
  return {
    child,
    getLogs: () => logs,
    stop: async () => {
      if (child.exitCode !== null || child.killed) return;
      child.kill('SIGTERM');
      await new Promise((r) => {
        const t = setTimeout(() => {
          try {
            child.kill('SIGKILL');
          } catch {
            // 무시
          }
          r();
        }, 3000);
        child.once('exit', () => {
          clearTimeout(t);
          r();
        });
      });
    },
  };
}

function stats(values) {
  if (!values.length) return { n: 0, min: 0, p50: 0, p95: 0, max: 0, avg: 0 };
  const s = [...values].sort((a, b) => a - b);
  const pick = (q) => s[Math.min(s.length - 1, Math.max(0, Math.ceil(q * s.length) - 1))];
  const avg = s.reduce((a, b) => a + b, 0) / s.length;
  const r = (v) => Math.round(v * 100) / 100;
  return {
    n: s.length,
    min: r(s[0]),
    p50: r(pick(0.5)),
    p95: r(pick(0.95)),
    max: r(s[s.length - 1]),
    avg: r(avg),
  };
}

// ── DOM 자동 검사기 (브라우저 내부 실행) ──
async function auditDom(page, { rootSelector, platform, checkOverlapSelectors = [] }) {
  const forbidden = platform === 'macos' ? WIN_ONLY_TERMS : MAC_ONLY_TERMS;
  return page.evaluate(
    ({ rootSelector, forbidden, checkOverlapSelectors }) => {
      const root = rootSelector ? document.querySelector(rootSelector) : document.body;
      if (!root) {
        return {
          found: false,
          overflows: [],
          truncations: [],
          boundsViolations: [],
          viewportClips: [],
          overlaps: [],
          osMixes: [],
          textLength: 0,
        };
      }

      const vw = window.innerWidth;
      const vh = window.innerHeight;
      const rootRect = root.getBoundingClientRect();

      const isVisible = (el) => {
        const cs = getComputedStyle(el);
        if (cs.display === 'none' || cs.visibility === 'hidden' || Number(cs.opacity) === 0) return false;
        const r = el.getBoundingClientRect();
        return r.width > 0.5 && r.height > 0.5;
      };

      const describeEl = (el) => {
        const tag = el.tagName.toLowerCase();
        const cls =
          typeof el.className === 'string'
            ? el.className
                .trim()
                .split(/\s+/)
                .filter((c) => c && !c.startsWith('s-'))
                .slice(0, 3)
                .join('.')
            : '';
        const txt = (el.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 48);
        return `${tag}${cls ? '.' + cls : ''}${txt ? `("${txt}")` : ''}`;
      };

      const overflows = [];
      const truncations = [];
      const boundsViolations = [];
      const viewportClips = [];
      const overlaps = [];

      // 루트 자체 뷰포트 잘림 확인 (스크롤 가능한 전체 페이지 제외)
      if (root !== document.body && rootRect.width > 0 && rootRect.height > 0) {
        if (
          rootRect.left < -2 ||
          rootRect.top < -2 ||
          rootRect.right > vw + 2 ||
          rootRect.bottom > vh + 2
        ) {
          // island 모음 페이지 같은 의도적 세로 페이지는 top/bottom 제외
          const rootCs = getComputedStyle(root);
          const pageScrollable = root.classList.contains('pv-island-page');
          if (!pageScrollable || rootRect.left < -2 || rootRect.right > vw + 2) {
            viewportClips.push({
              el: describeEl(root),
              rect: {
                left: Math.round(rootRect.left),
                top: Math.round(rootRect.top),
                right: Math.round(rootRect.right),
                bottom: Math.round(rootRect.bottom),
              },
              viewport: { vw, vh },
            });
          }
        }
      }

      const all = [root, ...root.querySelectorAll('*')];
      const seenOverflow = new Set();

      for (const el of all) {
        if (!(el instanceof HTMLElement)) continue;
        if (!isVisible(el)) continue;
        // 미리보기 보조 바/메타는 제외
        if (el.closest('.pv-bar, .draw-preview-hud')) continue;

        const cs = getComputedStyle(el);
        const r = el.getBoundingClientRect();

        // 1) 가로 넘침·잘림 검사 (scrollWidth > clientWidth)
        const sw = el.scrollWidth;
        const cw = el.clientWidth;
        const isIntentionalHScroll = cs.overflowX === 'auto' || cs.overflowX === 'scroll';
        if (!isIntentionalHScroll && cw > 0 && sw > cw + 1.5) {
          // 자식이 이미 같은 넘침을 보고하는 단순 래퍼(텍스트 노드 없음)는 중복 축소
          const hasDirectText = [...el.childNodes].some(
            (n) => n.nodeType === Node.TEXT_NODE && (n.textContent || '').trim().length > 0,
          );
          const isLeafLike =
            hasDirectText ||
            el.tagName === 'BUTTON' ||
            el.tagName === 'INPUT' ||
            el.tagName === 'SPAN' ||
            el.tagName === 'P' ||
            el.tagName === 'H1' ||
            el.tagName === 'H2' ||
            el.tagName === 'LABEL' ||
            el.children.length === 0;
          const isEllipsis = cs.textOverflow === 'ellipsis' && cs.overflowX === 'hidden';
          const isClippedHidden = cs.overflowX === 'hidden' || cs.overflowX === 'clip';

          if (isLeafLike || isClippedHidden) {
            const key = `${describeEl(el)}@${Math.round(sw)}/${Math.round(cw)}`;
            if (!seenOverflow.has(key)) {
              seenOverflow.add(key);
              const entry = {
                el: describeEl(el),
                scrollWidth: Math.round(sw),
                clientWidth: Math.round(cw),
                diffPx: Math.round(sw - cw),
                overflowX: cs.overflowX,
                textOverflow: cs.textOverflow,
                whiteSpace: cs.whiteSpace,
              };
              if (isEllipsis) {
                truncations.push(entry);
              } else {
                overflows.push(entry);
              }
            }
          }
        }

        // 2) 세로 잘림 검사 (고정 높이/overflow-y:hidden인데 내용이 세로로 잘림)
        const sh = el.scrollHeight;
        const ch = el.clientHeight;
        const isVHidden = cs.overflowY === 'hidden' || cs.overflowY === 'clip';
        if (isVHidden && ch > 0 && sh > ch + 3) {
          // 의도적 창 외곽 컨테이너(.ink-settings-win 등은 내부 .ink-scroll에서 스크롤)는 제외
          const hasScrollChild = Boolean(el.querySelector('.ink-scroll'));
          if (!hasScrollChild && el.tagName !== 'SVG') {
            const hasDirectText = [...el.childNodes].some(
              (n) => n.nodeType === Node.TEXT_NODE && (n.textContent || '').trim().length > 0,
            );
            if (hasDirectText || el.tagName === 'BUTTON' || el.tagName === 'P' || el.tagName === 'SPAN') {
              overflows.push({
                el: describeEl(el),
                axis: 'vertical',
                scrollHeight: Math.round(sh),
                clientHeight: Math.round(ch),
                diffPx: Math.round(sh - ch),
                overflowY: cs.overflowY,
              });
            }
          }
        }

        // 3) 상위 컨테이너 경계 밖 이탈(visible overflow로 삐져나감)
        const parentBox = el.closest(
          '.ink-settings-win, .ink-tray-popover, .ink-first-dialog, .ink-cheat-card, .ink-island, .ink-cw-panel, .ink-fp, .ink-toolbar, .ink-draw-toolbar',
        );
        if (parentBox && parentBox !== el && root.contains(parentBox)) {
          // 팝오버/말풍선/포커스 링/화살표/글꼴 피커/검색 팝오버는 바깥으로 뜨는 것이 의도된 동작
          const isFloatingChild = Boolean(
            el.closest('.ink-fp, .ink-search-popover, .ink-toolbar-tip, .ink-draw-tip, .ink-cw-arrow, .ink-cw-thumb-tip'),
          );
          if (!isFloatingChild) {
            const pr = parentBox.getBoundingClientRect();
            const pCs = getComputedStyle(parentBox);
            // 가로로 컨테이너 오른쪽/왼쪽 경계 이탈
            if (r.right > pr.right + 2 || r.left < pr.left - 2) {
              // 스크롤 컨테이너 안에 있는 요소가 스크롤 영역에 가려진 경우는 제외
              const scrollAnc = el.closest('.ink-scroll');
              if (!scrollAnc || scrollAnc === parentBox) {
                boundsViolations.push({
                  el: describeEl(el),
                  parent: describeEl(parentBox),
                  overflowRightPx: Math.max(0, Math.round(r.right - pr.right)),
                  overflowLeftPx: Math.max(0, Math.round(pr.left - r.left)),
                  parentOverflowX: pCs.overflowX,
                });
              }
            }
          }
        }
      }

      // 4) 지정된 행/헤더 내부의 직계 형제 요소 겹침 검사
      for (const sel of checkOverlapSelectors) {
        const containers = root.querySelectorAll(sel);
        for (const box of containers) {
          if (!(box instanceof HTMLElement) || !isVisible(box)) continue;
          const kids = [...box.children].filter(
            (c) =>
              c instanceof HTMLElement &&
              isVisible(c) &&
              getComputedStyle(c).position !== 'absolute' &&
              getComputedStyle(c).position !== 'fixed',
          );
          for (let i = 0; i < kids.length; i++) {
            const a = kids[i].getBoundingClientRect();
            for (let j = i + 1; j < kids.length; j++) {
              const b = kids[j].getBoundingClientRect();
              const ox = Math.min(a.right, b.right) - Math.max(a.left, b.left);
              const oy = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
              if (ox > 2 && oy > 2) {
                overlaps.push({
                  container: describeEl(box),
                  a: describeEl(kids[i]),
                  b: describeEl(kids[j]),
                  overlapPx: { x: Math.round(ox), y: Math.round(oy) },
                });
              }
            }
          }
        }
      }

      // 5) OS 표기 섞임 검사 (렌더된 텍스트 + aria-label + title + placeholder)
      const osMixes = [];
      const regexes = forbidden.map((t) => ({ name: t.name, re: new RegExp(t.source, t.flags) }));
      const checkText = (text, source, elDesc) => {
        if (!text) return;
        const clean = text.replace(/\s+/g, ' ').trim();
        if (!clean) return;
        for (const item of regexes) {
          const m = item.re.exec(clean);
          if (m) {
            const idx = m.index;
            const snippet = clean.slice(Math.max(0, idx - 18), Math.min(clean.length, idx + m[0].length + 18));
            osMixes.push({
              term: item.name,
              matched: m[0],
              source,
              el: elDesc,
              snippet,
            });
          }
        }
      };

      // 텍스트 노드 단위 순회(미리보기 HUD/바 제외)
      const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
      let node;
      let totalTextLen = 0;
      while ((node = walker.nextNode())) {
        const parentEl = node.parentElement;
        if (!parentEl || !isVisible(parentEl)) continue;
        if (parentEl.closest('.pv-bar, .draw-preview-hud, .pv-pane-label')) continue;
        const val = node.nodeValue || '';
        totalTextLen += val.trim().length;
        checkText(val, 'text', describeEl(parentEl));
      }

      // 접근성 이름·툴팁·플레이스홀더 검사
      for (const el of all) {
        if (!(el instanceof HTMLElement) || !isVisible(el)) continue;
        if (el.closest('.pv-bar, .draw-preview-hud')) continue;
        const desc = describeEl(el);
        for (const attr of ['aria-label', 'title', 'placeholder']) {
          const v = el.getAttribute(attr);
          if (v) checkText(v, attr, desc);
        }
      }

      // 중복 OS 섞임 제거
      const uniqueOsMixes = [];
      const seenOs = new Set();
      for (const item of osMixes) {
        const k = `${item.term}|${item.source}|${item.snippet}`;
        if (!seenOs.has(k)) {
          seenOs.add(k);
          uniqueOsMixes.push(item);
        }
      }

      return {
        found: true,
        overflows,
        truncations,
        boundsViolations,
        viewportClips,
        overlaps,
        osMixes: uniqueOsMixes,
        textLength: totalTextLen,
      };
    },
    { rootSelector, forbidden, checkOverlapSelectors },
  );
}

// ── 헬퍼: 페이지 안정화 대기 ──
async function settle(page, ms = 120) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  });
  if (ms > 0) await page.waitForTimeout(ms);
}

async function captureAndAudit(page, { id, label, lang, platform, theme, rootSelector, clipTarget, overlapSelectors = [] }) {
  await settle(page, 80);
  const audit = await auditDom(page, {
    rootSelector,
    platform,
    checkOverlapSelectors: overlapSelectors,
  });

  const fileName = `${id}__${platform}_${lang}_${theme}.png`;
  const filePath = join(SHOT_DIR, fileName);

  try {
    if (clipTarget) {
      const loc = page.locator(clipTarget).first();
      if ((await loc.count()) > 0 && (await loc.isVisible())) {
        await loc.screenshot({ path: filePath, animations: 'disabled' });
      } else {
        await page.screenshot({ path: filePath, animations: 'disabled' });
      }
    } else {
      await page.screenshot({ path: filePath, animations: 'disabled' });
    }
  } catch (err) {
    return {
      id,
      label,
      lang,
      platform,
      theme,
      fileName,
      screenshotError: String(err?.message ?? err),
      audit,
    };
  }

  return {
    id,
    label,
    lang,
    platform,
    theme,
    fileName,
    audit,
  };
}

// ── 1. 스냅숏 시험 (R2-QA-1) ──
async function runSnapshots(baseUrl) {
  const results = [];
  const consoleErrors = [];
  const pageErrors = [];

  mkdirSync(SHOT_DIR, { recursive: true });

  const browser = await chromium.launch({ headless: true });
  try {
    const context = await browser.newContext({ viewport: VIEWPORT, deviceScaleFactor: 1 });
    const page = await context.newPage();

    page.on('console', (msg) => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });
    page.on('pageerror', (err) => {
      pageErrors.push(String(err));
    });

    const overlapCommon = ['.ink-page-head', '.ink-row', '.ink-island', '.ink-cheat-row', '.ink-first-topbar', '.ink-tray-tile-top'];

    for (const platform of PLATFORMS) {
      for (const lang of LANGS) {
        for (const theme of THEMES) {
          const comboTag = `${platform}/${lang}/${theme}`;
          console.log(`[스냅숏] ${comboTag} 시작...`);
          await page.emulateMedia({ colorScheme: theme });

          // 1) 트레이 팝오버 (screen=tray는 light/dark 두 패널을 렌더하므로 해당 테마 패널만 캡처·검사)
          await page.goto(`${baseUrl}/?screen=tray&platform=${platform}&lang=${lang}&theme=${theme}&chrome=0`, {
            waitUntil: 'networkidle',
          });
          const trayPaneSel = `.pv-tray-pane[data-theme="${theme}"]`;
          results.push(
            await captureAndAudit(page, {
              id: 'tray',
              label: '트레이 팝오버',
              lang,
              platform,
              theme,
              rootSelector: `${trayPaneSel} [role="menu"], ${trayPaneSel}`,
              clipTarget: trayPaneSel,
              overlapSelectors: ['.ink-tray-tile-top', '.ink-hotkey-row', '.ink-pin-row'],
            }),
          );

          // 2) 설정 창 — 단축키(전역 / 화면 안), 캡처·녹화, 그리기·확대, 일반, 사용 설명서, 정보, 설정 찾기
          await page.goto(
            `${baseUrl}/?screen=settings&tab=shortcuts&platform=${platform}&lang=${lang}&theme=${theme}&perm=all-off&chrome=0`,
            { waitUntil: 'networkidle' },
          );
          results.push(
            await captureAndAudit(page, {
              id: 'settings-shortcuts',
              label: '설정 · 단축키(전역)',
              lang,
              platform,
              theme,
              rootSelector: '.ink-settings-win',
              clipTarget: '.ink-settings-win',
              overlapSelectors: overlapCommon,
            }),
          );

          // 단축키 화면 안 탭 클릭
          const inappTab = page.locator('.ink-scope-tab').nth(1);
          if ((await inappTab.count()) > 0) {
            await inappTab.click();
            results.push(
              await captureAndAudit(page, {
                id: 'settings-shortcuts-inapp',
                label: '설정 · 단축키(화면 안)',
                lang,
                platform,
                theme,
                rootSelector: '.ink-settings-win',
                clipTarget: '.ink-settings-win',
                overlapSelectors: overlapCommon,
              }),
            );
          }

          // 사이드바 탭 순회: capture, draw, general, help, info
          const navTabs = [
            { idx: 1, id: 'settings-capture', label: '설정 · 캡처·녹화' },
            { idx: 2, id: 'settings-draw', label: '설정 · 그리기·확대' },
            { idx: 3, id: 'settings-general', label: '설정 · 일반' },
            { idx: 4, id: 'settings-help', label: '설정 · 사용 설명서' },
            { idx: 5, id: 'settings-info', label: '설정 · 정보' },
          ];
          const navBtns = page.locator('.ink-sidebar .ink-nav');
          for (const nt of navTabs) {
            await navBtns.nth(nt.idx).click();
            results.push(
              await captureAndAudit(page, {
                id: nt.id,
                label: nt.label,
                lang,
                platform,
                theme,
                rootSelector: '.ink-settings-win',
                clipTarget: '.ink-settings-win',
                overlapSelectors: overlapCommon,
              }),
            );
          }

          // 설정 찾기 팝오버 열기
          const searchInput = page.locator('.ink-search-input');
          if ((await searchInput.count()) > 0) {
            await searchInput.fill(lang === 'en' ? 'draw' : '그리기');
            results.push(
              await captureAndAudit(page, {
                id: 'settings-search',
                label: '설정 · 찾기 팝오버',
                lang,
                platform,
                theme,
                rootSelector: '.ink-settings-win',
                clipTarget: '.ink-settings-win',
                overlapSelectors: ['.ink-search-item'],
              }),
            );
            await searchInput.fill('');
          }

          // 사용 설명서 모달 (찍기 가이드 + 문제 해결 가이드)
          for (const g of [
            { guide: 'capture', id: 'help-modal-capture', label: '사용 설명서 모달 · 찍기' },
            { guide: 'troubleshoot', id: 'help-modal-troubleshoot', label: '사용 설명서 모달 · 문제 해결' },
          ]) {
            await page.goto(
              `${baseUrl}/?screen=settings&tab=help&guide=${g.guide}&platform=${platform}&lang=${lang}&theme=${theme}&chrome=0`,
              { waitUntil: 'networkidle' },
            );
            results.push(
              await captureAndAudit(page, {
                id: g.id,
                label: g.label,
                lang,
                platform,
                theme,
                rootSelector: '[data-dialog-content], .ink-settings-win',
                clipTarget: '[data-dialog-content]',
                overlapSelectors: ['.ink-row'],
              }),
            );
          }

          // 3) 첫 실행 카드 5장 (step 0~4)
          const stepLabels = [
            '첫 실행 ① 반가워요',
            platform === 'macos' ? '첫 실행 ② 화면 기록 권한' : '첫 실행 ② 단축키 겹침',
            platform === 'macos' ? '첫 실행 ③ 손쉬운 사용 권한' : '첫 실행 ③ 켤 때 같이 켜기',
            '첫 실행 ④ 꼭 알아 둘 키 세 개',
            '첫 실행 ⑤ 준비 끝',
          ];
          for (let step = 0; step < 5; step++) {
            await page.goto(
              `${baseUrl}/?screen=first&step=${step}&perm=all-off&platform=${platform}&lang=${lang}&theme=${theme}&chrome=0`,
              { waitUntil: 'networkidle' },
            );
            results.push(
              await captureAndAudit(page, {
                id: `first-step${step}`,
                label: stepLabels[step],
                lang,
                platform,
                theme,
                rootSelector: '.ink-first-dialog',
                clipTarget: '.ink-first-dialog',
                overlapSelectors: ['.ink-first-topbar', '.ink-first-row', '.ink-first-footer'],
              }),
            );
          }

          // 4) 캡처 오버레이 → 툴바 → 색·굵기 패널 → 글꼴 고르기 팝오버
          await page.goto(
            `${baseUrl}/?screen=overlay&platform=${platform}&lang=${lang}&theme=${theme}&chrome=0`,
            { waitUntil: 'networkidle' },
          );
          await page.locator('.pv-overlay-launcher .pv-bar-group').nth(1).locator('button').first().click();
          await page.waitForSelector('.ink-overlay', { timeout: 5000 });
          await settle(page, 100);

          // 드래그로 600×300 범위 확정 → 툴바 표시
          await page.mouse.move(320, 200);
          await page.mouse.down();
          await page.mouse.move(920, 480, { steps: 6 });
          await page.mouse.up();
          await page.waitForSelector('.ink-toolbar', { timeout: 5000 });
          results.push(
            await captureAndAudit(page, {
              id: 'capture-toolbar',
              label: '캡처 오버레이 · 툴바',
              lang,
              platform,
              theme,
              rootSelector: '.ink-overlay',
              overlapSelectors: [],
            }),
          );

          // 펜 도구 + 색·굵기 패널 열기
          await page.locator('.ink-toolbar [data-tool="pen"]').click();
          await page.locator('.ink-toolbar [data-action="style"]').click();
          await page.waitForSelector('.ink-cw-panel', { timeout: 5000 });
          results.push(
            await captureAndAudit(page, {
              id: 'color-width-panel',
              label: '색·굵기 패널(펜)',
              lang,
              platform,
              theme,
              rootSelector: '.ink-cw-panel',
              clipTarget: '.ink-cw-panel',
              overlapSelectors: ['.ink-cw-head', '.ink-cw-custom-row'],
            }),
          );

          // 글자 도구 전환 → 글꼴 고르기 팝오버 열기
          await page.locator('.ink-toolbar [data-tool="text"]').click();
          await settle(page, 60);
          // 패널이 닫혔다면 다시 연다
          if ((await page.locator('.ink-cw-panel').count()) === 0) {
            await page.locator('.ink-toolbar [data-action="style"]').click();
            await page.waitForSelector('.ink-cw-panel', { timeout: 5000 });
          }
          await page.locator('.ink-cw-panel [data-action="font-picker"]').click();
          await page.waitForSelector('.ink-fp', { timeout: 5000 });
          results.push(
            await captureAndAudit(page, {
              id: 'font-picker',
              label: '글꼴 고르기 팝오버',
              lang,
              platform,
              theme,
              rootSelector: '.ink-overlay',
              overlapSelectors: ['.ink-fp-tabs'],
            }),
          );

          // 5) 그리기 화면 도구 막대 + 색·굵기 패널 (?draw=1)
          await page.goto(`${baseUrl}/?draw=1&platform=${platform}&lang=${lang}`, {
            waitUntil: 'networkidle',
          });
          await page.evaluate((th) => {
            document.documentElement.dataset.theme = th;
          }, theme);
          await page.waitForSelector('.ink-draw-toolbar', { timeout: 5000 });
          await page.locator('.ink-draw-toolbar [data-action="style"]').click();
          await page.waitForSelector('.ink-cw-panel', { timeout: 5000 });
          results.push(
            await captureAndAudit(page, {
              id: 'draw-toolbar',
              label: '그리기 도구 막대 · 색·굵기 패널',
              lang,
              platform,
              theme,
              rootSelector: '.draw-preview',
              overlapSelectors: ['.ink-cw-head'],
            }),
          );

          // 6) 다크 고정 화면(단축키 한 장 · 섬 16종): dark 테마에서만 1회 촬영(light와 UI 동일)
          if (theme === 'dark') {
            await page.goto(`${baseUrl}/?screen=cheat&platform=${platform}&lang=${lang}&theme=dark&chrome=0`, {
              waitUntil: 'networkidle',
            });
            results.push(
              await captureAndAudit(page, {
                id: 'cheat-sheet',
                label: '단축키 한 장(S16)',
                lang,
                platform,
                theme: 'dark',
                rootSelector: '.ink-cheat-card',
                clipTarget: '.ink-cheat-card',
                overlapSelectors: ['.ink-cheat-head', '.ink-cheat-row'],
              }),
            );

            await page.goto(`${baseUrl}/?screen=island&platform=${platform}&lang=${lang}&theme=dark&chrome=0`, {
              waitUntil: 'networkidle',
            });
            results.push(
              await captureAndAudit(page, {
                id: 'island-gallery',
                label: '섬 알림 16종 모음',
                lang,
                platform,
                theme: 'dark',
                rootSelector: '.pv-island-page',
                overlapSelectors: ['.ink-island'],
              }),
            );
          }
        }
      }
    }

    await context.close();
  } finally {
    await browser.close();
  }

  // 7) PDF 레이아웃 모델 OS 표기·생성 점검 (Node에서 직접 실행 — PDF 미리보기 화면은 P1 미구현)
  const pdfAudit = await auditShortcutPdfModel();

  return { results, consoleErrors, pageErrors, pdfAudit };
}

async function auditShortcutPdfModel() {
  const layoutMod = await import(pathToFileURL(join(UI_PKG_DIR, 'src/lib/pdf/layoutModel.js')).href);
  const renderMod = await import(pathToFileURL(join(UI_PKG_DIR, 'src/lib/pdf/renderPdf.js')).href);
  const fakeMod = await import(pathToFileURL(join(ROOT, 'packages/ink-client/src/fake.js')).href);

  const checks = [];
  for (const platform of PLATFORMS) {
    for (const lang of LANGS) {
      const client = fakeMod.createFakeClient({ platform, lang, snipasteRunning: false });
      const st = await client.getState();
      const model = layoutMod.buildShortcutSheetModel({
        shortcuts: st.shortcuts,
        lang,
        os: platform,
        version: st.version,
        date: DATE_STR,
      });

      const texts = layoutMod.extractTextFragments(model);
      const joined = texts.join(' | ');
      const forbidden = platform === 'macos' ? WIN_ONLY_TERMS : MAC_ONLY_TERMS;
      const osMixes = [];
      for (const term of forbidden) {
        const re = new RegExp(term.source, term.flags);
        const m = re.exec(joined);
        if (m) {
          const idx = m.index;
          osMixes.push({
            term: term.name,
            snippet: joined.slice(Math.max(0, idx - 18), Math.min(joined.length, idx + m[0].length + 18)),
          });
        }
      }

      const t0 = performance.now();
      const bytes = await renderMod.buildShortcutSheetPdf({
        shortcuts: st.shortcuts,
        lang,
        os: platform,
        version: st.version,
        date: DATE_STR,
      });
      const renderMs = Math.round((performance.now() - t0) * 100) / 100;
      const header = Buffer.from(bytes.slice(0, 5)).toString('ascii');

      checks.push({
        platform,
        lang,
        validHeader: header === '%PDF-',
        bytes: bytes.length,
        renderMs,
        pageCount: (model.pages ?? []).length,
        heroCount: (model.hero ?? []).length,
        textFragmentCount: texts.length,
        osMixes,
      });
    }
  }
  return checks;
}

// ── 2. 성능 예산 측정 (R2-PERF-1) ──
async function measurePerfInBrowser(browserType, browserName, baseUrl) {
  console.log(`[성능 측정] ${browserName} 시작...`);
  const browser = await browserType.launch({ headless: true });

  const fontColdRuns = [];
  const fontWarmRuns = [];
  const colorPanelFirst = [];
  const colorPanelWarm = [];
  const fontPickerCold = [];
  const fontPickerWarm = [];
  const overlayReadyRuns = [];

  try {
    // 1) 글꼴 불러오기 — 콜드(새 컨텍스트, 캐시 비움) 5회
    for (let i = 0; i < 5; i++) {
      const ctx = await browser.newContext({ viewport: VIEWPORT });
      const page = await ctx.newPage();
      await page.goto(`${baseUrl}/?screen=settings&tab=shortcuts&platform=macos&lang=ko&theme=light&chrome=0`, {
        waitUntil: 'load',
      });
      const data = await page.evaluate(async () => {
        const tBeforeWait = performance.now();
        await document.fonts.ready;
        const readyAt = performance.now();
        const paints = performance.getEntriesByType('paint');
        const fcp = paints.find((p) => p.name === 'first-contentful-paint')?.startTime ?? null;
        const nav = performance.getEntriesByType('navigation')[0];
        const woffResources = performance
          .getEntriesByType('resource')
          .filter((r) => r.name.includes('.woff2'))
          .map((r) => ({
            name: r.name.split('/').pop(),
            startTime: Math.round(r.startTime * 100) / 100,
            responseEnd: Math.round(r.responseEnd * 100) / 100,
            duration: Math.round(r.duration * 100) / 100,
            transferSize: r.transferSize ?? 0,
          }));
        const loadedFaces = [...document.fonts]
          .filter((f) => f.status === 'loaded')
          .map((f) => ({ family: f.family, weight: f.weight, display: f.display }));
        const maxFontEnd = woffResources.reduce((m, r) => Math.max(m, r.responseEnd), 0);
        const maxFontDur = woffResources.reduce((m, r) => Math.max(m, r.duration), 0);
        const firstDisplayDelay =
          fcp !== null ? Math.max(0, readyAt - fcp) : Math.max(0, readyAt - (nav?.domContentLoadedEventEnd ?? tBeforeWait));
        return {
          readyAt: Math.round(readyAt * 100) / 100,
          fcp: fcp !== null ? Math.round(fcp * 100) / 100 : null,
          domContentLoaded: nav ? Math.round(nav.domContentLoadedEventEnd * 100) / 100 : null,
          firstDisplayDelay: Math.round(firstDisplayDelay * 100) / 100,
          maxFontDuration: Math.round(maxFontDur * 100) / 100,
          maxFontEnd: Math.round(maxFontEnd * 100) / 100,
          woffCount: woffResources.length,
          woffResources,
          loadedFaces,
        };
      });
      fontColdRuns.push(data);
      await ctx.close();
    }

    // 2) 글꼴 불러오기 — 웜(같은 컨텍스트 캐시 유지, 앱 내 로컬 번들/재열기 근사) 5회
    const warmCtx = await browser.newContext({ viewport: VIEWPORT });
    const warmPage = await warmCtx.newPage();
    await warmPage.goto(`${baseUrl}/?screen=settings&tab=shortcuts&platform=macos&lang=ko&theme=light&chrome=0`, {
      waitUntil: 'networkidle',
    });
    for (let i = 0; i < 5; i++) {
      await warmPage.goto(`${baseUrl}/?screen=tray&platform=macos&lang=ko&theme=light&chrome=0`, {
        waitUntil: 'load',
      });
      const data = await warmPage.evaluate(async () => {
        await document.fonts.ready;
        const readyAt = performance.now();
        const paints = performance.getEntriesByType('paint');
        const fcp = paints.find((p) => p.name === 'first-contentful-paint')?.startTime ?? null;
        const nav = performance.getEntriesByType('navigation')[0];
        const woffResources = performance
          .getEntriesByType('resource')
          .filter((r) => r.name.includes('.woff2'))
          .map((r) => ({
            name: r.name.split('/').pop(),
            duration: Math.round(r.duration * 100) / 100,
            transferSize: r.transferSize ?? 0,
          }));
        const maxFontDur = woffResources.reduce((m, r) => Math.max(m, r.duration), 0);
        const firstDisplayDelay =
          fcp !== null ? Math.max(0, readyAt - fcp) : Math.max(0, readyAt - (nav?.domContentLoadedEventEnd ?? readyAt));
        return {
          readyAt: Math.round(readyAt * 100) / 100,
          fcp: fcp !== null ? Math.round(fcp * 100) / 100 : null,
          firstDisplayDelay: Math.round(firstDisplayDelay * 100) / 100,
          maxFontDuration: Math.round(maxFontDur * 100) / 100,
          woffCount: woffResources.length,
        };
      });
      fontWarmRuns.push(data);
    }
    await warmCtx.close();

    // 3) 색·굵기 패널 열기(<= 16 ms) + 글꼴 패널 첫 열기(<= 120 ms)
    //    매 회 새 페이지에서 콜드 첫 열기를 잰 뒤, 같은 페이지에서 재열기(웜)도 측정한다.
    const ctx = await browser.newContext({ viewport: VIEWPORT });
    const page = await ctx.newPage();

    const openConfirmedOverlay = async () => {
      await page.goto(`${baseUrl}/?screen=overlay&platform=macos&lang=ko&theme=dark&chrome=0`, {
        waitUntil: 'networkidle',
      });
      await settle(page, 60);
      await page.locator('.pv-overlay-launcher .pv-bar-group').nth(1).locator('button').first().click();
      await page.waitForSelector('.ink-overlay', { timeout: 5000 });
      await settle(page, 60);

      // overlayReady timing 읽기 (미리보기 하단 텍스트 또는 DOM 안정 확인)
      await page.mouse.move(320, 200);
      await page.mouse.down();
      await page.mouse.move(920, 480, { steps: 4 });
      await page.mouse.up();
      await page.waitForSelector('.ink-toolbar', { timeout: 5000 });
      await settle(page, 40);
    };

    // 브라우저 내부 performance.now()로 클릭 → 대상 DOM 표시 + rAF 1회(첫 프레임) / rAF 2회(합성 완료) 정밀 측정
    const measureClickToSelector = async (clickSel, targetSel) => {
      return page.evaluate(
        ({ clickSel, targetSel }) =>
          new Promise((resolve, reject) => {
            const btn = document.querySelector(clickSel);
            if (!btn) {
              reject(new Error(`버튼을 찾지 못함: ${clickSel}`));
              return;
            }
            const t0 = performance.now();
            let domMs = null;
            const finish = () => {
              const tDom = domMs ?? performance.now() - t0;
              requestAnimationFrame(() => {
                const raf1Ms = performance.now() - t0;
                requestAnimationFrame(() => {
                  const raf2Ms = performance.now() - t0;
                  resolve({
                    domMs: Math.round(tDom * 100) / 100,
                    raf1Ms: Math.round(raf1Ms * 100) / 100,
                    raf2Ms: Math.round(raf2Ms * 100) / 100,
                  });
                });
              });
            };

            const mo = new MutationObserver(() => {
              const el = document.querySelector(targetSel);
              if (el) {
                domMs = performance.now() - t0;
                mo.disconnect();
                finish();
              }
            });
            mo.observe(document.body, { childList: true, subtree: true, attributes: true });
            const timer = setTimeout(() => {
              mo.disconnect();
              reject(new Error(`대상 표시 시간 초과: ${targetSel}`));
            }, 4000);

            btn.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
            // 동기적으로 바로 붙은 경우
            if (document.querySelector(targetSel)) {
              clearTimeout(timer);
              domMs = performance.now() - t0;
              mo.disconnect();
              finish();
            } else {
              const orig = mo.disconnect.bind(mo);
              mo.disconnect = () => {
                clearTimeout(timer);
                orig();
              };
            }
          }),
        { clickSel, targetSel },
      );
    };

    // 콜드 첫 열기 5회 (매 회 페이지 새로 열기)
    for (let i = 0; i < 5; i++) {
      await openConfirmedOverlay();

      // 펜 도구 선택 후 색·굵기 패널 첫 열기
      await page.locator('.ink-toolbar [data-tool="pen"]').click();
      await settle(page, 40);
      const cw = await measureClickToSelector('.ink-toolbar [data-action="style"]', '.ink-cw-panel');
      colorPanelFirst.push(cw);

      // 글자 도구 전환 후 글꼴 패널 첫 열기
      await page.locator('.ink-toolbar [data-tool="text"]').click();
      await settle(page, 40);
      if ((await page.locator('.ink-cw-panel').count()) === 0) {
        await page.locator('.ink-toolbar [data-action="style"]').click();
        await page.waitForSelector('.ink-cw-panel', { timeout: 3000 });
      }
      const fp = await measureClickToSelector('.ink-cw-panel [data-action="font-picker"]', '.ink-fp[data-ready="true"]');
      fontPickerCold.push(fp);
    }

    // 웜(같은 페이지) 반복 열기 10회
    await openConfirmedOverlay();
    await page.locator('.ink-toolbar [data-tool="pen"]').click();
    await settle(page, 40);
    for (let i = 0; i < 10; i++) {
      const cw = await measureClickToSelector('.ink-toolbar [data-action="style"]', '.ink-cw-panel');
      colorPanelWarm.push(cw);
      // 닫기
      await page.locator('.ink-toolbar [data-action="style"]').click();
      await page.waitForSelector('.ink-cw-panel', { state: 'detached', timeout: 3000 });
      await settle(page, 20);
    }

    // 글자 도구 + 글꼴 패널 웜 재열기 10회
    await page.locator('.ink-toolbar [data-tool="text"]').click();
    await settle(page, 40);
    await page.locator('.ink-toolbar [data-action="style"]').click();
    await page.waitForSelector('.ink-cw-panel', { timeout: 3000 });
    await settle(page, 30);
    for (let i = 0; i < 10; i++) {
      const fp = await measureClickToSelector('.ink-cw-panel [data-action="font-picker"]', '.ink-fp[data-ready="true"]');
      fontPickerWarm.push(fp);
      await page.locator('.ink-cw-panel [data-action="font-picker"]').click();
      await page.waitForSelector('.ink-fp', { state: 'detached', timeout: 3000 });
      await settle(page, 20);
    }

    // 캡처 오버레이 열기(가짜 클라이언트 미리보기 기준 overlayReady 소요 시간) 5회 참고 측정
    await page.goto(`${baseUrl}/?screen=overlay&platform=macos&lang=ko&theme=dark&chrome=0`, {
      waitUntil: 'networkidle',
    });
    for (let i = 0; i < 5; i++) {
      await settle(page, 40);
      await page.locator('.pv-overlay-launcher .pv-bar-group').nth(1).locator('button').first().click();
      await page.waitForSelector('.ink-overlay', { timeout: 5000 });
      await settle(page, 80);
      await page.keyboard.press('Escape');
      await page.waitForSelector('.ink-overlay', { state: 'detached', timeout: 5000 });
      const timingText = await page
        .locator('.pv-overlay-launcher .ink-mono')
        .first()
        .textContent()
        .catch(() => '');
      if (timingText) overlayReadyRuns.push(timingText.trim());
    }

    await ctx.close();
  } finally {
    await browser.close();
  }

  return {
    browser: browserName,
    fontCold: {
      runs: fontColdRuns,
      firstDisplayDelay: stats(fontColdRuns.map((r) => r.firstDisplayDelay)),
      maxFontDuration: stats(fontColdRuns.map((r) => r.maxFontDuration)),
      readyAt: stats(fontColdRuns.map((r) => r.readyAt)),
      sampleWoff: fontColdRuns[0]?.woffResources ?? [],
      loadedFaces: fontColdRuns[0]?.loadedFaces ?? [],
    },
    fontWarm: {
      runs: fontWarmRuns,
      firstDisplayDelay: stats(fontWarmRuns.map((r) => r.firstDisplayDelay)),
      maxFontDuration: stats(fontWarmRuns.map((r) => r.maxFontDuration)),
      readyAt: stats(fontWarmRuns.map((r) => r.readyAt)),
    },
    colorWidthPanel: {
      coldFirst: {
        domMs: stats(colorPanelFirst.map((r) => r.domMs)),
        raf1Ms: stats(colorPanelFirst.map((r) => r.raf1Ms)),
        raf2Ms: stats(colorPanelFirst.map((r) => r.raf2Ms)),
        raw: colorPanelFirst,
      },
      warmReopen: {
        domMs: stats(colorPanelWarm.map((r) => r.domMs)),
        raf1Ms: stats(colorPanelWarm.map((r) => r.raf1Ms)),
        raf2Ms: stats(colorPanelWarm.map((r) => r.raf2Ms)),
        raw: colorPanelWarm,
      },
    },
    fontPicker: {
      coldFirst: {
        domMs: stats(fontPickerCold.map((r) => r.domMs)),
        raf1Ms: stats(fontPickerCold.map((r) => r.raf1Ms)),
        raf2Ms: stats(fontPickerCold.map((r) => r.raf2Ms)),
        raw: fontPickerCold,
      },
      warmReopen: {
        domMs: stats(fontPickerWarm.map((r) => r.domMs)),
        raf1Ms: stats(fontPickerWarm.map((r) => r.raf1Ms)),
        raf2Ms: stats(fontPickerWarm.map((r) => r.raf2Ms)),
        raw: fontPickerWarm,
      },
    },
    overlayReadyNote: overlayReadyRuns,
  };
}

// ── 메인 ──
async function main() {
  const port = await findFreePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  console.log(`[미리보기 서버] Vite 기동 중: ${baseUrl}`);
  const vite = startViteServer(port);

  const summary = {
    date: DATE_STR,
    startedAt: new Date().toISOString(),
    baseUrl,
    viewport: VIEWPORT,
    snapshots: null,
    perf: null,
  };

  try {
    await waitForHttp(`${baseUrl}/`);
    console.log(`[미리보기 서버] 준비 완료`);

    if (ONLY === 'all' || ONLY === 'snapshots') {
      summary.snapshots = await runSnapshots(baseUrl);
    }

    if (ONLY === 'all' || ONLY === 'perf') {
      summary.perf = {
        chromium: await measurePerfInBrowser(chromium, 'chromium-1248', baseUrl),
      };
      if (INCLUDE_WEBKIT_PERF) {
        try {
          summary.perf.webkit = await measurePerfInBrowser(webkit, 'webkit-2370', baseUrl);
        } catch (err) {
          summary.perf.webkitError = String(err?.message ?? err);
        }
      }
    }

    summary.finishedAt = new Date().toISOString();
    mkdirSync(dirname(JSON_OUT), { recursive: true });
    writeFileSync(JSON_OUT, JSON.stringify(summary, null, 2));
    console.log(`[완료] 원본 결과 저장: ${JSON_OUT}`);

    // 핵심 요약 출력
    if (summary.snapshots) {
      const items = summary.snapshots.results;
      const withOverflow = items.filter((r) => r.audit.overflows.length > 0);
      const withTrunc = items.filter((r) => r.audit.truncations.length > 0);
      const withBounds = items.filter((r) => r.audit.boundsViolations.length > 0);
      const withClip = items.filter((r) => r.audit.viewportClips.length > 0);
      const withOverlap = items.filter((r) => r.audit.overlaps.length > 0);
      const withOs = items.filter((r) => r.audit.osMixes.length > 0);
      const notFound = items.filter((r) => !r.audit.found);
      console.log(
        `[스냅숏 요약] 총 ${items.length}장 | 미발견=${notFound.length} | 넘침=${withOverflow.length} | 잘림(말줄임)=${withTrunc.length} | 경계이탈=${withBounds.length} | 뷰포트잘림=${withClip.length} | 겹침=${withOverlap.length} | OS표기섞임=${withOs.length} | 콘솔오류=${summary.snapshots.consoleErrors.length} | 페이지오류=${summary.snapshots.pageErrors.length}`,
      );
    }
    if (summary.perf?.chromium) {
      const p = summary.perf.chromium;
      console.log(
        `[성능 요약 chromium] 글꼴 첫 표시(cold p50)=${p.fontCold.firstDisplayDelay.p50}ms (warm p50=${p.fontWarm.firstDisplayDelay.p50}ms) | 색·굵기 패널(cold raf1 p50)=${p.colorWidthPanel.coldFirst.raf1Ms.p50}ms (dom p50=${p.colorWidthPanel.coldFirst.domMs.p50}ms, warm raf1 p50=${p.colorWidthPanel.warmReopen.raf1Ms.p50}ms) | 글꼴 패널 첫 열기(cold raf1 p50)=${p.fontPicker.coldFirst.raf1Ms.p50}ms (dom p50=${p.fontPicker.coldFirst.domMs.p50}ms)`,
      );
    }
    if (summary.perf?.webkit) {
      const p = summary.perf.webkit;
      console.log(
        `[성능 요약 webkit] 글꼴 첫 표시(cold p50)=${p.fontCold.firstDisplayDelay.p50}ms (warm p50=${p.fontWarm.firstDisplayDelay.p50}ms) | 색·굵기 패널(cold raf1 p50)=${p.colorWidthPanel.coldFirst.raf1Ms.p50}ms (dom p50=${p.colorWidthPanel.coldFirst.domMs.p50}ms, warm raf1 p50=${p.colorWidthPanel.warmReopen.raf1Ms.p50}ms) | 글꼴 패널 첫 열기(cold raf1 p50)=${p.fontPicker.coldFirst.raf1Ms.p50}ms (dom p50=${p.fontPicker.coldFirst.domMs.p50}ms)`,
      );
    }
  } finally {
    await vite.stop();
    console.log('[미리보기 서버] 종료 완료');
  }
}

main().catch((err) => {
  console.error('[오류]', err);
  process.exit(1);
});
