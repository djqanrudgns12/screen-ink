// FakeInkClient — 브라우저 미리보기·UI 시험용. 확정본 v1과 단축키 카탈로그를 그대로 쓰고,
// "이 PC에 Snipaste가 켜져 있다"는 장면(F04 시안)을 흉내 낸다. 판정은 단순화본(진짜는 Rust ink-domain).
import defaults from '../../../docs/shortcuts/defaults-v1.json' with { type: 'json' };
import catalog from '../../../docs/shortcuts/catalog-v1.json' with { type: 'json' };

const SNIPASTE = new Set(['F1', 'F3', 'Shift+F1', 'Ctrl+F1']);
const SUGGEST = ['Ctrl+Alt+1', 'Ctrl+Alt+2', 'Ctrl+Alt+3', 'Alt+Shift+2', 'Alt+Shift+3', 'Ctrl+Shift+F1', 'Alt+Shift+F1', 'Ctrl+Alt+Q'];

const DEFAULT_SETTINGS = {
  'capture.result': 'copy', 'capture.evidence': false, 'capture.cursor': false, 'capture.sound': false, 'capture.element': true, 'capture.magnet': true,
  'capture.historyMax': 20, 'capture.excluded': [{ exe: 'licecap.exe', mode: 'exclude', auto: false }],
  'output.folder': 'C:\\Users\\…\\Pictures\\Shookshok', 'output.template': '수업_{yyyy}{MM}{dd}_{n}',
  'record.fps': 30, 'record.mic': false, 'record.micDevice': '', 'record.cursor': true, 'record.firstHintDone': false, 'capture.delay': 3,
  'draw.color': 'R', 'draw.width': 4, 'annotate.style': '', 'annotate.font': '', 'update.auto': true, 'update.snooze': '', 'draw.smoothing': 'light', 'draw.straighten': true, 'draw.shapeOnHold': true, 'draw.shapeApply': 'suggest',
  'draw.startZoom': 1.75, 'draw.keep': true, 'draw.esc': 'step', 'draw.rightClickPan': true,
  'zoom.follow': 'pointer', 'zoom.minimap': true,
  'general.theme': 'system', 'general.lang': 'ko', 'general.autostart': true,
  'onboarding.done': false, 'onboarding.step': 0,
  'topmost.border': 4, 'topmost.sound': false,
};

const CAPTURE_ISLAND = {
  region: { text: '범위 캡처', variant: 'region', hints: [{ key: '휠', label: '넓히기·좁히기' }, { key: 'Tab', label: '창' }, { key: 'Enter', label: '복사' }, { key: 'Esc', label: '취소' }] },
  window: { text: '창 캡처', variant: 'window', hints: [{ key: '클릭', label: '찍기' }, { key: 'F2', label: '활성 창' }, { key: 'Esc', label: '취소' }] },
  record: { text: '녹화 범위', variant: 'region', hints: [{ key: '드래그', label: '범위' }, { key: '클릭', label: '모니터' }, { key: 'Esc', label: '취소' }] },
};
const CAPTURE_ACTION = { 'capture.region': 'region', 'capture.window': 'window', 'record.toggle': 'record' };
const DRAW_ACTION = { draw: 'draw', board: 'board', 'draw.restore': 'restore' };
const DRAW_TOOLS = { pen: '펜', marker: '형광펜', text: '글자', select: '고르기', eraser: '지우개', pan: '손' };
const RESTORE_TEXT = { same: '이전 판서를 불러왔어요', rescaled: '배율이 바뀌어 크기를 맞췄어요', differentMonitor: '다른 모니터라 불러올 수 없어요', none: '불러올 판서가 없어요' };

/** InkDocument 항목 수(쪽마다 items) — Rust ink_items와 같은 규칙. */
const inkItems = (doc) => {
  const count = (v) => (Array.isArray(v?.items) ? v.items.length : 0);
  return Array.isArray(doc?.pages) ? doc.pages.reduce((n, p) => n + count(p), 0) : count(doc);
};

/**
 * @param {{platform?: 'windows'|'macos', snipasteRunning?: boolean, frameUrl?: string, channel?: 'unpackaged'|'msix-store'|'msix-direct', autostartNotice?: string, permissions?: {screen?: boolean, accessibility?: boolean, screenRevoked?: boolean, accessibilityRevoked?: boolean}, lang?: 'ko'|'en', screenNeedsRelaunch?: boolean}} [opts]
 *   frameUrl = 미리보기에서 오버레이 바탕으로 쓸 정지 이미지(없으면 오버레이가 빈 바탕)
 */
export function createFakeClient({ platform = 'windows', snipasteRunning = true, frameUrl = undefined, channel = 'unpackaged', autostartNotice = undefined, permissions = undefined, lang = 'ko', screenNeedsRelaunch = false } = {}) {
  const combos = Object.fromEntries(Object.entries(defaults.slots).map(([id, v]) => [id, (platform === 'macos' ? v.mac : v.win) || '']));
  const settings = { ...DEFAULT_SETTINGS, 'general.lang': lang };
  /** @type {{ screen: boolean, accessibility: boolean, screenRevoked: boolean, accessibilityRevoked: boolean } | undefined} */
  let perms = permissions
    ? {
        screen: Boolean(permissions.screen),
        accessibility: Boolean(permissions.accessibility),
        screenRevoked: Boolean(permissions.screenRevoked),
        accessibilityRevoked: Boolean(permissions.accessibilityRevoked),
      }
    : platform === 'macos'
      ? { screen: true, accessibility: true, screenRevoked: false, accessibilityRevoked: false }
      : undefined;
  const pendingPerms = { screen: false, accessibility: false };
  /** 캡처 핀 흉내: id → {w, h, zoom} */
  const pins = new Map();
  let pinSeq = 0;
  const history = [];
  const subs = new Set();
  let hotkeysEnabled = true;
  /** 단축키 녹화 중(전역 키를 풀어 둔 흉내) — 시험이 `isShortcutCapturing()`으로 본다 */
  let shortcutCapturing = false;
  let mode = { mode: 'idle' };
  let sessionSeq = 0;
  /** @type {any} 가짜 고르기 상태(세션마다 새로) */
  let capture = null;
  /** @type {string | null} 마지막 단색 HEX 결과(칩 클릭 흉내) */
  let lastHex = null;
  const readyLog = [];
  // 판서 흉내: 지금 세션·보관 한 장·먼저 끝낸 세션(늦은 끝 대기)·섬 값·명령 기록
  /** @type {any} */
  let drawSession = null;
  /** @type {any} */
  let kept = null;
  /** @type {{session: number, board: boolean, quiet: boolean} | null} */
  let ended = null;
  /** @type {{color: string, tool: string, widthPx: number, zoom: number, pageIndex: number, pageCount: number, background: string, idleDim?: boolean, avoid?: boolean, suggestion?: string | null}} */
  let look = { color: 'R', tool: 'pen', widthPx: 4, zoom: 1, pageIndex: 0, pageCount: 1, background: 'frame', idleDim: undefined, avoid: undefined, suggestion: null };
  /** @type {any[]} */
  const drawLog = [];
  let saveSeq = 0;
  /** 실시간 확대 흉내(Alt+2): 배율만 쥐고 1920×1080 가운데를 비춘다 */
  let zoomScale = 0;
  /** 이어받기 흉내(3e-handoff): 결과 섬 [판서로]가 쓸 방금 고른 범위(다음 모드 진입이면 버림) */
  let lastRange = null;
  /** 다음 판서 진입의 시작 보기(확대 → 판서 · [판서로]) */
  let nextView = null;
  /**
   * 캡처 주석(4.9) 흉내: 창(모니터) → 주석 개수, 부탁 순번·기다리는 부탁, 받은 답(시험용).
   * @type {{ session: number, items: Map<number, number>, seq: number, pending: any, answers: any[], bypass: boolean }}
   */
  const annot = { session: 0, items: new Map(), seq: 0, pending: null, answers: [], bypass: false };
  const annotated = () => mode.mode === 'capture' && annot.session === mode.session && [...annot.items.values()].some((n) => n > 0);
  /** 주석 없이(또는 답을 받은 뒤) 그 결과를 지금 경로로 */
  const runPlain = async (self, action) => {
    annot.bypass = true;
    try {
      await self.captureInput({ type: 'command', name: action });
    } finally {
      annot.bypass = false;
    }
  };

  const judge = (slot, combo) => {
    if (!combo) return { status: 'off' };
    const dup = catalog.slots.find((o) => o.id !== slot.id && combos[o.id] === combo && (o.scope === slot.scope || o.scope === 'global' || slot.scope === 'global'));
    if (dup) return { status: 'conflict', reason: `‘${dup.label}’과 겹쳐요`, owner: '슉쇽' };
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
      platform, theme: settings['general.theme'], hotkeysEnabled, pinnedCount: 2, pins: pins.size, recording: { active: false, elapsedMs: 0 },
      settings: { ...settings }, shortcuts: r, version: '0.1.0',
      conflicts: { count: bad.length, app: owners.length === 1 && owners[0] !== '슉쇽' ? owners[0] : undefined, combos: bad.map((x) => x.combo) },
      readonly: false,
      location: {
        path: platform === 'macos' ? '~/Library/Application Support/Shookshok/settings.json' : 'C:\\Users\\…\\AppData\\Roaming\\Shookshok\\settings.json',
        portable: false,
        readOnly: false,
      },
      channel,
      ...(autostartNotice ? { autostartNotice } : {}),
      notices: [],
      mode,
      // 맥 권한(M10·R2-ONB) — 미리보기 ?perm=screen-off 등으로 넘긴다
      ...(perms ? { permissions: { ...perms } } : {}),
    };
  };
  // 가져오기 장면: 다른 PC에서 내보낸 파일(3개가 다름)
  const IMPORT_FILE = { 'draw.width': 8, 'general.theme': 'dark', 'shortcuts.active': 'Ctrl+Alt+F2' };
  const valueOf = (path) => (path.startsWith('shortcuts.') ? combos[path.slice(10)] : settings[path]);
  const emit = (e) => subs.forEach((h) => h(e));
  const island = (island) => emit({ type: 'island', island });
  const changed = () => emit({ type: 'state', state: state() });

  const zoomText = (z) => (Math.round(z * 100) % 10 === 0 ? z.toFixed(1) : z.toFixed(2));
  const drawIslandEvent = () => {
    const base = {
      kind: 'mode',
      mode: drawSession?.board ? 'board' : 'draw',
      ...(typeof look.idleDim === 'boolean' ? { idleDim: look.idleDim } : {}),
      ...(typeof look.avoid === 'boolean' ? { avoid: look.avoid } : {}),
    };
    if (drawSession?.board || look.background !== 'frame') {
      return {
        ...base, variant: 'board', penColor: look.color, text: `칠판 · ${look.background === 'black' ? '검은색' : '흰색'}`, value: `${look.pageIndex + 1} / ${look.pageCount}`,
        hints: [{ key: 'W·K', label: '바탕' }, { key: 'Esc', label: '보관' }, { key: '?', label: '단축키' }],
      };
    }
    if (look.suggestion) {
      return { ...base, variant: 'draw', text: look.suggestion, penColor: look.color, penWidth: `${Math.round(look.widthPx)}px`, button: { label: '바꾸기', action: 'draw.acceptSuggestion' } };
    }
    const restoreKey = combos['d-restore'];
    const hint = drawSession?.hasKept && !drawSession.restoreOnOpen && restoreKey ? `${restoreKey} 이전 판서` : undefined;
    return {
      ...base, variant: 'draw', text: `판서 · ${DRAW_TOOLS[look.tool] ?? '펜'}`, penColor: look.color, penWidth: `${Math.round(look.widthPx)}px`,
      ...(look.zoom > 1 ? { value: `×${zoomText(look.zoom)}` } : {}),
      ...(hint ? { detail: hint } : {}),
      hints: [{ key: 'Esc', label: '보관' }, { key: '?', label: '단축키' }],
    };
  };
  const keptIsland = () => ({ kind: 'result', text: combos.restore ? `판서를 보관했어요 · ${combos.restore}` : '판서를 보관했어요' });
  /** 보관 규칙: keep && 설정 draw.keep && 항목 ≥ 1. 설정이 꺼져 있으면 있던 것도 지운다. */
  const keepInk = (session, keep, payload) => {
    if (!settings['draw.keep']) { kept = null; return false; }
    if (!keep || inkItems(payload?.document) < 1) return false;
    kept = { monitorId: 0, layoutFingerprint: '0,0,1920,1080@100', physW: 1920, physH: 1080, scale: 1, board: Boolean(session?.board), document: payload.document };
    return true;
  };
  /** Rust가 먼저 끝냄(같은 키·다른 모드·잠금) — 늦은 drawFinish 한 번을 받는다. */
  const endDraw = (quiet) => {
    if (mode.mode !== 'draw') return;
    ended = { session: mode.session, board: mode.board, quiet };
    mode = { mode: 'idle' };
    drawSession = null;
    changed();
    island({ kind: 'end', text: '' });
  };
  // ── 실시간 확대 흉내(진짜 계산은 Rust ink-domain::zoom) ──
  // 배율 = 2^(k/8), 1×~8×(k 0~24) — Rust `wheel_target`. 미리보기는 부드러운 전환 없이 목표로 바로 간다
  const NOTCHES_PER_DOUBLE = 8;
  const scaleLabel = (s) => `×${(Math.round(s * 10) / 10).toFixed(1)}`;
  const zoomView = () => ({
    session: mode.session, monitorId: 0, monitorLabel: '모니터 1', scale: zoomScale, scaleLabel: scaleLabel(zoomScale),
    viewport: { x: (1 - 1 / zoomScale) / 2, y: (1 - 1 / zoomScale) / 2, w: 1 / zoomScale, h: 1 / zoomScale },
    cursorCss: { x: 960, y: 540 }, showMinimap: Boolean(settings['zoom.minimap']),
  });
  const zoomIsland = () => island({
    kind: 'mode', mode: 'zoom', variant: 'zoom', text: '실시간 확대', detail: '움직이는 화면 그대로', value: scaleLabel(zoomScale),
    hints: [{ key: '휠', label: '배율' }, { key: 'Alt 1', label: '멈추고 쓰기' }, { key: 'Esc', label: '끝' }],
  });
  const endZoom = () => {
    if (mode.mode !== 'liveZoom') return;
    mode = { mode: 'idle' };
    zoomScale = 0;
    emit({ type: 'zoom', zoom: null });
    island({ kind: 'end', text: '' });
    changed();
  };
  const startZoom = () => {
    endZoom();
    if (mode.mode === 'capture') { mode = { mode: 'idle' }; capture = null; island({ kind: 'end', text: '' }); }
    sessionSeq += 1;
    mode = { mode: 'liveZoom', session: sessionSeq, monitorId: 0, startedMs: Date.now() };
    zoomScale = 2;
    changed();
    zoomIsland();
    emit({ type: 'zoom', zoom: zoomView() });
  };
  const stepZoom = (notches, snap) => {
    const now = Math.log2(zoomScale) * NOTCHES_PER_DOUBLE;
    const n = Math.max(1, Math.round(Math.abs(notches)));
    const k = snap ? (notches < 0 ? Math.floor(now + 1e-6) + n : Math.ceil(now - 1e-6) - n) : now - notches;
    const next = Math.min(8, Math.max(1, 2 ** (k / NOTCHES_PER_DOUBLE)));
    if (Math.abs(next - zoomScale) < 1e-9) return;
    zoomScale = next;
    zoomIsland();
    emit({ type: 'zoom', zoom: zoomView() });
  };
  // ── 이어받기 흉내(진짜 계산은 Rust ink-domain::zoom::to_draw_transform · fit_rect_view) ──
  /** 확대 중 판서(5.3): 가운데를 비추던 확대 그대로 */
  const zoomHandoffView = () => ({ s: zoomScale, tx: -960 * (zoomScale - 1), ty: -540 * (zoomScale - 1) });
  /** [판서로]: 고른 범위가 1920×1080 화면 88%에 꽉 차게(0.01 내림, 1~8), 가운데로 옮긴 뒤 문서 안에 가둠 */
  const fitRectView = (r, vw = 1920, vh = 1080) => {
    const fit = Math.min((0.88 * vw) / r.w, (0.88 * vh) / r.h);
    const s = Math.min(8, Math.max(1, Math.floor(fit * 100 + 1e-9) / 100));
    const clampAxis = (t, view, content) => {
      const free = view - content;
      return free <= 0 ? Math.min(0, Math.max(free, t)) : Math.min(free, Math.max(0, t));
    };
    return { s, tx: clampAxis(vw / 2 - (r.x + r.w / 2) * s, vw, vw * s), ty: clampAxis(vh / 2 - (r.y + r.h / 2) * s, vh, vh * s) };
  };
  const toDrawButton = () => ({ secondaryButton: { label: '판서로', action: 'capture.toDraw' }, ...(combos.draw ? { secondaryButtonKey: combos.draw } : {}) });

  return {
    async getState() { return state(); },
    async updateSetting(path, value) {
      if (!(path in settings)) return { ok: false, error: `모르는 설정: ${path}`, state: state() };
      if (path === 'onboarding.step') {
        const step = Number(value);
        if (!Number.isInteger(step) || step < 0 || step > 4) {
          return { ok: false, error: '0~4만 가능해요', state: state() };
        }
      }
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
      return { cancelled: false, path: 'C:\\Users\\…\\Desktop\\Shookshok-settings-2026-11-25.json', ok: true, changes, warnings: [] };
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
    async shortcutCapture(active) {
      shortcutCapturing = Boolean(active);
    },
    /** 시험용: 단축키 녹화 중인지 */
    isShortcutCapturing() { return shortcutCapturing; },
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
      // 캡처 주석(4.9): 주석이 있으면 결과 전에 주석 층을 부탁한다(진짜는 Rust commit → screen-ink://capture-commit)
      const commitAction =
        input.type === 'command' && ['copy', 'save', 'quickSave', 'pin'].includes(input.name)
          ? input.name
          : (input.type === 'key' && input.key === 'Enter') || input.type === 'dblclick'
            ? 'copy'
            : null;
      if (commitAction && !annot.bypass && annotated()) {
        annot.seq += 1;
        annot.pending = { session: mode.session, seq: annot.seq, action: commitAction, rect: s.rect ?? chainAt(s.p)[s.level].r };
        emit({ type: 'captureCommit', request: { ...annot.pending } });
        return;
      }
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
      if (input.type === 'command' && input.name === 'pin') {
        const r = s.rect ?? chainAt(s.p)[s.level].r;
        await this.run('capture.cancel');
        pinSeq += 1;
        pins.set(pinSeq, { w: r.w, h: r.h, zoom: 100 });
        changed();
        return;
      }
      if (input.type === 'command' && (input.name === 'save' || input.name === 'quickSave')) {
        const r = s.rect ?? chainAt(s.p)[s.level].r;
        const finishedSession = mode.session;
        await this.run('capture.cancel');
        emit({ type: 'afterimage', afterimage: { session: finishedSession, rect: r, durationMs: 400 } });
        lastRange = r;
        island({ kind: 'result', variant: 'result', text: '저장했어요', value: `${r.w}×${r.h}`, detail: '수업_2026-03-04_090507.png', thumbUrl: frameUrl, button: { label: '폴더 열기', action: 'folder.open' }, ...toDrawButton() });
        return;
      }
      if ((input.type === 'key' && input.key === 'Enter') || input.type === 'dblclick' || (input.type === 'command' && input.name === 'copy')) {
        const r = s.rect ?? chainAt(s.p)[s.level].r;
        const finishedSession = mode.session;
        lastHex = '#FFFFFF';
        await this.run('capture.cancel');
        emit({ type: 'afterimage', afterimage: { session: finishedSession, rect: r, durationMs: 400 } });
        lastRange = r;
        island({ kind: 'result', variant: 'result', text: '복사했어요', value: `${r.w}×${r.h}`, detail: '한글·PPT에 Ctrl+V', thumbUrl: frameUrl, button: { label: '저장', action: 'capture.saveLast' }, buttonKey: 'Ctrl+S', chip: { label: `${lastHex} 복사`, action: 'capture.copyHex' }, ...toDrawButton() });
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
    async pinInfo(id) {
      const p = pins.get(id) ?? (id === 0 ? { w: 718, h: 240, zoom: 100 } : null);
      return p ? { ...p, url: frameUrl } : null;
    },
    async pinAction(id, action) {
      if (action === 'close') pins.delete(id);
      else if (action === 'closeAll') pins.clear();
      else if (action === 'copy') island({ kind: 'result', variant: 'result', text: '복사했어요' });
      else if (action === 'save') island({ kind: 'result', variant: 'result', text: '저장했어요', button: { label: '폴더 열기', action: 'folder.open' } });
      changed();
    },
    async pinZoom(id, dy) {
      const p = pins.get(id) ?? { zoom: 100 };
      const levels = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 125, 150, 175, 200, 225, 250, 275, 300, 325, 350, 375, 400];
      const i = levels.indexOf(p.zoom);
      p.zoom = levels[Math.max(0, Math.min(levels.length - 1, i - Math.sign(dy)))];
      return p.zoom;
    },
    async zoomInput(input) {
      if (mode.mode !== 'liveZoom' || !input) return;
      if (input.type === 'wheel') stepZoom(input.dy, true);
      else if (input.type === 'scroll') stepZoom(input.notches, false);
      else if ((input.type === 'key' && input.slot === 'z-exit') || input.type === 'exit') endZoom();
    },
    async overlayReady(session) {
      if (mode.mode === 'capture' && mode.session === session) readyLog.push(session);
    },
    async captureAnnotations(session, monitorId, items) {
      if (mode.mode !== 'capture' || mode.session !== session) return;
      if (annot.session !== session) {
        annot.session = session;
        annot.items = new Map();
        annot.pending = null;
      }
      if (items > 0) annot.items.set(monitorId, items);
      else annot.items.delete(monitorId);
      if (!annotated() && annot.pending) {
        const p = annot.pending;
        annot.pending = null;
        await runPlain(this, p.action);
      }
    },
    async captureCommitAnnotated(meta, rgba) {
      const p = annot.pending;
      if (!p || mode.mode !== 'capture' || meta.session !== p.session || meta.seq !== p.seq) return;
      annot.pending = null;
      annot.answers.push({ meta, bytes: rgba ? rgba.length : 0 });
      await runPlain(this, p.action);
    },
    /** 시험용: 받은 주석 층 답(머리·바이트 수) */
    get annotatedLog() { return annot.answers.map((a) => ({ ...a })); },
    /** 시험용: overlayReady가 받아들인 세션들 */
    get readyLog() { return [...readyLog]; },
    async drawReady(session, paintMs) {
      if (mode.mode === 'draw' && mode.session === session) drawLog.push({ type: 'ready', session, paintMs });
    },
    async drawFinish(payload) {
      const { session, keep = false, kept: data } = payload ?? {};
      if (mode.mode === 'draw' && mode.session === session) {
        const stored = keepInk(drawSession, keep, data);
        mode = { mode: 'idle' };
        drawSession = null;
        changed();
        island({ kind: 'end', text: '' });
        if (stored) island(keptIsland());
        return;
      }
      if (ended && ended.session === session) {
        const e = ended;
        ended = null;
        if (keepInk(e, keep, data) && !e.quiet) island(keptIsland());
      }
    },
    async drawRestore(session) {
      if (mode.mode !== 'draw' || mode.session !== session) return { status: 'none' };
      if (!settings['draw.keep']) kept = null;
      const status = kept ? 'same' : 'none';
      drawSession = { ...drawSession, restoreOnOpen: false };
      // 불러옴·배율·다른 모니터 문구는 엔진(restoreKept)이 drawIsland 안내로 낸다 — Rust처럼 "없음"만
      if (status === 'none') island({ kind: 'info', text: RESTORE_TEXT.none });
      return status === 'same' ? { status, kept: { ...kept } } : { status };
    },
    async drawExport(action, bytes, meta) {
      if (mode.mode !== 'draw' || (meta?.session != null && meta.session !== mode.session)) return { ok: false, error: 'not drawing' };
      if (!meta || bytes?.byteLength !== meta.w * meta.h * 4) return { ok: false, error: 'size mismatch' };
      // 확대 중이어도 늘 모니터 전체(물리 해상도 그대로) — 사용자 결정 2026-11-26
      const b = drawSession?.monitor?.bounds;
      if (!b || meta.w !== b.w || meta.h !== b.h) return { ok: false, error: 'not monitor size' };
      drawLog.push({ type: 'export', action, w: meta.w, h: meta.h, bytes: bytes.byteLength });
      if (action === 'copy') {
        island({ kind: 'result', variant: 'result', text: '복사했어요', value: `${meta.w}×${meta.h}`, detail: '한글·PPT에 Ctrl+V' });
        return { ok: true };
      }
      saveSeq += 1;
      const name = `수업_20260304_${saveSeq}.png`;
      island({ kind: 'result', variant: 'result', text: '저장했어요', detail: name, button: { label: '폴더 열기', action: 'folder.open' } });
      return { ok: true, name };
    },
    async drawIsland(patch) {
      const p = patch ?? {};
      if (mode.mode !== 'draw' || (p.session != null && p.session !== mode.session)) return;
      for (const k of ['color', 'tool', 'widthPx', 'zoom', 'pageIndex', 'pageCount', 'background', 'idleDim', 'avoid']) if (p[k] != null) look[k] = p[k];
      look.suggestion = p.suggestion || null;
      island(drawIslandEvent());
      if (p.notice) island({ kind: 'info', text: p.notice, ...(p.noticeAction === 'undo' ? { button: { label: '되돌리기', action: 'draw.undo' } } : {}) });
    },
    async drawPenMetrics(sample) {
      if (sample?.n > 0) drawLog.push({ type: 'pen', line: `pen latency n=${sample.n} p50=${sample.p50.toFixed(1)} p95=${sample.p95.toFixed(1)} max=${sample.max.toFixed(1)} ms` });
    },
    /** 시험용: 판서 명령 기록(ready·export·pen) */
    get drawLog() { return [...drawLog]; },
    /** 시험용: 지금 보관 판서 */
    get keptInk() { return kept; },
    /** 시험용: 잠금 같은 시스템 사건 흉내 — Rust처럼 판서를 먼저 끝낸다(늦은 끝으로 보관) */
    simulateLock() { endDraw(true); },
    async run(action) {
      if (action === 'settings.reveal' || action === 'firstRun.open') return;
      if (action === 'perm.screen') {
        if (perms && !screenNeedsRelaunch) pendingPerms.screen = true;
        return;
      }
      if (action === 'perm.accessibility') {
        if (perms) pendingPerms.accessibility = true;
        return;
      }
      if (action === 'perm.refresh') {
        if (!perms) return;
        let dirty = false;
        if (pendingPerms.screen && !perms.screen) {
          perms.screen = true;
          perms.screenRevoked = false;
          pendingPerms.screen = false;
          dirty = true;
        }
        if (pendingPerms.accessibility && !perms.accessibility) {
          perms.accessibility = true;
          perms.accessibilityRevoked = false;
          pendingPerms.accessibility = false;
          dirty = true;
        }
        if (dirty) changed();
        return;
      }
      if (action === 'app.relaunch') {
        if (perms && (!perms.screen || pendingPerms.screen)) {
          perms.screen = true;
          perms.screenRevoked = false;
          pendingPerms.screen = false;
          changed();
        }
        return;
      }
      if (action === 'pins.closeAll') { pins.clear(); changed(); return; }
      if (action === 'draw.acceptSuggestion' || action === 'draw.undo') {
        if (mode.mode === 'draw') emit({ type: 'drawCommand', session: mode.session, command: action.slice('draw.'.length) });
        return;
      }
      if (action === 'zoom') {
        if (mode.mode === 'draw') endDraw(true);
        lastRange = null;
        startZoom();
        return;
      }
      if (action === 'capture.toDraw') {
        if (!lastRange) { island({ kind: 'info', text: '판서로 넘길 캡처가 없어요' }); return; }
        nextView = fitRectView(lastRange);
        return this.run('draw');
      }
      if (DRAW_ACTION[action]) {
        const kind = DRAW_ACTION[action];
        // 판서 중 직전 판서 열기 = 같은 세션을 복원 표시로 한 번 더
        if (kind === 'restore' && mode.mode === 'draw' && !mode.board) {
          drawSession = { ...drawSession, restoreOnOpen: true };
          emit({ type: 'draw', draw: drawSession });
          return;
        }
        // 이어받기: [판서로]가 정한 보기, 확대 중 판서는 그 확대 그대로(칠판·직전 판서는 1×)
        const view = kind === 'draw' ? nextView ?? (mode.mode === 'liveZoom' ? zoomHandoffView() : null) : null;
        nextView = null;
        lastRange = null;
        if (mode.mode === 'capture') await this.run('capture.cancel');
        if (mode.mode === 'draw') endDraw(true);
        endZoom();
        if (!settings['draw.keep']) kept = null;
        const board = kind === 'board' || (kind === 'restore' && Boolean(kept?.board));
        sessionSeq += 1;
        const bounds = { x: 0, y: 0, w: 1920, h: 1080 };
        drawSession = {
          session: sessionSeq, board,
          monitor: { id: 0, bounds, work: { ...bounds, h: 1040 }, scale: 1, primary: true, ...(board || !frameUrl ? {} : { frameUrl }) },
          layoutFingerprint: '0,0,1920,1080@100', initialView: view ?? { s: 1, tx: 0, ty: 0 },
          restoreOnOpen: kind === 'restore', hasKept: Boolean(kept), keptCompatible: kept ? 'same' : 'none', cursor: { x: 760, y: 420 },
        };
        look = { color: settings['draw.color'], tool: 'pen', widthPx: settings['draw.width'], zoom: view?.s ?? 1, pageIndex: 0, pageCount: 1, background: board ? 'white' : 'frame' };
        mode = { mode: 'draw', session: sessionSeq, board, monitorId: 0, startedMs: Date.now() };
        // 세션 먼저, 상태 나중(Rust와 같은 순서)
        emit({ type: 'draw', draw: drawSession });
        changed();
        island(drawIslandEvent());
        return;
      }
      if (CAPTURE_ACTION[action]) {
        if (mode.mode === 'draw') endDraw(true);
        endZoom();
        lastRange = null;
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
      if (action === 'capture.saveLast') {
        island({ kind: 'result', variant: 'result', text: '저장했어요', detail: '수업_2026-03-04_090507.png', button: { label: '폴더 열기', action: 'folder.open' } });
        return;
      }
      if (action === 'capture.copyHex') {
        const hex = lastHex ?? '#FF9F0A';
        island({ kind: 'result', variant: 'result', text: `${hex} 복사했어요` });
        return;
      }
      island({ kind: 'info', text: `미리보기: ${action}` });
    },
    /** 시험용: 권한 상태를 직접 바꾼다 */
    setPermissions(patch) {
      if (!perms) return state();
      Object.assign(perms, patch);
      changed();
      return state();
    },
    subscribe(handler) { subs.add(handler); return () => subs.delete(handler); },
  };
}
