//! InkService — 설정 바로 반영(R-SET-0), 단축키 즉시 등록 트랜잭션(R-KEY-3, 02 §6), 창 안 되돌리기.
//!
//! 단축키 바꾸기 = "새 키 등록 → 성공하면 옛 키 해제 → 저장". 어느 단계든 실패하면 옛 키 그대로.
//! 카탈로그로 추정한 다른 앱 겹침(Snipaste 등)은 실제로 등록해 보고 확인하고,
//! 규칙·이 표·설정 파일에서 읽은 키(PowerToys 저수준 훅)는 등록이 성공해도 막는다.

use crate::ports::{HotkeyError, Ports, SystemEvent};
use crate::state::*;
use ink_domain::geometry::MonitorLayout;
use ink_domain::mode::{self, Mode, ModeCommand};
use ink_domain::settings::{self, ExcludedApp, LoadError, Settings};
use ink_domain::shortcuts::{canonicalize, catalog, spoken, to_accelerator, Analysis, Environment, Level, Platform, SlotDef};
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
pub(crate) struct Failure {
    reason: String,
    owner: Option<String>,
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
    /// 단축키 잠시 끄기·켜기를 처리함
    Toggled(bool),
    Ignored,
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
    /// 칸 → 등록한 가속기
    pub(crate) registered: BTreeMap<String, String>,
    /// 등록하지 못한 전역 칸과 이유
    pub(crate) failures: BTreeMap<String, Failure>,
    /// 실패한 줄마다 보여 준 첫 추천 — 다른 줄과 겹치지 않게
    pub(crate) offered: BTreeMap<String, String>,
    pub(crate) history: Vec<Undo>,
    pub(crate) readonly: bool,
    pub(crate) notices: Vec<String>,
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
}

/// 모드 섬 둘째 줄(처음 3번과 `?`일 때만 보임 — 섬 호스트가 정함).
fn capture_island(kind: CaptureKind) -> IslandEvent {
    let (text, variant, hints): (&str, &str, &[(&str, &str)]) = match kind {
        CaptureKind::Region => ("범위 캡처", "region", &[("휠", "넓히기·좁히기"), ("Tab", "창"), ("Enter", "복사"), ("Esc", "취소")]),
        CaptureKind::Window => ("창 캡처", "window", &[("클릭", "찍기"), ("F2", "활성 창"), ("Esc", "취소")]),
        CaptureKind::Record => ("녹화 범위", "region", &[("드래그", "범위"), ("클릭", "모니터"), ("Esc", "취소")]),
    };
    let mut ev = IslandEvent::new(IslandKind::Mode, text).variant(variant).hints(hints);
    ev.mode = Some(kind.slot().into());
    ev
}

fn failure_of(a: &Analysis) -> Failure {
    let top = a.hard_block().or_else(|| a.top());
    Failure {
        reason: a.reason().or_else(|| top.map(|t| t.what.clone())).unwrap_or_else(|| "쓸 수 없는 키예요".into()),
        owner: top.and_then(|t| t.owner.clone()),
    }
}

impl InkService {
    pub fn new(mut ports: Ports, platform: Platform, version: impl Into<String>) -> Self {
        let loaded = ports.store.load();
        // 범위 기록(05 §4) — 설정과 같은 파일의 history 구역
        let ranges = loaded
            .file
            .as_ref()
            .and_then(|f| f.pointer("/sections/history/data"))
            .and_then(|v| serde_json::from_value::<ink_domain::history::History>(v.clone()).ok())
            .unwrap_or_default();
        let mut notices: Vec<String> = loaded.notice.into_iter().collect();
        let mut readonly = ports.store.location().read_only;
        let mut first_run = false;
        let settings = match loaded.file {
            None => {
                first_run = true;
                Settings::defaults(platform)
            }
            Some(file) => match settings::migrate(&file) {
                Ok(data) => {
                    let (s, warnings) = Settings::from_data(&data, platform);
                    notices.extend(warnings);
                    s
                }
                Err(e @ LoadError::NewerSchema(_)) => {
                    readonly = true;
                    notices.push(e.to_string());
                    let data = file.pointer("/sections/settings/data").cloned().unwrap_or(Value::Null);
                    Settings::from_data(&data, platform).0
                }
                Err(e) => {
                    notices.push(format!("{e} — 처음 값으로 시작해요"));
                    Settings::defaults(platform)
                }
            },
        };
        if readonly && ports.store.location().read_only {
            notices.push("설정 폴더에 쓸 수 없어 이번에는 저장하지 않아요".into());
        }
        let mut svc = InkService {
            ports,
            platform,
            version: version.into(),
            settings,
            env: catalog().default_environment(),
            hotkeys_enabled: true,
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
        InkState {
            platform: match self.platform {
                Platform::Win => "windows",
                Platform::Mac => "macos",
            },
            theme: self.settings.str("general.theme").to_string(),
            hotkeys_enabled: self.hotkeys_enabled,
            pinned_count: self.pinned_count,
            recording: Recording::default(),
            settings: self.settings.values.clone(),
            conflicts: ConflictSummary { count: bad.len(), app, combos: bad.iter().map(|r| r.combo.clone()).collect() },
            shortcuts,
            version: self.version.clone(),
            readonly: self.readonly,
            location: self.ports.store.location(),
            notices: self.notices.clone(),
            mode: self.mode,
        }
    }

    fn assigned_except(&self, id: &str) -> BTreeMap<String, String> {
        self.settings.shortcuts.iter().filter(|(k, _)| k.as_str() != id).map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    fn slot(&self, id: &str) -> Option<&'static SlotDef> {
        catalog().slot(id).filter(|s| !(self.platform == Platform::Mac && s.windows_only))
    }

    fn analyze(&self, slot: &SlotDef, combo: &str) -> Analysis {
        catalog().analyze(slot, self.platform, combo, &self.assigned_except(&slot.id), &self.env)
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
        catalog()
            .slots_for(self.platform)
            .map(|s| {
                let combo = self.settings.shortcut(&s.id).to_string();
                let mut a = self.analyze(s, &combo);
                if self.registered.contains_key(&s.id) {
                    a = a.confirmed_free();
                }
                let (status, reason, owner) = match self.failures.get(&s.id) {
                    Some(f) => (RowStatus::Conflict, Some(f.reason.clone()), f.owner.clone()),
                    None => {
                        let status = Self::status_of(&a);
                        let reason = if a.empty { Some("트레이 메뉴로 시작할 수 있어요".into()) } else { a.reason() };
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
    pub fn refresh_environment(&mut self) {
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
        if self.hotkeys_enabled {
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
        let a = self.analyze(slot, combo);
        if a.hard_block().is_some() {
            return Err(failure_of(&a));
        }
        let accel = to_accelerator(combo, self.platform).ok_or(Failure { reason: "이 키는 등록할 수 없어요".into(), owner: None })?;
        match self.ports.hotkeys.register(&slot.id, &accel) {
            Ok(()) => Ok(accel),
            Err(HotkeyError::Taken) => {
                // 카탈로그 추정이 있으면 그 앱 이름, 없으면 실행 중인 대체 앱 중 하나도 확신할 수 없으니 "다른 앱"
                let owner = a.items.iter().find(|i| i.verifiable).and_then(|i| i.owner.clone());
                Err(Failure {
                    reason: owner.as_ref().map(|o| format!("{o}에서 먼저 쓰고 있어요")).unwrap_or_else(|| "다른 앱이 먼저 쓰고 있어요".into()),
                    owner,
                })
            }
            Err(e) => Err(Failure { reason: e.to_string(), owner: None }),
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
            return Err(failure_of(&a));
        }
        let live = slot.is_global() && (self.hotkeys_enabled || slot.id == PAUSE_SLOT);
        let mut status = Self::status_of(&a);
        if live && !combo.is_empty() {
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
        self.settings.set_shortcut(&slot.id, combo, self.platform).map_err(|e| Failure { reason: e.to_string(), owner: None })?;
        Ok(status)
    }

    fn fail_result(&mut self, slot: Option<&SlotDef>, f: Failure) -> SetShortcutResult {
        let suggestions = slot.map(|s| self.suggest(s)).unwrap_or_default();
        SetShortcutResult { ok: false, status: RowStatus::Conflict, reason: Some(f.reason), owner: f.owner, suggestions }
    }

    /// 키를 누르는 순간(S02). 빈 값 = 비우기.
    pub fn set_shortcut(&mut self, id: &str, combo: &str) -> SetShortcutResult {
        let Some(slot) = self.slot(id) else {
            return self.fail_result(None, Failure { reason: "모르는 칸이에요".into(), owner: None });
        };
        let Some(combo) = canonicalize(combo, self.platform) else {
            return self.fail_result(Some(slot), Failure { reason: "이 키는 읽을 수 없어요".into(), owner: None });
        };
        let prev = self.settings.shortcut(id).to_string();
        if combo == prev && !self.failures.contains_key(id) {
            let a = self.analyze(slot, &combo);
            return SetShortcutResult { ok: true, status: Self::status_of(&a), reason: a.reason(), owner: None, suggestions: vec![] };
        }
        match self.apply_shortcut(slot, &combo) {
            Ok(status) => {
                if let Err(f) = self.save() {
                    let _ = self.apply_shortcut(slot, &prev);
                    self.island(IslandEvent::new(IslandKind::Problem, "저장하지 못했어요"));
                    return self.fail_result(None, f);
                }
                self.history.push(Undo::Shortcut { id: id.to_string(), prev });
                let text = if combo.is_empty() { "비웠어요 · 트레이 메뉴로 시작할 수 있어요".to_string() } else { format!("{}로 바꿨어요", spoken(&combo, self.platform)) };
                self.island(IslandEvent::new(IslandKind::Result, text).undo_button());
                self.emit_state();
                let reason = if status == RowStatus::Warn { self.analyze(slot, &combo).reason() } else { None };
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
        let Some(slot) = self.slot(id) else {
            return PreviewResult { status: RowStatus::Conflict, reason: Some("모르는 칸이에요".into()), owner: None, suggestions: vec![] };
        };
        let Some(combo) = canonicalize(combo, self.platform) else {
            return PreviewResult { status: RowStatus::Conflict, reason: Some("이 키는 읽을 수 없어요".into()), owner: None, suggestions: vec![] };
        };
        let a = self.analyze(slot, &combo);
        let status = Self::status_of(&a);
        let suggestions = if status == RowStatus::Conflict || self.failures.contains_key(id) {
            let avoid: BTreeSet<String> = self.offered.iter().filter(|(k, _)| k.as_str() != id).map(|(_, v)| v.clone()).collect();
            catalog().suggestions(slot, self.platform, &self.assigned_except(id), &self.env, &avoid, 3)
        } else {
            vec![]
        };
        PreviewResult { status, reason: a.reason(), owner: a.top().and_then(|t| t.owner.clone()), suggestions }
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
                self.island(IslandEvent::new(IslandKind::Problem, "저장하지 못했어요"));
                self.emit_state();
                return self.state();
            }
            self.history.push(Undo::Batch(done));
            let text = if failed == 0 { "기본값으로 되돌렸어요".to_string() } else { format!("기본값으로 되돌렸어요 · {failed}개는 겹쳐서 그대로예요") };
            self.island(IslandEvent::new(IslandKind::Result, text).undo_button());
        }
        self.emit_state();
        self.state()
    }

    /// 단축키 잠시 끄기(`` Alt+` ``만 남김) · 다시 켜기(실패 줄 보고).
    pub fn set_hotkeys_enabled(&mut self, enabled: bool) -> InkState {
        if enabled == self.hotkeys_enabled {
            return self.state();
        }
        self.hotkeys_enabled = enabled;
        if enabled {
            self.start();
            let bad = self.failures.len();
            let ev = if bad == 0 { IslandEvent::new(IslandKind::Info, "단축키를 다시 켰어요") } else { IslandEvent::new(IslandKind::Problem, format!("단축키를 켰어요 · {bad}개는 다른 앱이 쓰고 있어요")) };
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
            let mut ev = IslandEvent::new(IslandKind::Off, "단축키를 잠시 껐어요");
            if !pause.is_empty() {
                ev = ev.detail(spoken(pause, self.platform));
            }
            self.island(ev);
        }
        self.emit_state();
        self.state()
    }

    /// 어댑터가 전역 키 눌림을 넘긴다.
    pub fn on_hotkey(&mut self, slot: &str) -> HotkeyOutcome {
        self.on_hotkey_at(slot, Instant::now())
    }

    /// `pressed` = 어댑터가 키를 받은 순간(진입 계측 M1의 시작점).
    pub fn on_hotkey_at(&mut self, slot: &str, pressed: Instant) -> HotkeyOutcome {
        if slot == PAUSE_SLOT {
            let next = !self.hotkeys_enabled;
            self.set_hotkeys_enabled(next);
            return HotkeyOutcome::Toggled(next);
        }
        if !self.hotkeys_enabled || !self.registered.contains_key(slot) {
            return HotkeyOutcome::Ignored;
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
                // 단계 3(F2)에서 활성 창 캡처로 — 지금은 모드만 끝낸다
                self.end_capture();
                HotkeyOutcome::Start("active".into())
            }
            ModeCommand::Ignore => HotkeyOutcome::Start(slot.to_string()),
        }
    }

    // ───────────── 캡처 진입(단계 2): 정지 스냅숏 → 오버레이 ─────────────

    fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
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
    fn prepare_overlays(&mut self) -> bool {
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
        self.end_capture();
        self.begin_capture(kind, Instant::now());
    }

    /// 캡처 먼저 → 표시 나중(R-CAP-1). 창 목록과 이미지는 같은 순간(08 §4.1-2): 창 목록(가벼움)을 바로 앞에서 찍는다.
    fn begin_capture(&mut self, kind: CaptureKind, pressed: Instant) {
        if self.ports.probe.secure_desktop() {
            if !self.secure_notified {
                self.secure_notified = true;
                self.island(IslandEvent::new(IslandKind::Off, "보안 화면에서는 쓸 수 없어요").variant("security"));
            }
            return;
        }
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
                // 조용히 실패하지 않는다(M10): 어디서 켜는지까지
                self.island(IslandEvent::new(IslandKind::Problem, "화면 기록 권한이 필요해요").detail("시스템 설정 › 개인정보 보호 및 보안 › 화면 기록에서 켜 주세요"));
                return;
            }
            other => {
                log::warn!("capture failed: {}", other.err().as_deref().unwrap_or("no frames"));
                self.island(IslandEvent::new(IslandKind::Problem, "화면을 가져오지 못했어요"));
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
        };

        let t = Instant::now();
        // 자르기·V 전처리용으로 쥐어 둔다(픽셀은 Arc라 복사 없음)
        let kept = frames.clone();
        let painted_at = match self.ports.overlay.show(&overlay, frames) {
            Ok(at) => at,
            Err(e) => {
                log::warn!("overlay show failed: {e}");
                self.ports.overlay.hide(session);
                self.island(IslandEvent::new(IslandKind::Problem, "캡처 화면을 띄우지 못했어요"));
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
        self.island(capture_island(kind));
        self.emit_state();
        // 고르기 시작: 상태 기계 + 커서 아래 창 UIA 색인(미리 깨우기, D9) + V 전처리 — 모두 진입 뒤(진입 시간에 안 들어감)
        self.start_selection(kept, cursor);
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

    fn record_timing(&mut self, timing: &EntryTiming) {
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
        self.mode = Mode::Idle;
        self.snapshot = None;
        self.end_selection();
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
        match ev {
            SystemEvent::Lock | SystemEvent::Suspend => self.end_capture(),
            SystemEvent::DisplayChange => {
                if self.mode != Mode::Idle {
                    self.end_capture();
                    self.island(IslandEvent::new(IslandKind::Info, "화면 구성이 바뀌어 캡처를 닫았어요"));
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
        let prev = match self.settings.set(path, value) {
            Ok(prev) => prev,
            Err(e) => return UpdateResult { ok: false, error: Some(e.to_string()), state: self.state() },
        };
        if prev == *self.settings.get(path).unwrap_or(&Value::Null) {
            return UpdateResult { ok: true, error: None, state: self.state() };
        }
        if let Err(f) = self.save() {
            let _ = self.settings.set(path, &prev);
            self.island(IslandEvent::new(IslandKind::Problem, "저장하지 못했어요"));
            return UpdateResult { ok: false, error: Some(f.reason), state: self.state() };
        }
        self.history.push(Undo::Setting { path: path.to_string(), prev });
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
        match entry {
            Undo::Setting { path, prev } => {
                let _ = self.settings.set(&path, &prev);
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
        self.start();
    }

    // ───────────── 내보내기 · 가져오기 · 모두 처음으로 (05 §1) ─────────────

    /// 내보낼 파일 내용(설정만, 기록·로그 제외).
    pub fn export_file(&self) -> Value {
        settings::export_file(&self.settings)
    }

    fn read_import(&self, file: &Value) -> Result<(Settings, Vec<String>), String> {
        let data = settings::migrate(file).map_err(|e| e.to_string())?;
        Ok(Settings::from_data(&data, self.platform))
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
            self.island(IslandEvent::new(IslandKind::Problem, "저장하지 못했어요"));
            return UpdateResult { ok: false, error: Some(f.reason), state: self.state() };
        }
        self.history.push(Undo::Whole(Box::new(prev)));
        self.reregister_all();
        let bad = self.failures.len();
        let text = if bad == 0 { "설정을 가져왔어요".to_string() } else { format!("설정을 가져왔어요 · 단축키 {bad}개는 겹쳐요") };
        self.island(IslandEvent::new(IslandKind::Result, text).undo_button());
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
            self.island(IslandEvent::new(IslandKind::Problem, "저장하지 못했어요"));
            return self.state();
        }
        self.history.clear();
        self.reregister_all();
        self.island(IslandEvent::new(IslandKind::Result, "처음 상태로 되돌렸어요"));
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
        self.ports.store.save_section("settings", &self.settings.to_data()).map_err(|e| Failure { reason: e.to_string(), owner: None })
    }

    fn emit_state(&self) {
        self.ports.notifier.state(&self.state());
    }

    pub fn island(&self, e: IslandEvent) {
        self.ports.notifier.island(&e);
    }
}
