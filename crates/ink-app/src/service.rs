//! InkService — 설정 바로 반영(R-SET-0), 단축키 즉시 등록 트랜잭션(R-KEY-3, 02 §6), 창 안 되돌리기.
//!
//! 단축키 바꾸기 = "새 키 등록 → 성공하면 옛 키 해제 → 저장". 어느 단계든 실패하면 옛 키 그대로.
//! 카탈로그로 추정한 다른 앱 겹침(Snipaste 등)은 실제로 등록해 보고 확인하고,
//! 규칙·이 표·설정 파일에서 읽은 키(PowerToys 저수준 훅)는 등록이 성공해도 막는다.

use crate::ports::{HotkeyError, Ports};
use crate::state::*;
use ink_domain::settings::{self, LoadError, Settings};
use ink_domain::shortcuts::{canonicalize, catalog, spoken, to_accelerator, Analysis, Environment, Level, Platform, SlotDef};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

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
struct Failure {
    reason: String,
    owner: Option<String>,
}

#[derive(Clone, Debug)]
enum Undo {
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
    /// 모드 시작 등 — 칸 ID
    Start(String),
    /// 단축키 잠시 끄기·켜기를 처리함
    Toggled(bool),
    Ignored,
}

pub struct InkService {
    ports: Ports,
    platform: Platform,
    version: String,
    settings: Settings,
    env: Environment,
    hotkeys_enabled: bool,
    /// 칸 → 등록한 가속기
    registered: BTreeMap<String, String>,
    /// 등록하지 못한 전역 칸과 이유
    failures: BTreeMap<String, Failure>,
    /// 실패한 줄마다 보여 준 첫 추천 — 다른 줄과 겹치지 않게
    offered: BTreeMap<String, String>,
    history: Vec<Undo>,
    readonly: bool,
    notices: Vec<String>,
    pinned_count: u32,
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
        };
        svc.refresh_environment();
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
        if slot == PAUSE_SLOT {
            let next = !self.hotkeys_enabled;
            self.set_hotkeys_enabled(next);
            return HotkeyOutcome::Toggled(next);
        }
        if !self.hotkeys_enabled || !self.registered.contains_key(slot) {
            return HotkeyOutcome::Ignored;
        }
        HotkeyOutcome::Start(slot.to_string())
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
