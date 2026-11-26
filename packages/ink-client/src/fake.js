// FakeInkClient — 브라우저 미리보기·UI 시험용. 확정본 v1과 단축키 카탈로그를 그대로 쓰고,
// "이 PC에 Snipaste가 켜져 있다"는 장면(F04 시안)을 흉내 낸다. 판정은 단순화본(진짜는 Rust ink-domain).
import defaults from '../../../docs/shortcuts/defaults-v1.json' with { type: 'json' };
import catalog from '../../../docs/shortcuts/catalog-v1.json' with { type: 'json' };

const SNIPASTE = new Set(['F1', 'F3', 'Shift+F1', 'Ctrl+F1']);
const SUGGEST = ['Ctrl+Alt+1', 'Ctrl+Alt+2', 'Ctrl+Alt+3', 'Alt+Shift+2', 'Alt+Shift+3', 'Ctrl+Shift+F1', 'Alt+Shift+F1', 'Ctrl+Alt+Q'];

const DEFAULT_SETTINGS = {
  'capture.result': 'copy', 'capture.evidence': false, 'capture.cursor': false, 'capture.sound': false, 'capture.element': true, 'capture.magnet': true,
  'capture.historyMax': 20, 'capture.excluded': [{ exe: 'licecap.exe', mode: 'exclude', auto: false }],
  'output.folder': 'C:\\Users\\…\\Pictures\\screen-ink', 'output.template': '수업_{yyyy}{MM}{dd}_{n}',
  'record.fps': 30, 'record.mic': false, 'record.micDevice': '',
  'draw.color': 'R', 'draw.width': 4, 'draw.smoothing': 'light', 'draw.straighten': true, 'draw.shapeOnHold': true,
  'draw.startZoom': 1.75, 'draw.keep': true, 'draw.esc': 'step', 'draw.rightClickPan': true,
  'zoom.follow': 'smooth', 'zoom.minimap': true,
  'general.theme': 'system', 'general.lang': 'ko', 'general.autostart': true,
  'topmost.border': 4, 'topmost.sound': false,
};

const CAPTURE_ISLAND = {
  region: { text: '범위 캡처', variant: 'region', hints: [{ key: '휠', label: '넓히기·좁히기' }, { key: 'Tab', label: '창' }, { key: 'Enter', label: '복사' }, { key: 'Esc', label: '취소' }] },
  window: { text: '창 캡처', variant: 'window', hints: [{ key: '클릭', label: '찍기' }, { key: 'F2', label: '활성 창' }, { key: 'Esc', label: '취소' }] },
  record: { text: '녹화 범위', variant: 'region', hints: [{ key: '드래그', label: '범위' }, { key: '클릭', label: '모니터' }, { key: 'Esc', label: '취소' }] },
};
const CAPTURE_ACTION = { 'capture.region': 'region', 'capture.window': 'window', 'record.toggle': 'record' };

/**
 * @param {{platform?: 'windows'|'macos', snipasteRunning?: boolean, frameUrl?: string}} [opts]
 *   frameUrl = 미리보기에서 오버레이 바탕으로 쓸 정지 이미지(없으면 오버레이가 빈 바탕)
 */
export function createFakeClient({ platform = 'windows', snipasteRunning = true, frameUrl = undefined } = {}) {
  const combos = Object.fromEntries(Object.entries(defaults.slots).map(([id, v]) => [id, (platform === 'macos' ? v.mac : v.win) || '']));
  const settings = { ...DEFAULT_SETTINGS };
  const history = [];
  const subs = new Set();
  let hotkeysEnabled = true;
  let mode = { mode: 'idle' };
  let sessionSeq = 0;
  /** @type {any} 가짜 고르기 상태(세션마다 새로) */
  let capture = null;
  const readyLog = [];

  const judge = (slot, combo) => {
    if (!combo) return { status: 'off' };
    const dup = catalog.slots.find((o) => o.id !== slot.id && combos[o.id] === combo && (o.scope === slot.scope || o.scope === 'global' || slot.scope === 'global'));
    if (dup) return { status: 'conflict', reason: `‘${dup.label}’과 겹쳐요`, owner: 'screen-ink' };
    if (slot.scope === 'global' && snipasteRunning && SNIPASTE.has(combo)) return { status: 'conflict', reason: 'Snipaste가 먼저 쓰고 있어요', owner: 'Snipaste' };
    if (slot.scope === 'global' && /^[A-Z0-9]$/.test(combo)) return { status: 'conflict', reason: '글자 하나는 전역 키로 쓸 수 없어요' };
    return { status: 'ok' };
  };
  const rows = () => catalog.slots
    .filter((s) => !(platform === 'macos' && s.windowsOnly))
    .map((s) => ({ id: s.id, scope: s.scope, group: s.group, label: s.label, combo: combos[s.id], later: s.later, windowsOnly: s.windowsOnly, ...judge(s, combos[s.id]) }));
  const state = () => {
    const r = rows();
    const bad = r.filter((x) => x.status === 'conflict' && x.scope === 'global');
    const owners = [...new Set(bad.map((x) => x.owner))];
    return {
      platform, theme: settings['general.theme'], hotkeysEnabled, pinnedCount: 2, recording: { active: false, elapsedMs: 0 },
      settings: { ...settings }, shortcuts: r, version: '0.1.0',
      conflicts: { count: bad.length, app: owners.length === 1 && owners[0] !== 'screen-ink' ? owners[0] : undefined, combos: bad.map((x) => x.combo) },
      readonly: false,
      location: { path: 'C:\\Users\\…\\AppData\\Roaming\\screen-ink\\settings.json', portable: false, readOnly: false },
      notices: [],
      mode,
    };
  };
  // 가져오기 장면: 다른 PC에서 내보낸 파일(3개가 다름)
  const IMPORT_FILE = { 'draw.width': 8, 'general.theme': 'dark', 'shortcuts.active': 'Ctrl+Alt+F2' };
  const valueOf = (path) => (path.startsWith('shortcuts.') ? combos[path.slice(10)] : settings[path]);
  const emit = (e) => subs.forEach((h) => h(e));
  const island = (island) => emit({ type: 'island', island });
  const changed = () => emit({ type: 'state', state: state() });

  return {
    async getState() { return state(); },
    async updateSetting(path, value) {
      if (!(path in settings)) return { ok: false, error: `모르는 설정: ${path}`, state: state() };
      history.push({ kind: 'setting', path, prev: settings[path] });
      settings[path] = value; changed();
      return { ok: true, state: state() };
    },
    async previewShortcut(id, combo) { const s = catalog.slots.find((x) => x.id === id); return judge(s, combo); },
    async setShortcut(id, combo) {
      const slot = catalog.slots.find((x) => x.id === id);
      const prev = combos[id];
      combos[id] = combo;
      const j = judge(slot, combo);
      if (j.status === 'conflict') {
        combos[id] = prev;
        const used = new Set(Object.values(combos));
        return { ok: false, status: 'conflict', reason: j.reason, suggestions: SUGGEST.filter((c) => !used.has(c) && !SNIPASTE.has(c)).slice(0, 3) };
      }
      history.push({ kind: 'shortcut', id, prev });
      changed();
      island({ kind: 'result', text: combo ? `${combo.replaceAll('+', ' ')}로 바꿨어요` : '비웠어요 · 트레이 메뉴로 시작할 수 있어요', button: { label: '되돌리기', action: 'undo' } });
      return { ok: true, status: j.status };
    },
    async resetShortcutGroup(group) {
      for (const s of catalog.slots.filter((x) => x.group === group)) combos[s.id] = (platform === 'macos' ? defaults.slots[s.id].mac : defaults.slots[s.id].win) || '';
      changed(); return state();
    },
    async undo() {
      const h = history.pop();
      if (!h) return { ok: false, state: state() };
      if (h.kind === 'setting') settings[h.path] = h.prev;
      else if (h.kind === 'whole') { Object.assign(settings, h.settings); Object.assign(combos, h.combos); }
      else combos[h.id] = h.prev;
      changed(); return { ok: true, state: state() };
    },
    async previewImport() {
      const changes = Object.entries(IMPORT_FILE)
        .filter(([path, after]) => valueOf(path) !== after)
        .map(([path, after]) => ({ path, before: valueOf(path), after }));
      return { cancelled: false, path: 'C:\\Users\\…\\Desktop\\screen-ink-settings-2026-11-25.json', ok: true, changes, warnings: [] };
    },
    async applyImport() {
      history.push({ kind: 'whole', settings: { ...settings }, combos: { ...combos } });
      for (const [path, after] of Object.entries(IMPORT_FILE)) {
        if (path.startsWith('shortcuts.')) combos[path.slice(10)] = after; else settings[path] = after;
      }
      changed();
      island({ kind: 'result', text: '설정을 가져왔어요', button: { label: '되돌리기', action: 'undo' } });
      return { ok: true, state: state() };
    },
    async resetAll() {
      Object.assign(settings, DEFAULT_SETTINGS);
      for (const id of Object.keys(combos)) combos[id] = (platform === 'macos' ? defaults.slots[id].mac : defaults.slots[id].win) || '';
      history.length = 0;
      changed();
      island({ kind: 'result', text: '처음 상태로 되돌렸어요' });
      return state();
    },
    async setHotkeysEnabled(enabled) {
      hotkeysEnabled = enabled; changed();
      island(enabled ? { kind: 'info', text: '단축키를 다시 켰어요' } : { kind: 'off', text: '단축키를 잠시 껐어요', detail: 'Alt+`' });
      return state();
    },
    // 고르기 흉내: 커서 주변에 큰 것 → 작은 것 사슬(화면 › 창 › 표 › 2행)을 만들고 휠로 단계를 바꾼다.
    async captureInput(input) {
      if (mode.mode !== 'capture') return;
      const s = (capture ??= { phase: 'hover', level: 3, p: { x: 760, y: 420 }, rect: null, down: null, help: false });
      const chainAt = (p) => [
        { name: '화면', r: { x: 0, y: 0, w: 1920, h: 1080 } },
        { name: '창', r: { x: 240, y: 120, w: 1440, h: 820 } },
        { name: '표', r: { x: Math.max(240, p.x - 360), y: Math.max(120, p.y - 120), w: 720, h: 240 } },
        { name: '2행', r: { x: Math.max(240, p.x - 360), y: p.y - 20, w: 720, h: 40 } },
      ];
      if (input.type === 'move') {
        s.p = { x: input.x, y: input.y };
        if (s.down) {
          s.phase = 'drag';
          s.rect = { x: Math.min(s.down.x, s.p.x), y: Math.min(s.down.y, s.p.y), w: Math.abs(s.p.x - s.down.x) + 1, h: Math.abs(s.p.y - s.down.y) + 1 };
        }
      } else if (input.type === 'wheel' && s.phase === 'hover') {
        s.level = Math.max(0, Math.min(3, s.level + input.dy));
      } else if (input.type === 'down' && input.button === 0) {
        s.down = { x: input.x, y: input.y };
      } else if (input.type === 'up' && input.button === 0) {
        if (s.phase === 'hover') { s.rect = chainAt(s.p)[s.level].r; }
        s.phase = 'confirmed';
        s.down = null;
      } else if (input.type === 'down' && input.button === 2) {
        if (s.phase === 'hover') return this.run('capture.cancel');
        s.phase = 'hover'; s.rect = null;
      } else if ((input.type === 'key' && input.key === 'Escape') || (input.type === 'command' && input.name === 'cancel')) {
        return this.run('capture.cancel');
      } else if (input.type === 'key' && input.key === '?') {
        s.help = !s.help;
      }
      if (input.type === 'command' && (input.name === 'save' || input.name === 'quickSave')) {
        const r = s.rect ?? chainAt(s.p)[s.level].r;
        await this.run('capture.cancel');
        island({ kind: 'result', variant: 'result', text: '저장했어요', value: `${r.w}×${r.h}`, detail: '수업_2026-03-04_090507.png', thumbUrl: frameUrl, button: { label: '폴더 열기', action: 'folder.open' } });
        return;
      }
      if ((input.type === 'key' && input.key === 'Enter') || input.type === 'dblclick' || (input.type === 'command' && input.name === 'copy')) {
        const r = s.rect ?? chainAt(s.p)[s.level].r;
        await this.run('capture.cancel');
        island({ kind: 'result', variant: 'result', text: '복사했어요', value: `${r.w}×${r.h}`, detail: '한글·PPT에 Ctrl+V', thumbUrl: frameUrl, button: { label: '저장', action: 'capture.saveLast' }, buttonKey: 'Ctrl+S', chip: { label: '#FFFFFF 복사', action: 'capture.copyHex' } });
        return;
      }
      const chain = chainAt(s.p);
      const rect = s.phase === 'hover' ? chain[s.level].r : s.rect;
      emit({
        type: 'capture',
        view: {
          session: mode.session, phase: s.phase, rect, path: chain.map((c) => c.name), level: s.phase === 'hover' ? s.level : -1, guides: [], magnet: !input.alt, cursorIncluded: false, help: s.help,
          // 숨은 설정 capture.evidence: 작은 것 → 큰 것
          ...(settings['capture.evidence'] && s.phase === 'hover'
            ? { evidence: [...chain].reverse().map((c, i) => ({ rect: c.r, label: c.name, score: [0.86, 0.74, 0.61, 0.3][i], sources: ['W·A·V', 'W·V', 'W', 'W'][i], current: chain.length - 1 - i === s.level, hidden: false })) }
            : {}),
        },
      });
    },
    async overlayReady(session) {
      if (mode.mode === 'capture' && mode.session === session) readyLog.push(session);
    },
    /** 시험용: overlayReady가 받아들인 세션들 */
    get readyLog() { return [...readyLog]; },
    async run(action) {
      if (CAPTURE_ACTION[action]) {
        const kind = CAPTURE_ACTION[action];
        sessionSeq += 1;
        mode = { mode: 'capture', session: sessionSeq, kind, startedMs: Date.now() };
        const bounds = { x: 0, y: 0, w: 1920, h: 1080 };
        emit({
          type: 'overlay',
          overlay: {
            session: sessionSeq,
            kind,
            cursor: { x: 760, y: 420 },
            nativeFrame: false,
            highlight: { x: 480, y: 220, w: 960, h: 560 },
            monitors: [{ id: 0, bounds, work: { ...bounds, h: 1040 }, scale: 1, primary: true, frameUrl }],
          },
        });
        changed();
        island({ kind: 'mode', mode: kind, ...CAPTURE_ISLAND[kind] });
        return;
      }
      if (action === 'capture.cancel') {
        if (mode.mode !== 'capture') return;
        mode = { mode: 'idle' };
        capture = null;
        changed();
        island({ kind: 'end', text: '' });
        return;
      }
      if (action === 'folder.pick') {
        history.push({ kind: 'setting', path: 'output.folder', prev: settings['output.folder'] });
        settings['output.folder'] = 'D:\\수업자료\\캡처'; changed(); return;
      }
      if (action === 'settings.export') { island({ kind: 'result', text: '설정을 내보냈어요' }); return; }
      if (action === 'capture.history.clear') { island({ kind: 'info', text: '지울 기록이 없어요' }); return; }
      island({ kind: 'info', text: `미리보기: ${action}` });
    },
    subscribe(handler) { subs.add(handler); return () => subs.delete(handler); },
  };
}
