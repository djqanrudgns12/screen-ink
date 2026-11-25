// FakeInkClient — 브라우저 미리보기·UI 시험용. 확정본 v1과 단축키 카탈로그를 그대로 쓰고,
// "이 PC에 Snipaste가 켜져 있다"는 장면(F04 시안)을 흉내 낸다. 판정은 단순화본(진짜는 Rust ink-domain).
import defaults from '../../../docs/shortcuts/defaults-v1.json' with { type: 'json' };
import catalog from '../../../docs/shortcuts/catalog-v1.json' with { type: 'json' };

const SNIPASTE = new Set(['F1', 'F3', 'Shift+F1', 'Ctrl+F1']);
const SUGGEST = ['Ctrl+Alt+1', 'Ctrl+Alt+2', 'Ctrl+Alt+3', 'Alt+Shift+2', 'Alt+Shift+3', 'Ctrl+Shift+F1', 'Alt+Shift+F1', 'Ctrl+Alt+Q'];

const DEFAULT_SETTINGS = {
  'capture.result': 'copy', 'capture.cursor': false, 'capture.sound': false, 'capture.element': true, 'capture.magnet': true,
  'capture.historyMax': 20, 'capture.excluded': [{ exe: 'licecap.exe', mode: 'exclude', auto: false }],
  'output.folder': 'C:\\Users\\…\\Pictures\\screen-ink', 'output.template': '수업_{yyyy}{MM}{dd}_{n}',
  'record.fps': 30, 'record.mic': false, 'record.micDevice': '',
  'draw.color': 'R', 'draw.width': 4, 'draw.smoothing': 'light', 'draw.straighten': true, 'draw.shapeOnHold': true,
  'draw.startZoom': 1.75, 'draw.keep': true, 'draw.esc': 'step', 'draw.rightClickPan': true,
  'zoom.follow': 'smooth', 'zoom.minimap': true,
  'general.theme': 'system', 'general.lang': 'ko', 'general.autostart': true,
  'topmost.border': 4, 'topmost.sound': false,
};

export function createFakeClient({ platform = 'windows', snipasteRunning = true } = {}) {
  const combos = Object.fromEntries(Object.entries(defaults.slots).map(([id, v]) => [id, (platform === 'macos' ? v.mac : v.win) || '']));
  const settings = { ...DEFAULT_SETTINGS };
  const history = [];
  const subs = new Set();
  let hotkeysEnabled = true;

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
    async run(action) {
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
