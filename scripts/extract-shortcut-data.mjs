// 단축키 입력표(docs/shortcuts/shortcut-picker.html)의 SLOTS·SRC·REF를 꺼내
// docs/shortcuts/catalog-v1.json으로 쓴다. Rust(ink-domain)와 UI가 이 한 파일을 같이 읽어
// "입력표와 앱의 판정이 같다"(PRD R-KEY-5)를 지킨다. 사용: npm run shortcuts:data
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import vm from 'node:vm';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const html = readFileSync(path.join(root, 'docs/shortcuts/shortcut-picker.html'), 'utf8');
const script = html.match(/<script>([\s\S]*)<\/script>/)[1];
const head = script.slice(0, script.indexOf('/* ─────────────── 3.'));
const sb = {};
vm.runInNewContext(`${head}\nglobalThis.__out = { SLOTS, GROUPS, SRC, REF };`, sb);
const { SLOTS, SRC, REF } = sb.__out;

// 화면 라벨(12자 안)과 설정 창 묶음 — 04-screens S02
const LABEL = { full: '모니터 전체 바로 캡처', active: '활성 창 바로 캡처', last: '직전 범위 바로 캡처', pause: '단축키 켜기·끄기', topmost: '창 항상 위', restore: '직전 판서 열기', pinToggle: '핀 숨기기·보이기', pin: '화면에 붙이기' };
const GLOBAL_GROUP = { region: 'capture', window: 'capture', record: 'capture', last: 'capture', full: 'capture', active: 'capture', delay: 'capture', draw: 'tools', zoom: 'tools', board: 'tools', topmost: 'tools', restore: 'quick', pause: 'quick', pin: 'quick', pinToggle: 'quick', timer: 'quick' };
const MODE_GROUP = { capture: 'capture-mode', draw: 'draw-mode', zoom: 'zoom-mode' };

const slots = SLOTS.map((s) => ({
  id: s.id,
  scope: s.g,
  group: s.g === 'global' ? GLOBAL_GROUP[s.id] : MODE_GROUP[s.g],
  label: LABEL[s.id] || s.name,
  required: !!s.need,
  later: s.opt === '후속',
  windowsOnly: !!s.noMac,
}));
const sources = Object.fromEntries(Object.entries(SRC).map(([k, v]) => [k, { label: v.label, level: v.lv, keepKey: v.keepKey || null }]));
const refs = REF.map(([src, platform, combo, what, level]) => ({ src, platform, combo, what, level: level || null }));

const catalog = {
  schema: 'screen-ink.shortcuts.catalog',
  version: 1,
  generatedFrom: 'docs/shortcuts/shortcut-picker.html',
  rules: {
    modOrder: { win: ['Ctrl', 'Alt', 'Shift', 'Win'], mac: ['Ctrl', 'Option', 'Shift', 'Cmd'] },
    imeKeys: ['한/영', '한자', 'Caps Lock'],
    specialAlone: ['PrtScn', 'Pause', 'ScrLk', 'Insert'],
    modeCheckSources: ['win', 'ime', 'mac'],
    defaultKeep: { snipaste: false, icanzoomit: false, zoomit: false, powertoys: true, powertoysAot: false },
  },
  slots,
  sources,
  refs,
};
const out = path.join(root, 'docs/shortcuts/catalog-v1.json');
writeFileSync(out, JSON.stringify(catalog, null, 2) + '\n');
console.log(`slots ${slots.length} · sources ${Object.keys(sources).length} · refs ${refs.length} → ${path.relative(root, out)}`);
