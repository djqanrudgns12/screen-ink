//! InkService — 설정 바로 반영(R-SET-0), 단축키 즉시 등록 트랜잭션(R-KEY-3, 02 §6), 창 안 되돌리기.
//!
//! 단축키 바꾸기 = "새 키 등록 → 성공하면 옛 키 해제 → 저장". 어느 단계든 실패하면 옛 키 그대로.
//! 카탈로그로 추정한 다른 앱 겹침(Snipaste 등)은 실제로 등록해 보고 확인하고,
//! 규칙·이 표·설정 파일에서 읽은 키(PowerToys 저수준 훅)는 등록이 성공해도 막는다.

use crate::ports::{HotkeyError, Ports, SystemEvent};
use crate::state::*;
use ink_domain::geometry::MonitorLayout;
use ink_domain::i18n::{tr, trf, Lang};
use ink_domain::mode::{self, Mode, ModeCommand};
use ink_domain::settings::{self, ExcludedApp, LoadError, Settings};
use ink_domain::shortcuts::{canonicalize, catalog, spoken_for, to_accelerator, Analysis, Environment, Level, Platform, SlotDef};
use ink_domain::snapshot::{filter_windows, CaptureKind, Snapshot};
use ink_domain::timing::{self, EntryTiming, Stats};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Instant;

/// 진입 시간 표본을 몇 개까지 기억할지(M1 통계, n ≥ 100).
const TIMING_KEEP: usize = 500;

/// 단축키를 잠시 꺼도 남겨 두는 칸(`` Alt+` ``).
pub const PAUSE_SLOT: &str = "pause";
const PROBE_SLOT: &str = "__probe";

/// 실행 중이면 "그 앱을 계속 쓴다"로 보는 대체 앱(02 §6.2-2). 값 = 카탈로그 keepKey.
const KNOWN_APPS: [(&str, &str); 5] = [
    ("snipaste.exe", "snipaste"),
    ("icanzoomit.exe", "icanzoomit"),
    ("zoomit.exe", "zoomit"),
    ("zoomit64.exe", "zoomit"),
    ("powertoys.exe", "powertoys"),
];

#[derive(Clone, Debug)]
pub(crate) enum FailureKind {
    Analysis,
    Taken,
    Unsupported,
    Message(String),
}

#[derive(Clone, Debug)]
pub(crate) struct Failure {
    kind: FailureKind,
    reason: String,
    owner: Option<String>,
}

impl Failure {
    fn reason_for(&self, lang: Lang, analysis: Option<&Analysis>) -> String {
        match &self.kind {
            FailureKind::Analysis => analysis.and_then(|a| a.reason_for(lang)).unwrap_or_else(|| self.reason.clone()),
            FailureKind::Taken => self
                .owner
                .as_ref()
                .map(|o| trf(lang, "shortcut.reason.owner", &[("owner", o)]))
                .unwrap_or_else(|| tr(lang, "error.hotkey.taken")),
            FailureKind::Unsupported => tr(lang, "error.hotkey.unsupported"),
            FailureKind::Message(s) => s.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum StartupNotice {
    KeyOrText(String),
    NewerSchema(u32),
    LoadFailedReset(LoadError),
    ReadOnlyDir,
    Text(String),
}

impl StartupNotice {
    fn render(&self, lang: Lang) -> String {
        match self {
            Self::KeyOrText(s) => tr(lang, s),
            Self::NewerSchema(v) => LoadError::NewerSchema(*v).message(lang),
            Self::LoadFailedReset(e) => {
                let err = e.message(lang);
                trf(lang, "error.settings.loadFailedReset", &[("err", &err)])
            }
            Self::ReadOnlyDir => tr(lang, "error.settings.readOnlyDir"),
            Self::Text(s) => s.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Undo {
    Setting { path: String, prev: Value },
    Shortcut { id: String, prev: String },
    Batch(Vec<Undo>),
    /// 가져오기처럼 한꺼번에 바뀐 것 — 통째로 되돌린다
    Whole(Box<Settings>),
}

/// 가져오기 미리 보기(05 §1 C: "12개 항목이 바뀌어요" 시트).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub changes: Vec<settings::Change>,
    /// 읽을 수 없어 처음 값으로 둔 항목
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HotkeyOutcome {
    /// 아직 서비스가 처리하지 않는 동작 — 칸 ID(어댑터가 처리하거나 "준비 중")
    Start(String),
    /// 캡처 모드를 시작·전환·끝냄
    Captured,
    /// 실시간 확대를 시작·끝냄·배율 바꿈(live_zoom.rs)
    Zoom,
    /// 단축키 잠시 끄기·켜기를 처리함
    Toggled(bool),
    Ignored,
    /// 판서·칠판을 시작·전환·끝냄(판서 중 직전 판서 키 = 그 자리 복원)
    Draw,
}

fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

pub struct InkService {
    pub(crate) ports: Ports,
    pub(crate) platform: Platform,
    pub(crate) version: String,
    pub(crate) settings: Settings,
    pub(crate) env: Environment,
    pub(crate) hotkeys_enabled: bool,
    /// 설정에서 단축키를 녹화하는 중(R2-KEY-REC) — 전역 키를 모두 조용히 풀어 둔다
    pub(crate) capturing_keys: bool,
    /// 칸 → 등록한 가속기
    pub(crate) registered: BTreeMap<String, String>,
    /// 등록하지 못한 전역 칸과 이유
    pub(crate) failures: BTreeMap<String, Failure>,
    /// 실패한 줄마다 보여 준 첫 추천 — 다른 줄과 겹치지 않게
    pub(crate) offered: BTreeMap<String, String>,
    pub(crate) history: Vec<Undo>,
    pub(crate) readonly: bool,
    pub(crate) notices: Vec<StartupNotice>,
    pub(crate) pinned_count: u32,
    // ── 모드·캡처(단계 2) ──
    pub(crate) epoch: Instant,
    pub(crate) mode: Mode,
    pub(crate) session_seq: u64,
    /// 지난번 오버레이를 맞춘 배치 — 지문이 다르면 다시 맞춘다
    pub(crate) prepared: Option<MonitorLayout>,
    pub(crate) snapshot: Option<Snapshot>,
    /// (단축키 시각, 진입 계측) — UI가 그렸다고 알리면 마감
    pub(crate) pending: Option<(Instant, EntryTiming)>,
    pub(crate) ready_samples: VecDeque<f64>,
    pub(crate) secure_notified: bool,
    // ── 고르기·결과(단계 3) ──
    pub(crate) cap: crate::capture::CaptureState,
    // ── 판서·칠판(단계 4) ──
    pub(crate) draw: crate::draw::DrawState,
    // ── 실시간 확대(단계 5) ──
    pub(crate) zoom: crate::live_zoom::ZoomState,
    // ── 패키지 모드·자동 실행·업데이터(D3) ──
    pub(crate) pkg: crate::package::PackageState,
    // ── 창 캡처·바로 캡처·녹화(단계 6) ──
    pub(crate) rec: crate::record::RecordState,
    // ── 맥 권한(M10) ──
    pub(crate) permissions: Option<crate::state::Permissions>,
    /// 손쉬운 사용 칩을 이번 실행에 보였음(캡처 모드 섬에 한 번만)
    pub(crate) ax_chip_shown: bool,
}

/// 권한 단추 동작(M10) — 플러그인이 받아 요청 + 설정 칸 열기를 한다.
pub const ACTION_PERM_SCREEN: &str = "perm.screen";
pub const ACTION_PERM_AX: &str = "perm.accessibility";
/// 권한을 다시 읽는다(R2-ONB-2 — 첫 실행 카드·설정 점검이 열려 있는 동안 1초마다). 바뀌었을 때만 상태를 보낸다.
pub const ACTION_PERM_REFRESH: &str = "perm.refresh";
/// 권한 안내 섬은 설정 앱을 다녀올 동안 남겨 둔다.
pub const PERMISSION_ISLAND_MS: u32 = 15_000;

fn peek_lang(file: &Value) -> Lang {
    let s = file
        .pointer("/sections/settings/data/values/general.lang")
        .or_else(|| file.pointer("/sections/settings/data/general.lang"))
        .or_else(|| file.pointer("/sections/settings/data/general/lang"))
        .and_then(Value::as_str)
        .unwrap_or("");
    Lang::from_setting(s)
}

fn failure_of(a: &Analysis, lang: Lang) -> Failure {
    let top = a.hard_block().or_else(|| a.top());
    Failure {
        kind: FailureKind::Analysis,
        reason: a.reason_for(lang).or_else(|| top.map(|t| t.what.clone())).unwrap_or_else(|| tr(lang, "error.hotkey.unusable")),
        owner: top.and_then(|t| t.owner.clone()),
    }
}

impl InkService {
    /// 모드 섬 둘째 줄(처음 3번과 `?`일 때만 보임 — 섬 호스트가 정함).
    pub(crate) fn capture_island(&self, kind: CaptureKind) -> IslandEvent {
        let (text, variant, hints): (String, &str, Vec<(String, String)>) = match kind {
            CaptureKind::Region => (
                self.t("island.capture.regionTitle"),
                "region",
                vec![
                    (self.t("island.hint.key.wheel"), self.t("island.hint.resize")),
                    ("Tab".into(), self.t("island.hint.window")),
                    ("Enter".into(), self.t("island.hint.copy")),
                    ("Esc".into(), self.t("island.hint.cancel")),
                ],
            ),
            CaptureKind::Window => {
                let win_key = self.key_text("window").unwrap_or_else(|| "F2".into());
                (
                    self.t("island.capture.windowTitle"),
                    "window",
                    vec![
                        (self.t("island.hint.key.click"), self.t("island.hint.shoot")),
                        (win_key, self.t("island.hint.currentWindow")),
                        ("Esc".into(), self.t("island.hint.cancel")),
                    ],
                )
            }
            CaptureKind::Record => (
                self.t("island.capture.recordTitle"),
                "region",
                vec![
                    (self.t("island.hint.key.drag"), self.t("island.hint.area")),
                    (self.t("island.hint.key.click"), self.t("island.hint.monitor")),
                    ("Esc".into(), self.t("island.hint.cancel")),
                ],
            ),
        };
        let pairs: Vec<(&str, &str)> = hints.iter().map(|(k, l)| (k.as_str(), l.as_str())).collect();
        let mut ev = IslandEvent::new(IslandKind::Mode, text).variant(variant).hints(&pairs);
        ev.mode = Some(kind.slot().into());
        ev
    }

    pub fn new(mut ports: Ports, platform: Platform, version: impl Into<String>) -> Self {
        let loaded = ports.store.load();
        // 범위 기록(05 §4) — 설정과 같은 파일의 history 구역
        let ranges = loaded
            .file
            .as_ref()
            .and_then(|f| f.pointer("/sections/history/data"))
            .and_then(|v| serde_json::from_value::<ink_domain::history::History>(v.clone()).ok())
            .unwrap_or_default();
        let mut notices: Vec<StartupNotice> = loaded.notice.into_iter().map(StartupNotice::KeyOrText).collect();
        let mut readonly = ports.store.location().read_only;
        let mut first_run = false;
        let settings = match loaded.file {
            None => {
                first_run = true;
                Settings::defaults(platform)
            }
            Some(file) => {
                let init_lang = peek_lang(&file);
                match settings::migrate(&file) {
                    Ok(data) => {
                        let (s, warnings) = Settings::from_data_for(&data, platform, init_lang);
                        notices.extend(warnings.into_iter().map(StartupNotice::Text));
                        s
                    }
                    Err(LoadError::NewerSchema(v)) => {
                        readonly = true;
                        notices.push(StartupNotice::NewerSchema(v));
                        let data = file.pointer("/sections/settings/data").cloned().unwrap_or(Value::Null);
                        Settings::from_data_for(&data, platform, init_lang).0
                    }
                    Err(e) => {
                        notices.push(StartupNotice::LoadFailedReset(e));
                        Settings::defaults(platform)
                    }
                }
            }
        };
        if readonly && ports.store.location().read_only {
            notices.push(StartupNotice::ReadOnlyDir);
        }
        let mut svc = InkService {
            ports,
            platform,
            version: version.into(),
            settings,
            env: catalog().default_environment(),
            hotkeys_enabled: true,
            capturing_keys: false,
            registered: BTreeMap::new(),
            failures: BTreeMap::new(),
            offered: BTreeMap::new(),
            history: Vec::new(),
            readonly,
            notices,
            pinned_count: 0,
            epoch: Instant::now(),
            mode: Mode::Idle,
            session_seq: 0,
            prepared: None,
            snapshot: None,
            pending: None,
            ready_samples: VecDeque::new(),
            secure_notified: false,
            cap: crate::capture::CaptureState::with_ranges(ranges),
            draw: Default::default(),
            zoom: Default::default(),
            pkg: Default::default(),
            rec: Default::default(),
            permissions: None,
            ax_chip_shown: false,
        };
        svc.refresh_environment();
        svc.prepare_overlays();
        if first_run && !svc.readonly {
            if let Err(e) = svc.save() {
                log::warn!("first save failed: {e:?}");
            }
        }
        svc
    }

    // ───────────── 읽기 ─────────────

    pub fn platform(&self) -> Platform {
        self.platform
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn is_first_run(&self) -> bool {
        !self.settings.bool("onboarding.done")
    }

    pub fn state(&self) -> InkState {
        let shortcuts = self.rows();
        let bad: Vec<&ShortcutRow> = shortcuts.iter().filter(|r| r.status == RowStatus::Conflict && r.scope == "global").collect();
        let owners: BTreeSet<Option<&String>> = bad.iter().map(|r| r.owner.as_ref()).collect();
        let app = match owners.into_iter().collect::<Vec<_>>().as_slice() {
            [Some(o)] => Some((*o).clone()),
            _ => None,
        };
        let channel = self.pkg.runtime.channel();
        let mut location = self.ports.store.location();
        location.portable = ink_domain::package::resolve_storage_policy(channel, location.portable).use_portable_dir;
        InkState {
            platform: match self.platform {
                Platform::Win => "windows",
                Platform::Mac => "macos",
            },
            theme: self.settings.str("general.theme").to_string(),
            hotkeys_enabled: self.hotkeys_enabled,
            pinned_count: self.pinned_count,
            pins: self.cap.pins.len() as u32,
            recording: self.recording_view(),
            settings: self.settings.values.clone(),
            conflicts: ConflictSummary { count: bad.len(), app, combos: bad.iter().map(|r| r.combo.clone()).collect() },
            shortcuts,
            version: self.version.clone(),
            readonly: self.readonly,
            location,
            channel,
            autostart_notice: self.pkg.autostart_notice.as_ref().and_then(|n| n.guidance(self.lang())),
            notices: self.notices.iter().map(|n| n.render(self.lang())).collect(),
            mode: self.mode,
            permissions: self.permissions,
        }
    }

    fn assigned_except(&self, id: &str) -> BTreeMap<String, String> {
        self.settings.shortcuts.iter().filter(|(k, _)| k.as_str() != id).map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    fn slot(&self, id: &str) -> Option<&'static SlotDef> {
        catalog().slot(id).filter(|s| !(self.platform == Platform::Mac && s.windows_only))
    }

    fn analyze(&self, slot: &SlotDef, combo: &str) -> Analysis {
        catalog().analyze_for(slot, self.platform, combo, &self.assigned_except(&slot.id), &self.env, self.lang())
    }

    fn status_of(a: &Analysis) -> RowStatus {
        if a.empty {
            return RowStatus::Off;
        }
        match a.level {
            Level::Block => RowStatus::Conflict,
            Level::Warn => RowStatus::Warn,
            Level::Ok | Level::Repl => RowStatus::Ok,
        }
    }

    fn rows(&self) -> Vec<ShortcutRow> {
        let lang = self.lang();
        catalog()
            .slots_for(self.platform)
            .map(|s| {
                let combo = self.settings.shortcut(&s.id).to_string();
                let mut a = self.analyze(s, &combo);
                if self.registered.contains_key(&s.id) {
                    a = a.confirmed_free();
                }
                let (status, reason, owner) = match self.failures.get(&s.id) {
                    Some(f) => (RowStatus::Conflict, Some(f.reason_for(lang, Some(&a))), f.owner.clone()),
                    None => {
                        let status = Self::status_of(&a);
                        let reason = if a.empty { Some(self.t("shortcut.emptyHint")) } else { a.reason_for(lang) };
                        (status, reason, a.top().and_then(|t| t.owner.clone()))
                    }
                };
                ShortcutRow {
                    id: s.id.clone(),
                    scope: s.scope.clone(),
                    group: s.group.clone(),
                    label: s.label.clone(),
                    combo,
                    status,
                    reason,
                    owner,
                    later: s.later,
                    windows_only: s.windows_only,
                }
            })
            .collect()
    }

    // ───────────── 이 PC 사정 ─────────────

    /// 실행 중인 대체 앱과 설정 파일의 키를 다시 읽고, 실패했던 줄을 다시 등록해 본다
    /// (Snipaste를 끄고 설정 창을 열면 F1이 저절로 살아난다).
    /// 맥 권한을 다시 확인한다(M10). 있으면 "전에 있었음"을 기록하고, 전에 있었는데 지금 없으면 풀림으로 표시.
    pub fn refresh_permissions(&mut self) -> Option<Permissions> {
        let (screen, accessibility) = self.ports.probe.permissions()?;
        let had_screen = self.settings.bool("mac.hadScreenPerm");
        let had_ax = self.settings.bool("mac.hadAxPerm");
        for (key, now, had) in [("mac.hadScreenPerm", screen, had_screen), ("mac.hadAxPerm", accessibility, had_ax)] {
            if now && !had && self.settings.set(key, &Value::Bool(true)).is_ok() && !self.readonly {
                if let Err(e) = self.save() {
                    log::warn!("permission flag save failed: {e:?}");
                }
            }
        }
        let p = Permissions { screen, accessibility, screen_revoked: had_screen && !screen, accessibility_revoked: had_ax && !accessibility };
        self.permissions = Some(p);
        Some(p)
    }

    /// 권한을 다시 읽고 **바뀌었으면** 상태를 보낸다(R2-ONB-2). 켜지는 순간 카드에 체크가 뜨게 하는 감시용. 바뀌었으면 true.
    /// 권한 개념이 없는 OS(Windows·시험 기본)는 아무 일도 없다.
    pub fn poll_permissions(&mut self) -> bool {
        let before = self.permissions;
        let now = self.refresh_permissions();
        let changed = now.is_some() && now != before;
        if changed {
            self.emit_state();
        }
        changed
    }

    /// 시작할 때 한 번: 업데이트·재서명 뒤 화면 기록 권한이 풀렸으면 바로 알린다(M10).
    pub fn startup_permission_check(&mut self) {
        if self.refresh_permissions().is_some_and(|p| p.screen_revoked) {
            let ev = IslandEvent::new(IslandKind::Problem, self.t("island.perm.screenRevoked"))
                .detail(self.t("island.perm.screenRevokedDetail"))
                .button(&self.t("island.perm.openScreenBtn"), ACTION_PERM_SCREEN, None)
                .duration_ms(PERMISSION_ISLAND_MS);
            self.island(ev);
        }
    }

    /// 화면 기록 권한 없음 섬(M10): 단추 하나로 요청 + 설정 칸 열기(맥만 생긴다 — Windows는 권한 개념 없음).
    pub(crate) fn no_screen_permission_island(&self) -> IslandEvent {
        IslandEvent::new(IslandKind::Problem, self.t("island.perm.screenNeeded"))
            .detail(self.t("island.perm.screenDetail"))
            .button(&self.t("island.perm.openScreenBtn"), ACTION_PERM_SCREEN, None)
            .duration_ms(PERMISSION_ISLAND_MS)
    }

    pub fn refresh_environment(&mut self) {
        self.refresh_permissions();
        let running: BTreeSet<String> = self.ports.probe.running_apps().into_iter().map(|s| s.to_lowercase()).collect();
        let mut env = catalog().default_environment();
        for v in env.keep.values_mut() {
            *v = false;
        }
        for (exe, key) in KNOWN_APPS {
            if running.contains(exe) {
                env.keep.insert(key.to_string(), true);
            }
        }
        match self.ports.probe.live_bindings() {
            // 설정 파일에서 정확한 키를 읽었으면 카탈로그 추정(PowerToys 기본 키)은 버린다
            Some(live) => {
                env.keep.insert("powertoys".into(), false);
                env.keep.insert("powertoysAot".into(), false);
                env.live = live;
            }
            None => {
                let pt = running.contains("powertoys.exe");
                env.keep.insert("powertoys".into(), pt);
                env.keep.insert("powertoysAot".into(), pt);
            }
        }
        self.env = env;
        if self.hotkeys_enabled && !self.capturing_keys {
            let failed: Vec<String> = self.failures.keys().cloned().collect();
            for id in failed {
                let combo = self.settings.shortcut(&id).to_string();
                let _ = self.register_slot(&id, &combo);
            }
        }
    }

    pub fn environment(&self) -> &Environment {
        &self.env
    }

    // ───────────── 단축키 등록 ─────────────

    /// 시작할 때 전역 칸 전부 등록. 실패한 줄은 이유와 함께 남고 앱은 계속 돈다(05 §2 C).
    pub fn start(&mut self) -> usize {
        let ids: Vec<String> = catalog().slots_for(self.platform).filter(|s| s.is_global()).map(|s| s.id.clone()).collect();
        for id in ids {
            if !self.hotkeys_enabled && id != PAUSE_SLOT {
                continue;
            }
            let combo = self.settings.shortcut(&id).to_string();
            let _ = self.register_slot(&id, &combo);
        }
        self.failures.len()
    }

    /// 칸 하나를 지금 값으로 등록(이미 등록돼 있으면 해제 후). 설정은 건드리지 않는다.
    fn register_slot(&mut self, id: &str, combo: &str) -> Result<(), Failure> {
        if let Some(old) = self.registered.remove(id) {
            self.ports.hotkeys.unregister(&old);
        }
        self.failures.remove(id);
        if combo.is_empty() {
            return Ok(());
        }
        let Some(slot) = self.slot(id) else { return Ok(()) };
        let r = self.try_register(slot, combo);
        match &r {
            Ok(accel) => {
                self.registered.insert(id.to_string(), accel.clone());
            }
            Err(f) => {
                self.failures.insert(id.to_string(), f.clone());
            }
        }
        r.map(|_| ())
    }

    /// 판정 → 가속기 → 운영체제 등록. 성공하면 가속기.
    fn try_register(&mut self, slot: &SlotDef, combo: &str) -> Result<String, Failure> {
        let lang = self.lang();
        let a = self.analyze(slot, combo);
        if a.hard_block().is_some() {
            return Err(failure_of(&a, lang));
        }
        let accel = to_accelerator(combo, self.platform).ok_or_else(|| Failure {
            kind: FailureKind::Unsupported,
            reason: self.t("error.hotkey.unsupported"),
            owner: None,
        })?;
        match self.ports.hotkeys.register(&slot.id, &accel) {
            Ok(()) => Ok(accel),
            Err(HotkeyError::Taken) => {
                // 카탈로그 추정이 있으면 그 앱 이름, 없으면 실행 중인 대체 앱 중 하나도 확신할 수 없으니 "다른 앱"
                let owner = a.items.iter().find(|i| i.verifiable).and_then(|i| i.owner.clone());
                let reason = owner
                    .as_ref()
                    .map(|o| self.tf("shortcut.reason.owner", &[("owner", o)]))
                    .unwrap_or_else(|| self.t("error.hotkey.taken"));
                Err(Failure { kind: FailureKind::Taken, reason, owner })
            }
            Err(e) => Err(Failure { kind: FailureKind::Message(e.message(lang)), reason: e.message(lang), owner: None }),
        }
    }

    /// 추천 3개 — 판정 통과 + (전역이면) 지금 실제로 등록 가능 + 다른 실패 줄의 첫 추천과 다르게.
    fn suggest(&mut self, slot: &SlotDef) -> Vec<String> {
        let avoid: BTreeSet<String> = self.offered.iter().filter(|(k, _)| k.as_str() != slot.id).map(|(_, v)| v.clone()).collect();
        let candidates = catalog().suggestions(slot, self.platform, &self.assigned_except(&slot.id), &self.env, &avoid, 8);
        let mut out = Vec::new();
        for c in candidates {
            if slot.is_global() && self.hotkeys_enabled {
                let Some(accel) = to_accelerator(&c.combo, self.platform) else { continue };
                if self.ports.hotkeys.register(PROBE_SLOT, &accel).is_err() {
                    continue;
                }
                self.ports.hotkeys.unregister(&accel);
            }
            out.push(c.combo);
            if out.len() == 3 {
                break;
            }
        }
        if let Some(first) = out.first() {
            self.offered.insert(slot.id.clone(), first.clone());
        }
        out
    }

    /// 새 키 등록 → 옛 키 해제 → 설정 반영. 실패하면 아무것도 바꾸지 않는다.
    fn apply_shortcut(&mut self, slot: &SlotDef, combo: &str) -> Result<RowStatus, Failure> {
        let a = self.analyze(slot, combo);
        if a.hard_block().is_some() {
            return Err(failure_of(&a, self.lang()));
        }
        let live = slot.is_global() && (self.hotkeys_enabled || slot.id == PAUSE_SLOT);
        let mut status = Self::status_of(&a);
        if live && !combo.is_empty() && self.capturing_keys {
            // 녹화 중: 다른 앱이 쓰는지 운영체제에 한 번 물어보고(등록 → 바로 해제) 진짜 등록은 녹화가 끝날 때
            let accel = self.try_register(slot, combo)?;
            self.ports.hotkeys.unregister(&accel);
            self.registered.remove(&slot.id);
            status = Self::status_of(&a.confirmed_free());
        } else if live && !combo.is_empty() {
            let accel = self.try_register(slot, combo)?;
            if let Some(old) = self.registered.insert(slot.id.clone(), accel.clone()) {
                if old != accel {
                    self.ports.hotkeys.unregister(&old);
                }
            }
            status = Self::status_of(&a.confirmed_free());
        } else if let Some(old) = self.registered.remove(&slot.id) {
            self.ports.hotkeys.unregister(&old);
        }
        self.failures.remove(&slot.id);
        self.offered.remove(&slot.id);
        let lang = self.lang();
        self.settings
            .set_shortcut(&slot.id, combo, self.platform)
            .map_err(|e| Failure { kind: FailureKind::Message(e.message(lang)), reason: e.message(lang), owner: None })?;
        Ok(status)
    }

    fn fail_result(&mut self, slot: Option<&SlotDef>, f: Failure) -> SetShortcutResult {
        let suggestions = slot.map(|s| self.suggest(s)).unwrap_or_default();
        SetShortcutResult { ok: false, status: RowStatus::Conflict, reason: Some(f.reason), owner: f.owner, suggestions }
    }

    /// 키를 누르는 순간(S02). 빈 값 = 비우기.
    pub fn set_shortcut(&mut self, id: &str, combo: &str) -> SetShortcutResult {
        let lang = self.lang();
        let Some(slot) = self.slot(id) else {
            return self.fail_result(None, Failure { kind: FailureKind::Message(self.t("error.hotkey.unknownSlot")), reason: self.t("error.hotkey.unknownSlot"), owner: None });
        };
        let Some(combo) = canonicalize(combo, self.platform) else {
            return self.fail_result(Some(slot), Failure { kind: FailureKind::Message(self.t("error.hotkey.badCombo")), reason: self.t("error.hotkey.badCombo"), owner: None });
        };
        let prev = self.settings.shortcut(id).to_string();
        if combo == prev && !self.failures.contains_key(id) {
            let a = self.analyze(slot, &combo);
            return SetShortcutResult { ok: true, status: Self::status_of(&a), reason: a.reason_for(lang), owner: None, suggestions: vec![] };
        }
        match self.apply_shortcut(slot, &combo) {
            Ok(status) => {
                if let Err(f) = self.save() {
                    let _ = self.apply_shortcut(slot, &prev);
                    self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.saveError")));
                    return self.fail_result(None, f);
                }
                self.history.push(Undo::Shortcut { id: id.to_string(), prev });
                let ev = if combo.is_empty() {
                    IslandEvent::new(IslandKind::Result, self.t("island.settings.shortcutCleared")).detail(self.t("shortcut.emptyHint")).undo_button_for(lang)
                } else {
                    let key_name = spoken_for(&combo, self.platform, lang);
                    IslandEvent::new(IslandKind::Result, self.tf("island.settings.shortcutChanged", &[("combo", &key_name)])).undo_button_for(lang)
                };
                self.island(ev);
                self.emit_state();
                let reason = if status == RowStatus::Warn { self.analyze(slot, &combo).reason_for(lang) } else { None };
                SetShortcutResult { ok: true, status, reason, owner: None, suggestions: vec![] }
            }
            Err(f) => {
                let r = self.fail_result(Some(slot), f);
                self.emit_state();
                r
            }
        }
    }

    /// 누르기 전 미리 보기(입력 중 표시·실패 줄의 추천 다시 받기). 등록은 하지 않는다.
    pub fn preview_shortcut(&mut self, id: &str, combo: &str) -> PreviewResult {
        let lang = self.lang();
        let Some(slot) = self.slot(id) else {
            return PreviewResult { status: RowStatus::Conflict, reason: Some(self.t("error.hotkey.unknownSlot")), owner: None, suggestions: vec![] };
        };
        let Some(combo) = canonicalize(combo, self.platform) else {
            return PreviewResult { status: RowStatus::Conflict, reason: Some(self.t("error.hotkey.badCombo")), owner: None, suggestions: vec![] };
        };
        let a = self.analyze(slot, &combo);
        let status = Self::status_of(&a);
        let suggestions = if status == RowStatus::Conflict || self.failures.contains_key(id) {
            let avoid: BTreeSet<String> = self.offered.iter().filter(|(k, _)| k.as_str() != id).map(|(_, v)| v.clone()).collect();
            catalog().suggestions(slot, self.platform, &self.assigned_except(id), &self.env, &avoid, 3)
        } else {
            vec![]
        };
        PreviewResult { status, reason: a.reason_for(lang), owner: a.top().and_then(|t| t.owner.clone()), suggestions }
    }

    /// 묶음 [기본값] — 그 묶음만 확정본 값으로. 등록 실패 줄은 이전 값 유지 + 이유(05 §2 U).
    pub fn reset_shortcut_group(&mut self, group: &str) -> InkState {
        let slots: Vec<&'static SlotDef> = catalog().slots_for(self.platform).filter(|s| s.group == group).collect();
        let mut done = Vec::new();
        let mut failed = 0;
        for slot in slots {
            let def = catalog().default_combo(&slot.id, self.platform);
            let prev = self.settings.shortcut(&slot.id).to_string();
            if def == prev {
                continue;
            }
            match self.apply_shortcut(slot, &def) {
                Ok(_) => done.push(Undo::Shortcut { id: slot.id.clone(), prev }),
                Err(f) => {
                    failed += 1;
                    self.failures.insert(slot.id.clone(), f);
                }
            }
        }
        if !done.is_empty() {
            if self.save().is_err() {
                self.revert(Undo::Batch(done));
                self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.saveError")));
                self.emit_state();
                return self.state();
            }
            self.history.push(Undo::Batch(done));
            let text = if failed == 0 {
                self.t("island.settings.shortcutsRestored")
            } else {
                self.tp("island.settings.shortcutsRestoredPartial", failed as u64)
            };
            self.island(IslandEvent::new(IslandKind::Result, text).undo_button_for(self.lang()));
        }
        self.emit_state();
        self.state()
    }

    /// 지금 화면 언어(설정 `general.lang`, R2-I18N).
    pub fn lang(&self) -> ink_domain::i18n::Lang {
        ink_domain::i18n::Lang::from_setting(self.settings.str("general.lang"))
    }

    /// 지금 언어로 키의 글(Rust가 만드는 섬·오류·메뉴 글은 이것으로 — 문자열을 코드에 박지 않는다).
    /// 지금 OS 변형(`key~mac`·`key~win`)이 있으면 그것을 쓴다(R2-OSFIT-1).
    pub(crate) fn t(&self, key: &str) -> String {
        ink_domain::i18n::tr_os(self.lang(), self.platform, key)
    }

    /// [`Self::t`] + `{이름}` 끼우기.
    pub(crate) fn tf(&self, key: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
        ink_domain::i18n::trf_os(self.lang(), self.platform, key, args)
    }

    /// 복수형 선택(`{base}.one` / `{base}.other`) + `{n}` 끼우기.
    pub(crate) fn tp(&self, base: &str, n: u64) -> String {
        let key = if n == 1 { format!("{base}.one") } else { format!("{base}.other") };
        self.tf(&key, &[("n", &n)])
    }

    /// 단축키 잠시 끄기(`` Alt+` ``만 남김) · 다시 켜기(실패 줄 보고).
    pub fn set_hotkeys_enabled(&mut self, enabled: bool) -> InkState {
        if enabled == self.hotkeys_enabled {
            return self.state();
        }
        if self.capturing_keys {
            // 녹화 중: 값만 바꾸고 등록은 녹화가 끝날 때(`end_shortcut_capture` → `start`)
            self.hotkeys_enabled = enabled;
            return self.state();
        }
        self.hotkeys_enabled = enabled;
        if enabled {
            self.start();
            let bad = self.failures.len();
            let ev = if bad == 0 {
                IslandEvent::new(IslandKind::Info, self.t("island.hotkeys.on"))
            } else {
                IslandEvent::new(IslandKind::Problem, self.tp("island.hotkeys.onSomeCount", bad as u64))
            };
            self.island(ev);
        } else {
            let ids: Vec<String> = self.registered.keys().filter(|k| k.as_str() != PAUSE_SLOT).cloned().collect();
            for id in ids {
                if let Some(accel) = self.registered.remove(&id) {
                    self.ports.hotkeys.unregister(&accel);
                }
            }
            self.failures.retain(|k, _| k == PAUSE_SLOT);
            let pause = self.settings.shortcut(PAUSE_SLOT);
            let mut ev = IslandEvent::new(IslandKind::Off, self.t("island.hotkeys.off"));
            if !pause.is_empty() {
                ev = ev.detail(spoken_for(pause, self.platform, self.lang()));
            }
            self.island(ev);
        }
        self.emit_state();
        self.state()
    }

    /// 설정에서 단축키 녹화를 시작한다(R2-KEY-REC-1): 녹화 칸이 키를 받도록 **모든 전역 키(잠시 끄기 칸 포함)를 조용히 푼다**.
    /// 섬을 띄우지 않고 `hotkeys_enabled`도 그대로다(사용자가 끈 것과 다름). 이미 녹화 중이면 아무 일도 없다.
    pub fn begin_shortcut_capture(&mut self) {
        if self.capturing_keys {
            return;
        }
        self.capturing_keys = true;
        for (_, accel) in std::mem::take(&mut self.registered) {
            self.ports.hotkeys.unregister(&accel);
        }
        log::debug!("shortcut capture begin");
    }

    /// 녹화를 끝낸다 — 지금 설정 값으로 전역 키를 다시 등록한다(끈 상태면 잠시 끄기 칸만). 녹화 중이 아니면 아무 일도 없다.
    /// 설정 창이 닫히거나 뒤로 가거나 시간이 지나면 어댑터가 부른다(R2-KEY-REC-2) — 몇 번 불러도 안전.
    pub fn end_shortcut_capture(&mut self) {
        if !self.capturing_keys {
            return;
        }
        self.capturing_keys = false;
        let bad = self.start();
        log::debug!("shortcut capture end ({bad} failures)");
        self.emit_state();
    }

    pub fn capturing_shortcut(&self) -> bool {
        self.capturing_keys
    }

    /// 어댑터가 전역 키 눌림을 넘긴다.
    pub fn on_hotkey(&mut self, slot: &str) -> HotkeyOutcome {
        self.on_hotkey_at(slot, Instant::now())
    }

    /// `pressed` = 어댑터가 키를 받은 순간(진입 계측 M1의 시작점).
    pub fn on_hotkey_at(&mut self, slot: &str, pressed: Instant) -> HotkeyOutcome {
        // 녹화 중에는 운영체제가 키를 못 풀었더라도 아무것도 실행하지 않는다(R2-KEY-REC-4 이중 안전)
        if self.capturing_keys {
            return HotkeyOutcome::Ignored;
        }
        if slot == PAUSE_SLOT {
            let next = !self.hotkeys_enabled;
            self.set_hotkeys_enabled(next);
            return HotkeyOutcome::Toggled(next);
        }
        // 실시간 확대(Alt+2)·확대 안 키·확대 중 다른 모드 키(live_zoom.rs)
        if let Some(out) = self.zoom_hotkey(slot, pressed) {
            return out;
        }
        if !self.hotkeys_enabled || !self.registered.contains_key(slot) {
            return HotkeyOutcome::Ignored;
        }
        if let Some(outcome) = self.on_draw_hotkey(slot, pressed) {
            return outcome;
        }
        // 단계 6: 녹화 중 F3 = 중지, 바로 캡처 키 4종(R-CAP-13)
        if let Some(outcome) = self.stage6_hotkey(slot) {
            return outcome;
        }
        match mode::on_hotkey(&self.mode, slot, self.now_ms()) {
            ModeCommand::Start(kind) => {
                self.begin_capture(kind, pressed);
                HotkeyOutcome::Captured
            }
            ModeCommand::Switch(kind) => {
                self.end_capture();
                self.begin_capture(kind, pressed);
                HotkeyOutcome::Captured
            }
            ModeCommand::End => {
                self.end_capture();
                HotkeyOutcome::Captured
            }
            ModeCommand::ActiveWindow => {
                // F2 두 번(400 ms 안) = 지금 활성 창(R-WIN-2)
                self.end_capture();
                self.capture_active_window();
                HotkeyOutcome::Captured
            }
            ModeCommand::Ignore => HotkeyOutcome::Start(slot.to_string()),
            // 판서 명령은 위 on_draw_hotkey가 이미 처리했다
            ModeCommand::StartDraw(_) | ModeCommand::SwitchDraw(_) | ModeCommand::DrawRestore => HotkeyOutcome::Draw,
        }
    }

    // ───────────── 캡처 진입(단계 2): 정지 스냅숏 → 오버레이 ─────────────

    pub(crate) fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    /// 현재 모니터 배치(진단 기록 묶기 R2-DIAG-1용 — 장치 이름 없이 해상도·배율만).
    pub fn monitor_layout(&self) -> MonitorLayout {
        self.ports.monitors.layout()
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn snapshot(&self) -> Option<&Snapshot> {
        self.snapshot.as_ref()
    }

    /// 강조를 옮긴다(단계 3의 커서 질의 결과·Tab·휠). 지금 세션일 때만.
    pub fn set_highlight(&mut self, rect: Option<ink_domain::geometry::PhysRect>) {
        if let Mode::Capture { session, .. } = self.mode {
            self.ports.overlay.highlight(session, rect);
        }
    }

    /// 배치가 바뀌었으면 오버레이 창을 모니터에 다시 맞춘다. 바뀌었으면 true.
    pub(crate) fn prepare_overlays(&mut self) -> bool {
        let layout = self.ports.monitors.layout();
        if self.prepared.as_ref().map(|l| l.fingerprint()) == Some(layout.fingerprint()) {
            return false;
        }
        self.ports.overlay.prepare(&layout);
        self.prepared = Some(layout);
        true
    }

    fn excluded_apps(&self) -> Vec<ExcludedApp> {
        self.settings.get("capture.excluded").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
    }

    /// 트레이 메뉴·명령에서 시작(단축키가 꺼져 있어도). 이미 캡처 중이면 새로 시작.
    pub fn start_capture(&mut self, kind: CaptureKind) {
        // 확대 중이면 돋보기가 화면에서 사라진 뒤에 찍는다(확대 화면이 찍히지 않게)
        self.end_zoom_settled();
        self.end_capture();
        self.begin_capture(kind, Instant::now());
    }

    /// 캡처 먼저 → 표시 나중(R-CAP-1). 창 목록과 이미지는 같은 순간(08 §4.1-2): 창 목록(가벼움)을 바로 앞에서 찍는다.
    fn begin_capture(&mut self, kind: CaptureKind, pressed: Instant) {
        if self.ports.probe.secure_desktop() {
            if !self.secure_notified {
                self.secure_notified = true;
                self.island(IslandEvent::new(IslandKind::Off, self.t("island.security")).variant("security"));
            }
            return;
        }
        // 새 캡처가 지난 결과 섬([판서로])·버린 주석 되살리기를 덮는다 — 쥐어 둔 프레임을 놓는다
        self.cap.handoff = None;
        self.clear_discarded_capture();
        let t = Instant::now();
        self.prepare_overlays();
        let layout = self.prepared.clone().unwrap_or_default();
        let cursor = self.ports.monitors.cursor();
        let probe_ms = ms(t.elapsed());

        // 창 목록과 이미지를 나란히 — 같은 순간에 가깝고(08 §4.1-2) 진입 시간도 줄어든다
        let Ports { windows: catalog, screen, .. } = &mut self.ports;
        let catalog: &dyn crate::ports::WindowCatalog = catalog.as_ref();
        let ((raw, windows_ms), (captured, capture_ms)) = std::thread::scope(|s| {
            let list = s.spawn(|| {
                let t = Instant::now();
                let raw = catalog.windows();
                (raw, ms(t.elapsed()))
            });
            let t = Instant::now();
            let captured = screen.capture(&layout);
            let cap = (captured, ms(t.elapsed()));
            (list.join().unwrap_or_default(), cap)
        });
        // 커서 모양도 같은 순간에(오버레이가 뜨기 전 — 뜬 뒤엔 십자 커서로 바뀜)
        self.cap.cursor = self.ports.screen.cursor_sprite();
        let frames = match captured {
            Ok(f) if !f.is_empty() => f,
            Err(e) if e.starts_with(crate::NO_SCREEN_PERMISSION) => {
                log::warn!("capture failed: {e}");
                // 조용히 실패하지 않는다(M10): 단추 하나로 요청 + 설정 칸 열기
                self.island(self.no_screen_permission_island());
                return;
            }
            other => {
                log::warn!("capture failed: {}", other.err().as_deref().unwrap_or("no frames"));
                self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.screenError")));
                return;
            }
        };

        self.session_seq += 1;
        let session = self.session_seq;
        let windows = filter_windows(&raw, &layout, std::process::id(), &self.excluded_apps());
        if self.case_dir_on() {
            let own = std::process::id();
            self.cap.raw = raw.into_iter().filter(|w| w.pid != own).collect();
        }
        let snapshot = Snapshot { id: session, kind, layout: layout.clone(), windows, cursor };
        // 기본 강조: F1·F3 = 커서 모니터 전체(작업 표시줄 포함), F2 = 커서 아래 창(Snapshot::default_highlight)
        let highlight = snapshot.default_highlight();
        let overlay = OverlaySession {
            session,
            kind,
            cursor,
            native_frame: false,
            highlight,
            monitors: layout
                .monitors
                .iter()
                .map(|m| OverlayMonitor { id: m.id, bounds: m.bounds, work: m.work, scale: m.scale, primary: m.primary, frame_url: None })
                .collect(),
            restored_annotations: None,
        };

        // 자르기·V 전처리용으로 쥐어 둔다(픽셀은 Arc라 복사 없음)
        let kept = frames.clone();
        // 근거 준비는 화면을 띄우기 전에 배경에서 시작한다 — 커서 아래 창 UIA 색인(미리 깨우기, D9)과
        // V 전처리가 정지 화면 그리기와 겹친다(08 §4.7 "캡처와 겹쳐서 0 ms 추가"). 메인 스레드는 기다리지 않는다.
        self.cap.probe = crate::capture::EntryProbe { pressed: Some(pressed), window: snapshot.window_at(cursor, true).map(|w| w.handle), ..Default::default() };
        self.warm_evidence(&snapshot, &kept, cursor);

        let t = Instant::now();
        let painted_at = match self.ports.overlay.show(&overlay, frames) {
            Ok(at) => at,
            Err(e) => {
                log::warn!("overlay show failed: {e}");
                self.ports.overlay.hide(session);
                self.end_selection();
                self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.openError")));
                return;
            }
        };
        let show_ms = ms(t.elapsed());

        let timing = EntryTiming {
            probe_ms,
            windows_ms,
            capture_ms,
            show_ms,
            frame_ms: painted_at.map(|at| ms(at.saturating_duration_since(pressed))),
            monitors: layout.monitors.len(),
            windows: snapshot.windows.len(),
            ..Default::default()
        };
        self.pending = Some((pressed, timing));
        self.snapshot = Some(snapshot);
        self.mode = Mode::Capture { session, kind, started_ms: self.now_ms() };
        let mut ev = self.capture_island(kind);
        // 맥: 손쉬운 사용이 없으면 요소(A)를 못 잡는다 — 이번 실행 첫 캡처의 모드 섬에 칩 하나(M10, 거절해도 W·V로 캡처)
        if self.permissions.is_some_and(|p| !p.accessibility) && !self.ax_chip_shown {
            self.ax_chip_shown = true;
            ev.chip = Some(IslandButton { label: self.t("island.perm.axChip"), action: ACTION_PERM_AX.into() });
        }
        self.island(ev);
        self.emit_state();
        // 고르기 시작: 상태 기계(근거는 위에서 이미 준비 중)
        self.start_selection(kept, cursor);
        // 단계 6: F2 = 커서 아래 창 강조 + 창 캡처 준비, F3 = 녹화기 미리 준비
        self.after_entry(kind, cursor);
    }

    /// UI가 정지 화면을 그렸다(첫 모니터만 센다) → 계측 한 줄(M1).
    pub fn overlay_ready(&mut self, session: u64) -> Option<EntryTiming> {
        self.overlay_ready_with(session, None, None)
    }

    /// `ui_load_ms`·`ui_paint_ms` = UI가 잰 세션 알림→이미지 load, load→그림.
    pub fn overlay_ready_with(&mut self, session: u64, ui_load_ms: Option<f64>, ui_paint_ms: Option<f64>) -> Option<EntryTiming> {
        if !matches!(self.mode, Mode::Capture { session: s, .. } if s == session) {
            return None;
        }
        let (pressed, mut timing) = self.pending.take()?;
        timing.ready_ms = Some(ms(pressed.elapsed()));
        timing.ui_load_ms = ui_load_ms;
        timing.ui_paint_ms = ui_paint_ms;
        self.record_timing(&timing);
        Some(timing)
    }

    pub(crate) fn record_timing(&mut self, timing: &EntryTiming) {
        log::info!("{}", timing.log_line());
        if self.ready_samples.len() == TIMING_KEEP {
            self.ready_samples.pop_front();
        }
        self.ready_samples.push_back(timing.entry_ms());
    }

    /// 이번 실행의 진입 시간 통계(p50·p95).
    pub fn entry_stats(&self) -> Option<Stats> {
        timing::stats(&self.ready_samples.iter().copied().collect::<Vec<_>>())
    }

    /// 모드 끝(Esc·우클릭·같은 키·잠금). 정지 이미지는 어댑터가 버린다.
    pub fn end_capture(&mut self) {
        let Mode::Capture { session, .. } = self.mode else { return };
        self.ports.overlay.hide(session);
        // 녹화 중이면 녹화 상태로 돌아간다(녹화는 캡처와 겹칠 수 있음)
        self.mode = self.rest_mode();
        self.snapshot = None;
        self.end_selection();
        self.after_capture_end();
        // UI가 알리기 전에 끝났어도 네이티브 정지 화면 시간은 남긴다
        if let Some((_, timing)) = self.pending.take() {
            if timing.frame_ms.is_some() {
                self.record_timing(&timing);
            }
        }
        self.island(IslandEvent::new(IslandKind::End, ""));
        self.emit_state();
    }

    /// 잠금·절전이면 모드를 끝내고, 배치가 바뀌었거나 깨어나면 오버레이를 다시 맞춘다(R-SYS-3, 02 §7).
    pub fn on_system_event(&mut self, ev: SystemEvent) {
        self.draw_on_system_event(ev);
        self.zoom_system_event(ev);
        // 잠금·절전·화면 구성 변경 뒤의 [판서로]·되살리기는 쓸 수 없다 — 쥐어 둔 범위·프레임을 놓는다
        if matches!(ev, SystemEvent::Lock | SystemEvent::Suspend | SystemEvent::DisplayChange) {
            self.cap.handoff = None;
            self.clear_discarded_capture();
        }
        // 단계 6: 잠금·절전·모니터 해제 → 녹화 자동 중지·저장(05 §8). 배치를 다시 맞추기 전에 본다
        self.record_on_system_event(ev);
        match ev {
            SystemEvent::Lock | SystemEvent::Suspend => self.end_capture(),
            SystemEvent::DisplayChange => {
                if matches!(self.mode, Mode::Capture { .. }) {
                    self.end_capture();
                    self.island(IslandEvent::new(IslandKind::Info, self.t("island.capture.displayChanged")));
                }
                self.prepare_overlays();
            }
            SystemEvent::Unlock | SystemEvent::Resume => {
                self.secure_notified = false;
                // 깨어난 뒤에는 지문이 같아도 창을 다시 맞춘다(투명도·위치 재지정, R-SYS-6)
                self.prepared = None;
                self.prepare_overlays();
            }
            SystemEvent::ThemeChange => self.emit_state(),
        }
    }

    // ───────────── 설정 ─────────────

    /// 바꾸는 순간 바로 반영(R-SET-0). 검증 실패·저장 실패면 값은 그대로.
    pub fn update_setting(&mut self, path: &str, value: &Value) -> UpdateResult {
        let lang = self.lang();
        let prev = match self.settings.set_for(path, value, self.platform, lang) {
            Ok(prev) => prev,
            Err(e) => return UpdateResult { ok: false, error: Some(e.message(lang)), state: self.state() },
        };
        if prev == *self.settings.get(path).unwrap_or(&Value::Null) {
            return UpdateResult { ok: true, error: None, state: self.state() };
        }
        if let Err(f) = self.save() {
            let _ = self.settings.set_for(path, &prev, self.platform, lang);
            self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.saveError")));
            return UpdateResult { ok: false, error: Some(f.reason), state: self.state() };
        }
        self.history.push(Undo::Setting { path: path.to_string(), prev });
        if path == "general.lang" {
            self.emit_zoom();
            match self.mode {
                Mode::Capture { kind, .. } => self.island(self.capture_island(kind)),
                Mode::Draw { .. } => self.emit_draw_island(),
                Mode::LiveZoom { .. } => {
                    if let Some(v) = self.zoom.view {
                        self.island(self.zoom_island(v.target));
                    }
                }
                Mode::Recording { .. } => {
                    let elapsed = self.clock_ms().saturating_sub(self.rec.started_ms);
                    self.island(self.record_island(elapsed));
                }
                Mode::Idle => {}
            }
        }
        self.emit_state();
        UpdateResult { ok: true, error: None, state: self.state() }
    }

    /// 설정 창을 열 때·닫을 때 — 되돌리기 기록은 "이번에 연 동안"만(05 §1).
    pub fn begin_settings_session(&mut self) {
        self.history.clear();
        self.offered.clear();
        self.refresh_environment();
        self.emit_state();
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// 마지막 변경 되돌리기(설정 창 Ctrl+Z · 섬 [되돌리기]).
    pub fn undo(&mut self) -> UpdateResult {
        let Some(entry) = self.history.pop() else {
            return UpdateResult { ok: false, error: None, state: self.state() };
        };
        self.revert(entry);
        let saved = self.save();
        self.emit_state();
        UpdateResult { ok: saved.is_ok(), error: saved.err().map(|f| f.reason), state: self.state() }
    }

    fn revert(&mut self, entry: Undo) {
        let lang = self.lang();
        match entry {
            Undo::Setting { path, prev } => {
                let _ = self.settings.set_for(&path, &prev, self.platform, lang);
            }
            Undo::Shortcut { id, prev } => {
                if let Some(slot) = self.slot(&id) {
                    if let Err(f) = self.apply_shortcut(slot, &prev) {
                        self.failures.insert(id, f);
                    }
                }
            }
            Undo::Batch(list) => {
                for e in list.into_iter().rev() {
                    self.revert(e);
                }
            }
            Undo::Whole(prev) => {
                self.settings = *prev;
                self.reregister_all();
            }
        }
    }

    /// 등록을 모두 풀고 지금 설정으로 다시(가져오기·모두 처음으로·통째 되돌리기).
    fn reregister_all(&mut self) {
        let all: Vec<String> = std::mem::take(&mut self.registered).into_values().collect();
        for accel in all {
            self.ports.hotkeys.unregister(&accel);
        }
        self.failures.clear();
        self.offered.clear();
        // 녹화 중이면 녹화가 끝날 때 다시 등록한다
        if !self.capturing_keys {
            self.start();
        }
    }

    // ───────────── 내보내기 · 가져오기 · 모두 처음으로 (05 §1) ─────────────

    /// 내보낼 파일 내용(설정만, 기록·로그 제외).
    pub fn export_file(&self) -> Value {
        settings::export_file(&self.settings)
    }

    fn read_import(&self, file: &Value) -> Result<(Settings, Vec<String>), String> {
        let lang = self.lang();
        let data = settings::migrate(file).map_err(|e| e.message(lang))?;
        Ok(Settings::from_data_for(&data, self.platform, lang))
    }

    /// 무엇이 바뀌는지만 본다(취소하면 변경 0).
    pub fn preview_import(&self, file: &Value) -> ImportPreview {
        match self.read_import(file) {
            Ok((next, warnings)) => ImportPreview { ok: true, error: None, changes: self.settings.diff(&next), warnings },
            Err(e) => ImportPreview { ok: false, error: Some(e), changes: vec![], warnings: vec![] },
        }
    }

    /// [가져오기]를 누르면 한 번에 반영(되돌리기 기록 1건). 겹친 단축키 줄만 실패로 남는다(05 §11-11).
    pub fn apply_import(&mut self, file: &Value) -> UpdateResult {
        let (next, _) = match self.read_import(file) {
            Ok(x) => x,
            Err(e) => return UpdateResult { ok: false, error: Some(e), state: self.state() },
        };
        let prev = std::mem::replace(&mut self.settings, next);
        if let Err(f) = self.save() {
            self.settings = prev;
            self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.saveError")));
            return UpdateResult { ok: false, error: Some(f.reason), state: self.state() };
        }
        self.history.push(Undo::Whole(Box::new(prev)));
        self.reregister_all();
        let bad = self.failures.len();
        let text = if bad == 0 {
            self.t("island.settings.imported")
        } else {
            self.tp("island.settings.importedWithConflicts", bad as u64)
        };
        self.island(IslandEvent::new(IslandKind::Result, text).undo_button_for(self.lang()));
        self.emit_state();
        UpdateResult { ok: true, error: None, state: self.state() }
    }

    /// 모두 처음으로(확인 시트 뒤). 첫 실행 안내 기록은 남겨 둔다.
    pub fn reset_all(&mut self) -> InkState {
        let onboarding = self.settings.get("onboarding.done").cloned();
        let prev = std::mem::replace(&mut self.settings, Settings::defaults(self.platform));
        if let Some(v) = onboarding {
            let _ = self.settings.set("onboarding.done", &v);
        }
        if self.save().is_err() {
            self.settings = prev;
            self.island(IslandEvent::new(IslandKind::Problem, self.t("island.capture.saveError")));
            return self.state();
        }
        self.history.clear();
        self.clear_kept_ink();
        self.reregister_all();
        self.island(IslandEvent::new(IslandKind::Result, self.t("island.settings.reset")));
        self.emit_state();
        self.state()
    }

    pub fn set_pinned_count(&mut self, n: u32) {
        if self.pinned_count != n {
            self.pinned_count = n;
            self.emit_state();
        }
    }

    // ───────────── 내부 ─────────────

    fn save(&mut self) -> Result<(), Failure> {
        if self.readonly {
            return Ok(());
        }
        let lang = self.lang();
        self.ports
            .store
            .save_section("settings", &self.settings.to_data())
            .map_err(|e| Failure { kind: FailureKind::Message(e.message(lang)), reason: e.message(lang), owner: None })
    }

    pub(crate) fn emit_state(&self) {
        self.ports.notifier.state(&self.state());
    }

    pub fn island(&self, e: IslandEvent) {
        self.ports.notifier.island(&e);
    }
}
