//! M 근거: MSAA 대체 경로(08 §4.2 M, R-CAP-4 · D12 · D17 · D19).
//!
//! - **UIA가 빈 창에만**: UIA 색인이 실패했거나 요소가 1개 이하인 창을 UIA 일꾼이 이어서 MSAA로 색인한다
//!   (`uia_is_empty`). 커서가 움직일 때는 여전히 메모리만 본다(D18).
//! - **일꾼 격리 + 마감(D17)**: 질의는 전용 MSAA 일꾼 스레드(MTA)에서 한다. 부르는 쪽(UIA 일꾼)은 `DEADLINE`까지만
//!   기다리고, 넘기면 그 일꾼을 버리고 다음 질의 때 새 일꾼을 띄운다. 멈춘 일꾼은 대상 앱이 풀리면 스스로 끝난다
//!   (보낼 곳이 없어 받기 고리가 끝남).
//! - **재질의 금지(D19)**: 한 번 마감을 넘긴 창·실행 파일은 앱을 끌 때까지 다시 묻지 않고, 버린 일꾼이
//!   `MAX_ABANDONED`개가 되면 MSAA를 아예 끈다(스레드 누수 상한).
//! - **걸음 예산**: 일꾼 안에서 `WALK_BUDGET` · 깊이 `MAX_DEPTH` · `MAX_ELEMENTS`개 · 마디당 자식 `MAX_CHILDREN`개 —
//!   자식을 끝없이 보고하는 앱(RB-20)도 여기서 끊긴다.
//! - **좌표(D12)**: DPI를 모르는 창은 `dpi::`로 물리 px로 바꾼다.
//! - 커서 점 질의(`AccessibleObjectFromPoint`)는 쓰지 않는다: 고르는 동안 우리 오버레이가 화면을 덮고 있어
//!   점 아래 창이 오버레이이고, 요소 소스 `request`는 점을 받지 않는다. 창 단위(`AccessibleObjectFromWindow`)로 색인한다.
//! - 이름·값·설명은 읽지 않는다(R-LOG-1) — 사각형·역할·상태만.

use ink_domain::geometry::PhysRect;
use ink_domain::recog::Control;
use std::collections::{HashSet, VecDeque};
use std::time::Duration;

/// 부르는 쪽이 MSAA 일꾼을 기다리는 마감(C §2.2).
pub const DEADLINE: Duration = Duration::from_millis(100);
/// 일꾼 안의 걸음 예산(08 §4.2 M "20 ms").
pub const WALK_BUDGET: Duration = Duration::from_millis(20);
pub const MAX_DEPTH: usize = 4;
pub const MAX_ELEMENTS: usize = 300;
/// 마디 하나에서 받을 자식 상한(`accChildCount`가 수백만을 말해도).
pub const MAX_CHILDREN: usize = 300;
/// 자식을 한 번에 받는 묶음 — 묶음 사이마다 시간 예산을 본다(자식 하나가 다른 프로세스 호출 하나라서).
pub const CHILD_CHUNK: usize = 32;
/// 버린(마감을 넘긴) 일꾼 상한 — 넘으면 MSAA를 끈다(C §5).
pub const MAX_ABANDONED: usize = 4;

/// `STATE_SYSTEM_INVISIBLE`
pub const STATE_INVISIBLE: u32 = 0x8000;
/// `STATE_SYSTEM_OFFSCREEN`
pub const STATE_OFFSCREEN: u32 = 0x1_0000;

/// UIA 결과가 비었는지(실패 = None). 08 §4.2 M: "A가 비거나 시간 초과일 때만".
pub fn uia_is_empty(found: Option<usize>) -> bool {
    found.map_or(true, |n| n <= 1)
}

/// MSAA 역할(`ROLE_SYSTEM_*`) → 컨트롤 종류. 이름·글자는 담지 않는다.
pub fn control_from_role(role: i32) -> Control {
    match role {
        0x01 => Control::TitleBar,                      // TITLEBAR
        0x02 | 0x0b => Control::Menu,                   // MENUBAR · MENUPOPUP
        0x0c => Control::MenuItem,                      // MENUITEM
        0x09 | 0x0a | 0x10 | 0x12 | 0x26 => Control::Pane, // WINDOW · CLIENT · PANE · DIALOG · PROPERTYPAGE
        0x0f => Control::Document,                      // DOCUMENT
        0x14 => Control::Group,                         // GROUPING
        0x16 => Control::ToolBar,                       // TOOLBAR
        0x18 => Control::Table,                         // TABLE
        0x19 | 0x1a => Control::Header,                 // COLUMNHEADER · ROWHEADER
        0x1c | 0x1d => Control::DataItem,               // ROW · CELL
        0x1e => Control::Hyperlink,                     // LINK
        0x21 => Control::List,                          // LIST
        0x22 => Control::ListItem,                      // LISTITEM
        0x23 => Control::Tree,                          // OUTLINE
        0x24 => Control::TreeItem,                      // OUTLINEITEM
        0x25 => Control::TabItem,                       // PAGETAB
        0x3c => Control::Tab,                           // PAGETABLIST
        0x28 => Control::Image,                         // GRAPHIC
        0x29 => Control::Text,                          // STATICTEXT
        0x2a => Control::Edit,                          // TEXT(편집 칸)
        0x2b | 0x38 | 0x39 | 0x3e => Control::Button,   // PUSHBUTTON · BUTTONDROPDOWN · BUTTONMENU · SPLITBUTTON
        0x2c | 0x2d => Control::CheckBox,               // CHECKBUTTON · RADIOBUTTON
        0x2e => Control::ComboBox,                      // COMBOBOX
        _ => Control::Other,
    }
}

/// 요소 하나의 사실(사각형은 MSAA가 준 그대로 — 좌표 공간 판정 전).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccInfo {
    pub rect: PhysRect,
    pub role: i32,
    pub state: u32,
}

/// 색인에 남길 요소: 보이고, 화면 안이고, 4 px 이상.
pub fn keep(info: &AccInfo) -> bool {
    info.state & (STATE_INVISIBLE | STATE_OFFSCREEN) == 0 && info.rect.w >= 4 && info.rect.h >= 4
}

/// 자식 하나: 온전한 객체(따로 질의) 또는 부모가 대신 답하는 단순 요소(자식 번호).
pub enum AccChild<N> {
    Object(N),
    Simple(i32),
}

/// MSAA 나무 한 마디(COM이든 시험용 가짜든).
pub trait AccNode: Sized {
    /// `child` = None이면 자기 자신(CHILDID_SELF), Some(n)이면 단순 자식 n.
    fn info(&self, child: Option<i32>) -> Option<AccInfo>;
    /// `start`번째(0부터)부터 많아야 `max`개의 자식. 더 없으면 빈 목록.
    fn children(&self, start: usize, max: usize) -> Vec<AccChild<Self>>;
}

/// 걸음 상한.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub depth: usize,
    pub max: usize,
    pub per_node: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { depth: MAX_DEPTH, max: MAX_ELEMENTS, per_node: MAX_CHILDREN }
    }
}

/// 뿌리 아래를 넓이 우선으로 걷는다(뿌리 자신은 빼고 — UIA `Descendants`와 같게). 큰 그릇이 먼저 모여서
/// 예산에서 끊겨도 쓸 만한 것이 남는다. `out_of_time()`이 참이면 그 자리에서 멈춘다.
pub fn walk<N: AccNode>(root: N, limits: &Limits, mut out_of_time: impl FnMut() -> bool) -> Vec<AccInfo> {
    let mut out = Vec::new();
    let mut queue: VecDeque<(N, usize)> = VecDeque::new();
    queue.push_back((root, 0));
    while let Some((node, depth)) = queue.pop_front() {
        if depth >= limits.depth {
            continue;
        }
        let mut start = 0;
        while start < limits.per_node {
            if out.len() >= limits.max || out_of_time() {
                return out;
            }
            let chunk = node.children(start, CHILD_CHUNK.min(limits.per_node - start));
            if chunk.is_empty() {
                break;
            }
            start += chunk.len();
            for child in chunk {
                if out.len() >= limits.max || out_of_time() {
                    return out;
                }
                match child {
                    AccChild::Simple(id) => {
                        if let Some(i) = node.info(Some(id)) {
                            out.push(i);
                        }
                    }
                    AccChild::Object(n) => {
                        if let Some(i) = n.info(None) {
                            out.push(i);
                            // 보이지 않는 그릇 아래는 걷지 않는다(숨은 메뉴·창 틀 조각)
                            if i.state & STATE_INVISIBLE == 0 {
                                queue.push_back((n, depth + 1));
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// 걸은 결과를 색인 요소로: 거르고(보임·크기), 창과 겹치지 않는 쓰레기 빼고, 같은 사각형 하나로.
pub fn to_elements(found: &[AccInfo], window: Option<PhysRect>) -> Vec<ink_domain::recog::ElementCand> {
    let mut seen: HashSet<(PhysRect, Control)> = HashSet::new();
    found
        .iter()
        .filter(|i| keep(i))
        .filter(|i| window.map_or(true, |w| i.rect.intersect(&w).is_some()))
        .map(|i| ink_domain::recog::ElementCand { rect: i.rect, control: control_from_role(i.role), from_msaa: true })
        .filter(|e| seen.insert((e.rect, e.control)))
        .collect()
}

/// 묻기 전 문지기(순수 상태): 마감을 넘긴 창·실행 파일과 버린 일꾼 수.
#[derive(Debug, Default)]
pub struct Gate {
    bad_windows: HashSet<u64>,
    bad_exes: HashSet<String>,
    abandoned: usize,
}

impl Gate {
    pub fn allows(&self, hwnd: u64, exe: &str) -> bool {
        self.abandoned < MAX_ABANDONED && !self.bad_windows.contains(&hwnd) && (exe.is_empty() || !self.bad_exes.contains(&exe.to_ascii_lowercase()))
    }

    /// 마감을 넘겼다 — 그 창·앱은 다시 묻지 않고, 버린 일꾼을 센다.
    pub fn timed_out(&mut self, hwnd: u64, exe: &str) {
        self.bad_windows.insert(hwnd);
        if !exe.is_empty() {
            self.bad_exes.insert(exe.to_ascii_lowercase());
        }
        self.abandoned += 1;
    }

    pub fn abandoned(&self) -> usize {
        self.abandoned
    }

    /// 버린 일꾼이 상한에 닿아 MSAA를 껐는지.
    pub fn disabled(&self) -> bool {
        self.abandoned >= MAX_ABANDONED
    }
}

#[cfg(windows)]
mod win {
    use super::{to_elements, walk, AccChild, AccInfo, AccNode, Gate, Limits, DEADLINE, WALK_BUDGET};
    use crate::dpi::{self, CoordSpace, WindowDpi};
    use ink_domain::geometry::PhysRect;
    use ink_domain::recog::ElementCand;
    use std::sync::mpsc;
    use std::time::Instant;
    use windows::core::Interface;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CoInitializeEx, IDispatch, COINIT_MULTITHREADED};
    use windows::Win32::System::Variant::{VARIANT, VT_DISPATCH, VT_I4};
    use windows::Win32::UI::Accessibility::{AccessibleChildren, AccessibleObjectFromWindow, IAccessible};

    /// `OBJID_CLIENT`(-4)
    const OBJID_CLIENT: i32 = -4;
    /// `CHILDID_SELF`
    const CHILDID_SELF: i32 = 0;

    struct ComNode(IAccessible);

    impl AccNode for ComNode {
        fn info(&self, child: Option<i32>) -> Option<AccInfo> {
            let var = VARIANT::from(child.unwrap_or(CHILDID_SELF));
            let (mut l, mut t, mut w, mut h) = (0i32, 0i32, 0i32, 0i32);
            // SAFETY: COM 메서드 호출, 출력 정수 네 개. 실패하면 이 요소만 버린다.
            unsafe { self.0.accLocation(&mut l, &mut t, &mut w, &mut h, &var) }.ok()?;
            // 역할이 문자열(사용자 정의)이면 0 → Other
            let role = unsafe { self.0.get_accRole(&var) }.ok().and_then(|v| i32::try_from(&v).ok()).unwrap_or(0);
            let state = unsafe { self.0.get_accState(&var) }.ok().and_then(|v| i32::try_from(&v).ok()).unwrap_or(0) as u32;
            Some(AccInfo { rect: PhysRect::new(l, t, w, h), role, state })
        }

        fn children(&self, start: usize, max: usize) -> Vec<AccChild<Self>> {
            // SAFETY: COM 메서드 호출.
            let total = unsafe { self.0.accChildCount() }.unwrap_or(0).max(0) as usize;
            let n = total.saturating_sub(start).min(max);
            if n == 0 {
                return Vec::new();
            }
            let mut buf = vec![VARIANT::default(); n];
            let mut got = 0i32;
            // SAFETY: 버퍼 길이만큼만 채워진다(받은 수 = got).
            if unsafe { AccessibleChildren(&self.0, start as i32, &mut buf, &mut got) }.is_err() {
                return Vec::new();
            }
            buf.iter()
                .take(got.clamp(0, n as i32) as usize)
                .filter_map(|v| {
                    if v.vt() == VT_I4 {
                        i32::try_from(v).ok().map(AccChild::Simple)
                    } else if v.vt() == VT_DISPATCH {
                        IDispatch::try_from(v).ok().and_then(|d| d.cast::<IAccessible>().ok()).map(|a| AccChild::Object(ComNode(a)))
                    } else {
                        None
                    }
                })
                .collect()
        }
    }

    fn root(hwnd: u64) -> Option<ComNode> {
        let mut p: *mut core::ffi::c_void = std::ptr::null_mut();
        // SAFETY: 성공하면 p는 AddRef된 IAccessible — from_raw가 소유권을 가져간다.
        unsafe {
            AccessibleObjectFromWindow(HWND(hwnd as _), OBJID_CLIENT as u32, &IAccessible::IID, &mut p).ok()?;
            (!p.is_null()).then(|| ComNode(IAccessible::from_raw(p)))
        }
    }

    fn window_rect(hwnd: u64) -> Option<PhysRect> {
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: 출력 구조체 하나, 읽기 전용 질의.
        (unsafe { GetWindowRect(hwnd as _, &mut r) } != 0).then(|| PhysRect::from_edges(r.left, r.top, r.right, r.bottom))
    }

    /// 일꾼 스레드 안: 창 하나를 걷고 물리 px 요소로. 여기서 멈춰도 부르는 쪽은 마감에서 끊는다.
    fn index_window(hwnd: u64) -> Vec<ElementCand> {
        let Some(root) = root(hwnd) else { return Vec::new() };
        let dpi = WindowDpi::of(hwnd);
        // 좌표 공간 판정은 걷기 전에 뿌리 하나로(창마다 한 번)
        let space = if dpi.virtualized() {
            match (root.info(None), dpi::client_rect_physical(hwnd)) {
                (Some(r), Some(phys)) => dpi::classify_space(r.rect, phys, dpi::to_logical_rect(hwnd, phys, &dpi)),
                _ => CoordSpace::Physical,
            }
        } else {
            CoordSpace::Physical
        };
        let start = Instant::now();
        let mut found = walk(root, &Limits::default(), || start.elapsed() >= WALK_BUDGET);
        if space == CoordSpace::Logical {
            for f in found.iter_mut() {
                f.rect = dpi::to_physical_rect(hwnd, f.rect, &dpi);
            }
        }
        to_elements(&found, window_rect(hwnd))
    }

    struct Req {
        hwnd: u64,
        reply: mpsc::Sender<Vec<ElementCand>>,
    }

    fn spawn_worker() -> Option<mpsc::Sender<Req>> {
        let (tx, rx) = mpsc::channel::<Req>();
        std::thread::Builder::new()
            .name("screen-ink-msaa".into())
            .spawn(move || {
                // D11: 질의 스레드는 MTA로 명시 초기화
                // SAFETY: 이 스레드에서 한 번, 끝날 때까지 유지.
                let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
                while let Ok(req) = rx.recv() {
                    let out = index_window(req.hwnd);
                    // 부르는 쪽이 이미 포기했으면 보낼 곳이 없다 — 그대로 다음
                    let _ = req.reply.send(out);
                }
            })
            .ok()?;
        Some(tx)
    }

    /// UIA 일꾼이 쥐는 MSAA 대체 경로. 그 일꾼 스레드에서만 부른다(내부 상태에 잠금 없음).
    #[derive(Default)]
    pub struct Msaa {
        gate: Gate,
        worker: Option<mpsc::Sender<Req>>,
    }

    impl Msaa {
        pub fn new() -> Msaa {
            Msaa::default()
        }

        /// 창 하나를 MSAA로 색인. 응답 없는 창·마감 초과·문지기 거절이면 빈 목록(기다림은 많아야 `DEADLINE` + 50 ms).
        pub fn index(&mut self, hwnd: u64, exe: &str) -> Vec<ElementCand> {
            if !self.gate.allows(hwnd, exe) || !crate::uia::responsive(hwnd) {
                return Vec::new();
            }
            if self.worker.is_none() {
                self.worker = spawn_worker();
            }
            let Some(tx) = self.worker.as_ref() else { return Vec::new() };
            let (rtx, rrx) = mpsc::channel();
            if tx.send(Req { hwnd, reply: rtx }).is_err() {
                self.worker = None;
                return Vec::new();
            }
            match rrx.recv_timeout(DEADLINE) {
                Ok(v) => v,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // 멈춘 일꾼은 버린다(보내는 쪽을 놓으면 풀린 뒤 스스로 끝남). 다음 질의는 새 일꾼.
                    self.worker = None;
                    self.gate.timed_out(hwnd, exe);
                    log::warn!("msaa deadline {} ms exceeded, worker abandoned ({} so far{})", DEADLINE.as_millis(), self.gate.abandoned(), if self.gate.disabled() { ", msaa off" } else { "" });
                    Vec::new()
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    self.worker = None;
                    Vec::new()
                }
            }
        }
    }
}

#[cfg(windows)]
pub use win::Msaa;

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn msaa_역할코드_control_매핑() {
        // C §2.2 매핑 12종 + 덧붙인 것
        let cases = [
            (43, Control::Button),
            (42, Control::Edit),
            (41, Control::Text),
            (40, Control::Image),
            (33, Control::List),
            (34, Control::ListItem),
            (24, Control::Table),
            (28, Control::DataItem),
            (29, Control::DataItem),
            (37, Control::TabItem),
            (12, Control::MenuItem),
            (30, Control::Hyperlink),
            (1, Control::TitleBar),
            (16, Control::Pane),
            (20, Control::Group),
            (44, Control::CheckBox),
            (45, Control::CheckBox),
            (46, Control::ComboBox),
            (35, Control::Tree),
            (36, Control::TreeItem),
            (60, Control::Tab),
            (15, Control::Document),
            (62, Control::Button),
        ];
        for (role, want) in cases {
            assert_eq!(control_from_role(role), want, "role {role}");
        }
        assert_eq!(control_from_role(0), Control::Other, "문자열 역할(사용자 정의)");
        assert_eq!(control_from_role(9999), Control::Other);
    }

    #[test]
    fn msaa_uia가_비었을_때만_대체() {
        assert!(uia_is_empty(None), "UIA 실패");
        assert!(uia_is_empty(Some(0)));
        assert!(uia_is_empty(Some(1)), "뿌리 하나뿐");
        assert!(!uia_is_empty(Some(2)));
        assert!(!uia_is_empty(Some(800)));
    }

    #[test]
    fn msaa_보이지_않거나_작은_요소는_버리고_같은_것은_하나로() {
        let r = PhysRect::new(100, 100, 80, 24);
        let found = [
            AccInfo { rect: r, role: 43, state: 0 },
            AccInfo { rect: r, role: 43, state: 0 }, // 같은 것 두 번(창 객체와 클라이언트 객체)
            AccInfo { rect: PhysRect::new(100, 130, 80, 24), role: 43, state: STATE_INVISIBLE },
            AccInfo { rect: PhysRect::new(100, 160, 80, 24), role: 43, state: STATE_OFFSCREEN },
            AccInfo { rect: PhysRect::new(100, 190, 3, 24), role: 43, state: 0 },
            AccInfo { rect: PhysRect::new(-30000, -30000, 50, 50), role: 43, state: 0 }, // 창 밖 쓰레기
            AccInfo { rect: r, role: 10, state: 0 },                                     // 같은 자리 다른 종류는 남김(융합이 합침)
        ];
        let els = to_elements(&found, Some(PhysRect::new(0, 0, 800, 600)));
        assert_eq!(els.len(), 2, "{els:?}");
        assert_eq!(els[0].control, Control::Button);
        assert_eq!(els[1].control, Control::Pane);
        assert!(els.iter().all(|e| e.from_msaa), "MSAA 요소는 근거 M으로 표시");
    }

    #[test]
    fn msaa_문지기_마감_넘긴_창과_앱은_다시_안_묻고_일꾼_4개면_끈다() {
        let mut g = Gate::default();
        assert!(g.allows(1, "old.exe"));
        g.timed_out(1, "Old.EXE");
        assert!(!g.allows(1, "other.exe"), "같은 창");
        assert!(!g.allows(2, "old.exe"), "같은 앱 다른 창(대소문자 무시)");
        assert!(g.allows(3, "fine.exe"));
        assert!(g.allows(4, ""), "실행 파일을 모르면 창으로만 판단");
        for h in 10..13 {
            g.timed_out(h, "");
        }
        assert_eq!(g.abandoned(), MAX_ABANDONED);
        assert!(g.disabled());
        assert!(!g.allows(99, "fresh.exe"), "버린 일꾼 상한 — MSAA 끔");
    }

    /// 시험용 나무: 마디마다 `fan`개 자식, 끝없이 깊다(RB-20 "자식을 끝없이 보고").
    #[derive(Clone)]
    struct Endless {
        depth: usize,
        fan: usize,
        simple: bool,
        calls: Rc<Cell<usize>>,
    }

    impl AccNode for Endless {
        fn info(&self, child: Option<i32>) -> Option<AccInfo> {
            self.calls.set(self.calls.get() + 1);
            let i = child.unwrap_or(0);
            Some(AccInfo { rect: PhysRect::new(10 + i, 10 + self.depth as i32, 40, 20), role: if self.simple { 34 } else { 16 }, state: 0 })
        }

        fn children(&self, start: usize, max: usize) -> Vec<AccChild<Self>> {
            (start..self.fan.min(start + max))
                .map(|i| if self.simple { AccChild::Simple(i as i32 + 1) } else { AccChild::Object(Endless { depth: self.depth + 1, ..self.clone() }) })
                .collect()
        }
    }

    #[test]
    fn msaa_끝없는_자식도_개수_상한에서_끊긴다() {
        let calls = Rc::new(Cell::new(0));
        let root = Endless { depth: 0, fan: 1_000_000, simple: false, calls: calls.clone() };
        let found = walk(root, &Limits::default(), || false);
        assert_eq!(found.len(), MAX_ELEMENTS);
        assert!(calls.get() <= MAX_ELEMENTS + 1, "상한 넘어 묻지 않는다: {}", calls.get());
    }

    #[test]
    fn msaa_깊이_상한() {
        let root = Endless { depth: 0, fan: 2, simple: false, calls: Rc::new(Cell::new(0)) };
        let found = walk(root, &Limits { depth: 3, max: 10_000, per_node: 300 }, || false);
        // 뿌리 빼고 2 + 4 + 8
        assert_eq!(found.len(), 14);
        assert!(found.iter().all(|i| i.rect.y <= 10 + 3));
    }

    #[test]
    fn msaa_단순_자식은_부모가_답한다() {
        let root = Endless { depth: 0, fan: 5, simple: true, calls: Rc::new(Cell::new(0)) };
        let found = walk(root, &Limits::default(), || false);
        assert_eq!(found.len(), 5);
        assert!(found.iter().all(|i| i.role == 34));
        assert_eq!(found[4].rect.x, 15, "자식 번호 5");
    }

    #[test]
    fn msaa_시간_예산이_다하면_그_자리에서_멈춘다() {
        let root = Endless { depth: 0, fan: 50, simple: false, calls: Rc::new(Cell::new(0)) };
        let ticks = Cell::new(0);
        let found = walk(root, &Limits::default(), || {
            ticks.set(ticks.get() + 1);
            ticks.get() > 7
        });
        // 예산은 묶음을 받기 전 1번 + 자식마다 1번 본다 → 7번 허락이면 자식 6개에서 멈춤
        assert_eq!(found.len(), 6);
    }

    #[test]
    fn msaa_자식은_묶음으로_받고_묶음_사이마다_시간을_본다() {
        // 한 마디의 자식 300개를 한 번에 받지 않는다 — 예산이 첫 묶음 뒤에 다하면 그 묶음까지만
        let fetched = Rc::new(Cell::new(0));
        struct Counting(Rc<Cell<usize>>);
        impl AccNode for Counting {
            fn info(&self, c: Option<i32>) -> Option<AccInfo> {
                Some(AccInfo { rect: PhysRect::new(c.unwrap_or(0), 0, 10, 10), role: 34, state: 0 })
            }
            fn children(&self, start: usize, max: usize) -> Vec<AccChild<Self>> {
                let v: Vec<_> = (start..(start + max).min(1000)).map(|i| AccChild::Simple(i as i32 + 1)).collect();
                self.0.set(self.0.get() + v.len());
                v
            }
        }
        let out = Cell::new(false);
        let found = walk(Counting(fetched.clone()), &Limits::default(), || {
            let o = out.get();
            out.set(fetched.get() >= CHILD_CHUNK);
            o
        });
        assert_eq!(fetched.get(), CHILD_CHUNK, "첫 묶음만 받음");
        assert!(found.len() <= CHILD_CHUNK);
        // 예산이 넉넉하면 마디당 상한(300)까지 이어 받는다
        let fetched = Rc::new(Cell::new(0));
        let all = walk(Counting(fetched.clone()), &Limits::default(), || false);
        assert_eq!(all.len(), MAX_CHILDREN);
        assert_eq!(all[MAX_CHILDREN - 1].rect.x, MAX_CHILDREN as i32, "자식 번호가 이어진다");
    }

    #[test]
    fn msaa_보이지_않는_그릇_아래는_걷지_않는다() {
        struct Hidden;
        impl AccNode for Hidden {
            fn info(&self, _c: Option<i32>) -> Option<AccInfo> {
                Some(AccInfo { rect: PhysRect::new(0, 0, 50, 50), role: 11, state: STATE_INVISIBLE })
            }
            fn children(&self, start: usize, _m: usize) -> Vec<AccChild<Self>> {
                if start == 0 {
                    vec![AccChild::Object(Hidden), AccChild::Object(Hidden)]
                } else {
                    Vec::new()
                }
            }
        }
        let found = walk(Hidden, &Limits::default(), || false);
        assert_eq!(found.len(), 2, "뿌리의 자식 둘만 — 숨은 메뉴 아래로 내려가지 않음");
    }
}
