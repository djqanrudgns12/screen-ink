//! RB-04 Win32 표준 컨트롤 시험 앱(08 §5.1, C §2.3 `3f-docs`).
//!
//! 단추·확인란·입력 칸·글자·묶음·라디오·목록(ListView)·트리(TreeView)·탭(Tab)을 띄우고, 컨트롤마다 **물리 px** 정답
//! 사각형과 정답 점을 JSON으로 쓴다(사례 `Case.points`와 같은 모양). 내용은 모두 지어낸 것(개인정보 없음).
//!
//!   rb04-win32 [--dpi pmv2|system|unaware] [--out rb04-truth.json]
//!   rb04-win32 merge <사례.json> <rb04-truth.json> [메모]      ← 사례의 points를 정답으로 바꿈
//!
//! - `--dpi`: 이 앱의 DPI 인식(기본 pmv2). `unaware`·`system`은 D12(DPI 모르는 창 좌표 보정) 시험용 —
//!   정답은 언제나 물리 px로 쓴다(스레드 DPI 문맥을 잠깐 Per-Monitor V2로 바꿔 읽음).
//! - 정답 파일은 창을 띄운 뒤 0.8초, 창을 옮기거나 크기를 바꿀 때마다, 배율이 다른 모니터로 갈 때마다 다시 쓴다.
//!   사례를 뜨기 직전에 창을 움직이지 않으면 최신이다.

#[cfg(not(windows))]
fn main() {
    if std::env::args().nth(1).as_deref() == Some("merge") {
        std::process::exit(merge::run(std::env::args().skip(2).collect()));
    }
    eprintln!("Windows 전용 시험 앱이에요(merge만 어디서나 돼요)");
    std::process::exit(2);
}

#[cfg(windows)]
fn main() {
    if std::env::args().nth(1).as_deref() == Some("merge") {
        std::process::exit(merge::run(std::env::args().skip(2).collect()));
    }
    win::run();
}

mod merge {
    /// 사례 JSON의 `points`(와 `note`)를 정답 파일의 것으로 바꾼다. 나머지(창 목록·색인·PNG 이름)는 그대로.
    pub fn run(args: Vec<String>) -> i32 {
        if args.len() < 2 {
            eprintln!("쓰는 법: rb04-win32 merge <사례.json> <rb04-truth.json> [메모]");
            return 2;
        }
        let read = |p: &str| -> Result<serde_json::Value, String> { std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}")).and_then(|t| serde_json::from_str(&t).map_err(|e| format!("{p}: {e}"))) };
        let result = (|| -> Result<usize, String> {
            let mut case = read(&args[0])?;
            let truth = read(&args[1])?;
            let points = truth.get("points").cloned().ok_or("정답 파일에 points가 없어요")?;
            let n = points.as_array().map_or(0, |a| a.len());
            let obj = case.as_object_mut().ok_or("사례가 JSON 객체가 아니에요")?;
            obj.insert("points".into(), points);
            if let Some(note) = args.get(2) {
                obj.insert("note".into(), serde_json::Value::String(note.clone()));
            }
            let text = serde_json::to_string_pretty(&case).map_err(|e| e.to_string())?;
            std::fs::write(&args[0], text).map_err(|e| e.to_string())?;
            Ok(n)
        })();
        match result {
            Ok(n) => {
                println!("{}: 정답 점 {n}개", args[0]);
                0
            }
            Err(e) => {
                eprintln!("{e}");
                1
            }
        }
    }
}

#[cfg(windows)]
mod win {
    use serde_json::json;
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, CreateFontW, DeleteObject, HFONT};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Controls::{
        InitCommonControlsEx, ICC_LISTVIEW_CLASSES, ICC_STANDARD_CLASSES, ICC_TAB_CLASSES, ICC_TREEVIEW_CLASSES, INITCOMMONCONTROLSEX, LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_TEXT, LVIR_BOUNDS,
        LVITEMW, LVM_GETITEMRECT, LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETITEMTEXTW, LVS_REPORT, TCIF_TEXT, TCITEMW, TCM_GETITEMRECT, TCM_INSERTITEMW, TVE_EXPAND, TVIF_TEXT, TVINSERTSTRUCTW,
        TVINSERTSTRUCTW_0, TVITEMW, TVI_LAST, TVI_ROOT, TVM_EXPAND, TVM_GETITEMRECT, TVM_INSERTITEMW, TVS_HASBUTTONS, TVS_HASLINES, TVS_LINESATROOT,
    };
    use windows_sys::Win32::UI::HiDpi::{
        GetDpiForWindow, SetProcessDpiAwarenessContext, SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
        DPI_AWARENESS_CONTEXT_UNAWARE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW, GetWindowRect, KillTimer, MoveWindow, PostQuitMessage, RegisterClassW, SendMessageW, SetTimer,
        SetWindowPos, ShowWindow, TranslateMessage, BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON, BS_GROUPBOX, BS_PUSHBUTTON, CW_USEDEFAULT, ES_AUTOHSCROLL, ES_MULTILINE, MSG, SWP_NOMOVE,
        SWP_NOZORDER, SW_SHOW, WM_DESTROY, WM_DPICHANGED, WM_EXITSIZEMOVE, WM_SETFONT, WM_TIMER, WNDCLASSW, WS_BORDER, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_OVERLAPPEDWINDOW,
        WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
    };

    const TIMER_TRUTH: usize = 1;
    /// `SS_LEFT`(windows-sys에서는 SystemServices 기능에 있어 값으로)
    const SS_LEFT: u32 = 0;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// 컨트롤 하나: 정답 이름, 창 종류, 글자, 스타일, 96 DPI 기준 배치.
    struct Spec {
        label: &'static str,
        class: &'static str,
        text: &'static str,
        style: u32,
        rect: [i32; 4],
    }

    const SPECS: &[Spec] = &[
        Spec { label: "단추 확인", class: "BUTTON", text: "확인", style: BS_PUSHBUTTON as u32 | WS_TABSTOP, rect: [24, 24, 120, 32] },
        Spec { label: "단추 취소", class: "BUTTON", text: "취소", style: BS_PUSHBUTTON as u32 | WS_TABSTOP, rect: [156, 24, 120, 32] },
        Spec { label: "확인란", class: "BUTTON", text: "보기", style: BS_AUTOCHECKBOX as u32 | WS_TABSTOP, rect: [288, 28, 120, 24] },
        Spec { label: "입력 칸", class: "EDIT", text: "예시 글자", style: ES_AUTOHSCROLL as u32 | WS_BORDER | WS_TABSTOP, rect: [24, 72, 384, 28] },
        Spec { label: "안내 글", class: "STATIC", text: "아래 목록에서 하나를 고르세요(예시)", style: SS_LEFT, rect: [24, 112, 384, 24] },
        Spec { label: "묶음", class: "BUTTON", text: "묶음", style: BS_GROUPBOX as u32, rect: [432, 24, 440, 120] },
        Spec { label: "라디오 가", class: "BUTTON", text: "가", style: BS_AUTORADIOBUTTON as u32, rect: [448, 52, 160, 24] },
        Spec { label: "라디오 나", class: "BUTTON", text: "나", style: BS_AUTORADIOBUTTON as u32, rect: [448, 84, 160, 24] },
        Spec { label: "목록", class: "SysListView32", text: "", style: LVS_REPORT | WS_BORDER, rect: [24, 160, 400, 200] },
        Spec { label: "트리", class: "SysTreeView32", text: "", style: TVS_HASLINES | TVS_LINESATROOT | TVS_HASBUTTONS | WS_BORDER, rect: [448, 160, 424, 200] },
        Spec { label: "탭", class: "SysTabControl32", text: "", style: 0, rect: [24, 384, 848, 220] },
        Spec { label: "여러 줄 입력", class: "EDIT", text: "첫 줄\r\n둘째 줄", style: ES_MULTILINE as u32 | WS_VSCROLL | WS_BORDER, rect: [48, 432, 380, 150] },
        Spec { label: "단추 적용", class: "BUTTON", text: "적용", style: BS_PUSHBUTTON as u32 | WS_TABSTOP, rect: [452, 432, 120, 32] },
    ];

    struct App {
        main: HWND,
        ctrls: Vec<HWND>,
        font: HFONT,
        tree_root: isize,
        out: String,
        dpi_mode: String,
    }

    thread_local! {
        static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    }

    fn scale(h: HWND) -> f64 {
        // SAFETY: 창 핸들 질의.
        let d = unsafe { GetDpiForWindow(h) };
        if d == 0 {
            1.0
        } else {
            d as f64 / 96.0
        }
    }

    fn sc(v: i32, s: f64) -> i32 {
        (v as f64 * s).round() as i32
    }

    /// 배율에 맞춰 컨트롤 자리·글꼴을 다시(배율이 다른 모니터로 옮길 때).
    fn relayout(app: &mut App) {
        let s = scale(app.main);
        // SAFETY: 이 앱이 만든 창·글꼴.
        unsafe {
            if !app.font.is_null() {
                DeleteObject(app.font);
            }
            let face = wide("Malgun Gothic");
            app.font = CreateFontW(-sc(13, s), 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());
            for (spec, &h) in SPECS.iter().zip(&app.ctrls) {
                let [x, y, w, hh] = spec.rect;
                MoveWindow(h, sc(x, s), sc(y, s), sc(w, s), sc(hh, s), 1);
                SendMessageW(h, WM_SETFONT, app.font as usize, 1);
            }
        }
    }

    /// 스레드 DPI 문맥을 잠깐 Per-Monitor V2로 — 이 앱이 DPI를 몰라도 물리 px를 읽는다.
    fn physical<T>(f: impl FnOnce() -> T) -> T {
        // SAFETY: 이 스레드의 문맥만 바꾸고 되돌린다.
        unsafe {
            let old: DPI_AWARENESS_CONTEXT = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let r = f();
            if !old.is_null() {
                SetThreadDpiAwarenessContext(old);
            }
            r
        }
    }

    #[derive(Clone, Copy)]
    struct R {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    }

    impl R {
        fn from(r: RECT) -> R {
            R { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top }
        }
        fn center(&self) -> (i32, i32) {
            (self.x + self.w / 2, self.y + self.h / 2)
        }
        fn at(&self, fx: f64, fy: f64) -> (i32, i32) {
            (self.x + (self.w as f64 * fx) as i32, self.y + (self.h as f64 * fy) as i32)
        }
    }

    fn window_rect_phys(h: HWND) -> R {
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: 출력 하나.
        physical(|| unsafe { GetWindowRect(h, &mut r) });
        R::from(r)
    }

    /// 컨트롤 클라이언트 좌표(그 컨트롤 자신의 DPI 문맥 = 논리)의 사각형 → 화면 물리 px.
    fn client_to_phys(h: HWND, rc: RECT) -> R {
        let mut logical = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let mut phys = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let mut origin = POINT { x: 0, y: 0 };
        // SAFETY: 출력 구조체만.
        unsafe { GetClientRect(h, &mut logical) };
        physical(|| unsafe {
            GetClientRect(h, &mut phys);
            ClientToScreen(h, &mut origin);
        });
        let rx = if logical.right > 0 { phys.right as f64 / logical.right as f64 } else { 1.0 };
        let ry = if logical.bottom > 0 { phys.bottom as f64 / logical.bottom as f64 } else { 1.0 };
        let fx = |v: i32| origin.x + (v as f64 * rx).round() as i32;
        let fy = |v: i32| origin.y + (v as f64 * ry).round() as i32;
        R::from(RECT { left: fx(rc.left), top: fy(rc.top), right: fx(rc.right), bottom: fy(rc.bottom) })
    }

    fn frame_phys(h: HWND) -> R {
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: RECT 크기를 넘긴다. DWM 확장 프레임 경계는 DPI 가상화되지 않는다(물리 px).
        let hr = unsafe { DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS as u32, &mut r as *mut _ as *mut c_void, std::mem::size_of::<RECT>() as u32) };
        if hr == 0 {
            R::from(r)
        } else {
            window_rect_phys(h)
        }
    }

    fn point(label: &str, at: (i32, i32), truth: R) -> serde_json::Value {
        json!({ "at": { "x": at.0, "y": at.1 }, "truth": { "x": truth.x, "y": truth.y, "w": truth.w, "h": truth.h }, "label": label })
    }

    fn write_truth(app: &App) {
        let mut points = Vec::new();
        let by = |label: &str| SPECS.iter().position(|s| s.label == label).map(|i| app.ctrls[i]);
        for (spec, &h) in SPECS.iter().zip(&app.ctrls) {
            let r = window_rect_phys(h);
            let at = match spec.label {
                // 그릇은 빈 자리(라디오·항목·안쪽 컨트롤을 피해)
                "묶음" => r.at(0.85, 0.7),
                "목록" => r.at(0.5, 0.9),
                "트리" => r.at(0.85, 0.85),
                "탭" => r.at(0.85, 0.8),
                _ => r.center(),
            };
            points.push(point(spec.label, at, r));
        }
        // 목록 첫 행 · 트리 뿌리 항목 · 첫 탭 단추
        if let Some(lv) = by("목록") {
            let mut rc = RECT { left: LVIR_BOUNDS as i32, top: 0, right: 0, bottom: 0 };
            // SAFETY: LVM_GETITEMRECT는 lParam RECT를 채운다(left = LVIR_BOUNDS).
            if unsafe { SendMessageW(lv, LVM_GETITEMRECT, 0, &mut rc as *mut _ as LPARAM) } != 0 {
                let r = client_to_phys(lv, rc);
                points.push(point("목록 첫 행", r.center(), r));
            }
        }
        if let Some(tv) = by("트리") {
            let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            // SAFETY: TVM_GETITEMRECT는 RECT 앞자리에 항목 핸들을 받아 그 자리에 사각형을 쓴다(wParam 1 = 글자 부분).
            unsafe { *(&mut rc as *mut RECT as *mut isize) = app.tree_root };
            if unsafe { SendMessageW(tv, TVM_GETITEMRECT, 1, &mut rc as *mut _ as LPARAM) } != 0 {
                let r = client_to_phys(tv, rc);
                points.push(point("트리 뿌리 항목", r.center(), r));
            }
        }
        if let Some(tab) = by("탭") {
            let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            // SAFETY: TCM_GETITEMRECT는 lParam RECT를 채운다.
            if unsafe { SendMessageW(tab, TCM_GETITEMRECT, 0, &mut rc as *mut _ as LPARAM) } != 0 {
                let r = client_to_phys(tab, rc);
                points.push(point("첫 탭", r.center(), r));
            }
        }
        // 제목 표시줄 위 = 창 전체(D6) · 컨트롤 없는 빈 곳 = 창 전체
        let frame = frame_phys(app.main);
        let s = scale(app.main);
        points.push(point("제목 표시줄", (frame.x + frame.w / 2, frame.y + sc(12, s)), frame));
        let client = window_rect_phys(app.main);
        points.push(point("창 빈 곳", (frame.x + frame.w / 2, client.y + client.h - sc(24, s)), frame));

        let doc = json!({ "tool": "rb04-win32", "dpi": app.dpi_mode, "windowScale": s, "points": points });
        match std::fs::write(&app.out, serde_json::to_string_pretty(&doc).unwrap_or_default()) {
            Ok(()) => println!("정답 {}개 → {}", doc["points"].as_array().map_or(0, |a| a.len()), app.out),
            Err(e) => eprintln!("정답을 쓰지 못했어요: {e}"),
        }
    }

    fn with_app(f: impl FnOnce(&mut App)) {
        APP.with(|a| {
            if let Ok(mut g) = a.try_borrow_mut() {
                if let Some(app) = g.as_mut() {
                    f(app);
                }
            }
        });
    }

    unsafe extern "system" fn proc(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        match msg {
            WM_TIMER if w == TIMER_TRUTH => {
                KillTimer(h, TIMER_TRUTH);
                with_app(|app| write_truth(app));
                0
            }
            WM_EXITSIZEMOVE => {
                SetTimer(h, TIMER_TRUTH, 300, None);
                0
            }
            WM_DPICHANGED => {
                // 배율이 다른 모니터: OS가 제안한 사각형으로 옮기고 다시 배치
                let r = &*(l as *const RECT);
                SetWindowPos(h, null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER);
                with_app(relayout);
                SetTimer(h, TIMER_TRUTH, 300, None);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(h, msg, w, l),
        }
    }

    fn create_child(parent: HWND, spec: &Spec) -> HWND {
        let class = wide(spec.class);
        let text = wide(spec.text);
        // SAFETY: 자식 창, 위치는 relayout이 정한다.
        unsafe { CreateWindowExW(0, class.as_ptr(), text.as_ptr(), WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | spec.style, 0, 0, 10, 10, parent, null_mut(), GetModuleHandleW(null()), null()) }
    }

    fn fill(app: &mut App) {
        let by = |label: &str| SPECS.iter().position(|s| s.label == label).map(|i| app.ctrls[i]);
        let s = scale(app.main);
        // SAFETY: 이 앱이 만든 컨트롤에 표준 메시지. 글자 버퍼는 호출 동안 산다.
        unsafe {
            if let Some(lv) = by("목록") {
                for (i, name) in ["모둠", "관찰"].iter().enumerate() {
                    let t = wide(name);
                    let col = LVCOLUMNW { mask: LVCF_TEXT | LVCF_WIDTH, cx: sc(180, s), pszText: t.as_ptr() as *mut u16, ..std::mem::zeroed() };
                    SendMessageW(lv, LVM_INSERTCOLUMNW, i, &col as *const _ as LPARAM);
                }
                for (i, (a, b)) in [("1모둠", "증발"), ("2모둠", "응결"), ("3모둠", "강수"), ("4모둠", "흐름"), ("5모둠", "모임")].iter().enumerate() {
                    let ta = wide(a);
                    let tb = wide(b);
                    let item = LVITEMW { mask: LVIF_TEXT, iItem: i as i32, iSubItem: 0, pszText: ta.as_ptr() as *mut u16, ..std::mem::zeroed() };
                    SendMessageW(lv, LVM_INSERTITEMW, 0, &item as *const _ as LPARAM);
                    let sub = LVITEMW { mask: LVIF_TEXT, iItem: i as i32, iSubItem: 1, pszText: tb.as_ptr() as *mut u16, ..std::mem::zeroed() };
                    SendMessageW(lv, LVM_SETITEMTEXTW, i, &sub as *const _ as LPARAM);
                }
            }
            if let Some(tv) = by("트리") {
                let insert = |parent: isize, text: &str| -> isize {
                    let t = wide(text);
                    let ins = TVINSERTSTRUCTW {
                        hParent: parent,
                        hInsertAfter: TVI_LAST,
                        Anonymous: TVINSERTSTRUCTW_0 { item: TVITEMW { mask: TVIF_TEXT, pszText: t.as_ptr() as *mut u16, ..std::mem::zeroed() } },
                    };
                    SendMessageW(tv, TVM_INSERTITEMW, 0, &ins as *const _ as LPARAM)
                };
                let root = insert(TVI_ROOT, "물의 순환");
                for c in ["증발", "응결", "강수"] {
                    insert(root, c);
                }
                SendMessageW(tv, TVM_EXPAND, TVE_EXPAND as usize, root);
                app.tree_root = root;
            }
            if let Some(tab) = by("탭") {
                for (i, name) in ["첫째 쪽", "둘째 쪽", "셋째 쪽"].iter().enumerate() {
                    let t = wide(name);
                    let item = TCITEMW { mask: TCIF_TEXT, pszText: t.as_ptr() as *mut u16, ..std::mem::zeroed() };
                    SendMessageW(tab, TCM_INSERTITEMW, i, &item as *const _ as LPARAM);
                }
            }
        }
    }

    pub fn run() {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let arg = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
        let dpi_mode = arg("--dpi").unwrap_or_else(|| "pmv2".into());
        let out = arg("--out").unwrap_or_else(|| "rb04-truth.json".into());
        let ctx = match dpi_mode.as_str() {
            "unaware" => DPI_AWARENESS_CONTEXT_UNAWARE,
            "system" => DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
            _ => DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        // SAFETY: 창을 만들기 전 한 번씩. 등록 구조체·문자열은 호출 동안 산다.
        unsafe {
            SetProcessDpiAwarenessContext(ctx);
            let icc = INITCOMMONCONTROLSEX { dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32, dwICC: ICC_LISTVIEW_CLASSES | ICC_TREEVIEW_CLASSES | ICC_TAB_CLASSES | ICC_STANDARD_CLASSES };
            InitCommonControlsEx(&icc);
            let class = wide("ScreenInkRb04");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: GetModuleHandleW(null()),
                // COLOR_BTNFACE + 1 — 대화 상자 바탕
                hbrBackground: 16 as _,
                lpszClassName: class.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
            let title = wide("RB-04");
            let main = CreateWindowExW(0, class.as_ptr(), title.as_ptr(), WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN, CW_USEDEFAULT, CW_USEDEFAULT, 940, 720, null_mut(), null_mut(), GetModuleHandleW(null()), null());
            if main.is_null() {
                eprintln!("창을 만들지 못했어요");
                std::process::exit(1);
            }
            let s = scale(main);
            SetWindowPos(main, null_mut(), 0, 0, sc(940, s), sc(720, s), SWP_NOMOVE | SWP_NOZORDER);
            let ctrls = SPECS.iter().map(|spec| create_child(main, spec)).collect();
            let mut app = App { main, ctrls, font: null_mut(), tree_root: 0, out, dpi_mode };
            relayout(&mut app);
            fill(&mut app);
            APP.with(|a| *a.borrow_mut() = Some(app));
            ShowWindow(main, SW_SHOW);
            SetTimer(main, TIMER_TRUTH, 800, None);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
