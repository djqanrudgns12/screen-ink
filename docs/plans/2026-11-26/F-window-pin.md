# F. 단계 6.5 창 항상 위 고정 구현 시안 (`6.5.1`~`6.5.3`, P1 · Windows 전용)

대상 항목(`progress.json`): `6.5.1` `WindowPinner` · `6.5.2` 테두리 창·따라가기 · `6.5.3` 정리·모두 풀기.
플랫폼 범위: **Windows 전용** (macOS는 공개 API 없음 → 설정·트레이에서 줄 숨김, 어댑터 `Unsupported`, `PRD §4.12`).

---

## 1. 목표 · 범위 (요구사항 ID)

| 요구사항 | 우선 | 요약 |
|---|---|---|
| `R-TOP-1` | P1 | `Alt+Shift+D` 한 번 = 지금 앞 창 고정(`HWND_TOPMOST` + `SetProp`), 한 번 더 = 풀기. 셸·자기 창·숨김·최소화·cloaked 건너뛰기, **원래 맨 위인 창은 건드리지 않음** |
| `R-TOP-2` | P1 | 고정 창 둘레 주황 테두리(기본 4 px, `topmost.border`: 0·2·4·6, 배지 없음, 클릭 통과 + `WDA_EXCLUDEFROMCAPTURE`) + `SetWinEventHook(WINEVENT_OUTOFCONTEXT)`로 16 ms 안에 이동·크기·최소화·숨김·닫힘 따라가기 |
| `R-TOP-3` | P1 | 관리자 권한 창(UIPI) 실패 안내, 창 닫힘 자동 제거, 앱 종료 시 모두 풀기, **비정상 종료 뒤 다음 시작 시 `EnumWindows`로 남은 표시 창 정리**, 전체 화면 D3D 게임 중 키 무시, 트레이 `"고정한 창 모두 풀기 (n)"` |
| `R-TOP-4` | P1 | PowerToys가 고정한 창(`AlwaysOnTop_Pinned` 속성)은 건드리지 않고 `"PowerToys가 고정한 창이에요"` 안내 |

---

## 2. 정확한 형식

### 2.1 창 표시(`SetPropW`) 상수 · 도메인 판정 모델

```rust
// crates/ink-domain/src/topmost.rs (신규, 순수 판정)
pub const PROP_SCREEN_INK: &str = "ScreenInk_TopmostPinned";
pub const PROP_POWERTOYS: &str = "AlwaysOnTop_Pinned";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateWindow {
    pub handle: u64,
    pub pid: u32,
    pub exe: String,             // 소문자, 경로 없음(화면 알림에는 확장자 뗀 이름만)
    pub class: String,           // Shell_TrayWnd, Progman, WorkerW 등 판정
    pub visible: bool,
    pub minimized: bool,
    pub cloaked: bool,
    pub topmost: bool,
    pub has_our_prop: bool,      // GetPropW(PROP_SCREEN_INK) != 0
    pub has_powertoys_prop: bool,// GetPropW(PROP_POWERTOYS) != 0
    pub fullscreen_d3d_game: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TopmostDecision {
    Pin,
    Unpin,
    IgnoreSilent,                // 전체 화면 D3D 게임 중
    Skip(&'static str),          // 섬 한 줄 이유
}

/// 02 §9 규칙을 순수 함수로 판정.
pub fn decide(win: &CandidateWindow, own_pid: u32) -> TopmostDecision;
```

**`decide` 판정 순서 (`02 §9`)**:
1. `fullscreen_d3d_game` → `IgnoreSilent`
2. `pid == own_pid` 또는 셸 클래스(`Progman`·`WorkerW`·`Shell_TrayWnd`·`Shell_SecondaryTrayWnd`·`Windows.UI.Core.CoreWindow` 중 시작 메뉴) 또는 `!visible || minimized || cloaked` → `Skip("이 창은 고정할 수 없어요")`
3. `has_our_prop` → `Unpin`
4. `has_powertoys_prop` → `Skip("PowerToys가 고정한 창이에요")`
5. `topmost` (표시 없이 원래 맨 위) → `Skip("이 창은 원래 맨 위에 있어요")`
6. 그 밖 → `Pin`

### 2.2 포트 (`WindowPinner`) · 모듈 · 파일 위치

```rust
// crates/ink-app/src/ports.rs 추가
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinError {
    AccessDenied, // UIPI — 관리자 권한 창
    Gone,         // 그 사이 창이 닫힘
    Unsupported,  // macOS 등
    Other(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderStyle {
    pub thickness_px: u32, // 0(끔) | 2 | 4 | 6 (논리 px, 어댑터가 모니터 scale 곱함)
    pub color_rgb: u32,    // 0xFF9F0A (주황)
}

pub trait WindowPinner: Send {
    fn foreground(&self) -> Option<ink_domain::topmost::CandidateWindow>;
    fn pin(&mut self, hwnd: u64, border: BorderStyle) -> Result<(), PinError>;
    fn unpin(&mut self, hwnd: u64) -> Result<(), PinError>;
    fn update_border_style(&mut self, border: BorderStyle);
    /// 시작 때 비정상 종료로 남은 PROP_SCREEN_INK 창을 EnumWindows로 찾아 모두 풀고 개수 반환.
    fn cleanup_orphans(&mut self) -> usize;
    fn unpin_all(&mut self);
    fn is_alive(&self, hwnd: u64) -> bool;
}
```

| 파일 위치 | 책임 |
|---|---|
| `crates/ink-domain/src/topmost.rs` | `CandidateWindow`, `TopmostDecision`, `decide()`, 셸 클래스 목록, 앱 표시 이름 정리(`chrome.exe` → `Chrome`) |
| `crates/ink-app/src/topmost.rs` | `TopmostState`(`pinned: BTreeMap<u64, PinnedRecord>` — 핸들·pid·고정 시각만, **창 제목 금지**), `toggle_topmost()`, `unpin_all()`, `on_window_destroyed(hwnd)`, `cleanup_on_start()`, `topmost.border` 설정 변경 반영 |
| `crates/ink-platform-windows/src/topmost.rs` | `WinWindowPinner`: `SetWindowPos(HWND_TOPMOST/NOTOPMOST, SWP_NOMOVE\|SWP_NOSIZE\|SWP_NOACTIVATE)`, `SetPropW`/`RemovePropW`, `SHQueryUserNotificationState` (`QUNS_RUNNING_D3D_FULL_SCREEN` 감지), `EnumWindows` 잔여 표시 정리 |
| `crates/ink-platform-windows/src/topmost_border.rs` | 테두리 전용 네이티브 계층 창(`ScreenInkPinBorder`, 1창당 1개, `WS_EX_LAYERED\|WS_EX_TRANSPARENT\|WS_EX_TOOLWINDOW\|WS_EX_NOACTIVATE\|WS_EX_TOPMOST` + `WDA_EXCLUDEFROMCAPTURE`) + `SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE · DESTROY · HIDE · SHOW · CLOAKED · UNCLOAKED, WINEVENT_OUTOFCONTEXT)` + 1초 폴백 위치 검사 |
| `crates/tauri-plugin-screen-ink/src/lib.rs` | `pin.toggle` / `pin.releaseAll` 연결, 앱 종료(`ExitRequested` / `app.quit`) 시 `unpin_all()`, 시작 시 `cleanup_orphans()` |

### 2.3 화면 상태 · 문구 (`04 S15`, `05 §12`)

- **고정 성공**: 섬 `variant: "pin"` · `"‘Chrome’을 맨 위에 고정했어요"` · `Alt Shift D` 풀기 (1.2초) — `topmost.sound=true`이면 짧은 시스템 효과음 1회.
- **풀기 성공**: `"고정을 풀었어요"` (1.2초).
- **모두 풀기**: `"고정한 창 n개를 풀었어요"` (1.2초).
- **다음 시작 정리**: `n >= 1`이면 `"지난번에 고정한 창 n개를 풀었어요"`.
- **실패·건너뛰기**:
  - 관리자 권한 창: `"관리자 권한 창이라 고정할 수 없어요"`
  - PowerToys 창: `"PowerToys가 고정한 창이에요"`
  - 원래 맨 위 창: `"이 창은 원래 맨 위에 있어요"`
  - 셸/불가 창: `"이 창은 고정할 수 없어요"`
- **트레이**: `InkState.pinned_count`는 이미 `TrayPopoverScreen`과 `tooltip()`에 연결되어 있음(`"고정한 창 2개"`, `[모두 풀기]`).

---

## 3. 알고리즘 · 결정 (대안과 고른 이유)

### 3.1 테두리 창 구현 — 네이티브 GDI/Layered 창 (WebView 대신)

- **채택**: Win32 네이티브 계층 창(`UpdateLayeredWindow`로 가운데가 완전 투명하고 바깥 4 px 주황 둥근 테두리만 그린 32bpp DIB)을 고정 창마다 1개 생성.
  - 이유: WebView2를 띄우면 고정 창 1개당 메모리 +30~50 MB·생성 100 ms가 들지만, 네이티브 계층 창은 **메모리 < 0.5 MB · 위치 갱신 < 0.5 ms**(16 ms 목표 여유 통과).
  - Win11 둥근 모서리 반경은 `DWMWA_WINDOW_CORNER_PREFERENCE`를 읽어 `8px × scale`(또는 `4px × scale`·직각)으로 맞춘다(`04 S15`).
  - Z 순서는 `SetWindowPos(border_hwnd, target_hwnd, ...)`로 대상 창 바로 위(또는 `HWND_TOPMOST` 직상)에 유지하고, `Alt+1`/`F1` 오버레이 진입 시 오버레이가 테두리보다 위로 올라가도록 기존 `show_raw`의 `HWND_TOPMOST` 재올림을 유지한다(`02 §9`).

### 3.2 창 추적 — Out-of-Context `SetWinEventHook` + 1초 안전 폴백

- `WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS`로 DLL 주입 없이 이벤트를 받는다(백신 오탐·보안 경고 0).
- 일부 UWP/최소화 복원 시 `EVENT_OBJECT_LOCATIONCHANGE`가 누락되는 Windows quirks를 막기 위해, 고정 창이 1개 이상일 때만 도는 1초 주기 타이머가 `IsWindow` · `IsIconic` · `DWMWA_CLOAKED` · `DWMWA_EXTENDED_FRAME_BOUNDS`를 재검증한다(`05 §12`).
- 고정 창이 0개가 되면 `UnhookWinEvent`하고 타이머를 정지해 대기 CPU 0%(`M3`) 유지.

---

## 4. 시험 · 합격 기준

| 시험 / 측정 | 이름 예시 | 합격 기준 |
|---|---|---|
| Rust 단위 (`ink-domain::topmost`) | `셸창_자기창_파워토이즈_원래맨위_건너뛰기` | `decide` 6가지 분기 전부 정확 |
| Rust 통합 (`ink-platform-fake`) | `고정_풀기_모두풀기_창닫힘_시작정리_흐름` | `pinned_count` 0→1→2→0 정확, 창 닫힘 시 알림 없이 목록 감소 |
| Windows 실기 (R-TOP-1) | Chrome·한글·PPT·탐색기·카카오톡 고정/풀기 100회 | 오류 0회, 다른 창 클릭해도 위 유지 |
| Windows 실기 (R-TOP-2) | 고정 창 끌기·최대화 + F1 캡처 | 테두리 추적 **≤ 16 ms**, F1 캡처 결과에 주황 테두리 **0 px** |
| Windows 실기 (R-TOP-3) | 창 고정 후 작업 관리자 강제 종료 → 재실행 | 남은 맨 위 창 **0개** + 정리 섬 표시 |
| Windows 실기 (R-TOP-4) | PowerToys 항상 위 켠 상태에서 고정된 창에 `Alt+Shift+D` | 상태 엇갈림 0, `"PowerToys가 고정한 창이에요"` 안내 |

---

## 5. 위험 · 대응

| 위험 | 발생 신호 | 대응 |
|---|---|---|
| **UIPI로 관리자 창 `SetWindowPos`가 `ERROR_ACCESS_DENIED` 또는 무음 실패** | 함수는 0이 아닌데 `WS_EX_TOPMOST` 비트가 안 바뀜 | `SetWindowPos` 직후 `GetWindowLongPtrW(GWL_EXSTYLE) & WS_EX_TOPMOST != 0`을 실제로 재확인해 무음 실패까지 `PinError::AccessDenied`로 검출 |
| **다중 DPI 경계를 넘겨 창을 끌 때 테두리 두께/반경 어긋남** | 100%→150% 이동 시 테두리가 두껍거나 잘림 | 위치 변경 이벤트에서 대상 창의 `GetDpiForWindow`가 바뀌면 테두리 비트맵을 새 배율로 1회 다시 그림 |
| **앱 비정상 종료 시 `SetPropW`가 대상 창에 남음** | 재실행 전까지 창이 맨 위에 고정 | 다음 실행 `setup()` 최우선 단계에서 `cleanup_orphans()` 자동 실행 + 정상 종료 경로(`app.quit`, `Ctrl+C`, `WM_ENDSESSION`) 모두 `unpin_all()` 호출 |

---

## 6. 작업일 · 난이도 · 먼저 끝나야 할 것

| 항목 | 작업일 | 난이도 | 선행 |
|---|---:|---|---|
| `6.5.1` `WindowPinner` (도메인 판정·Win32 고정/풀기·PowerToys 판정) | 0.34일 | 중 | 단계 2 (끝남) |
| `6.5.2` 테두리 창·`SetWinEventHook` 따라가기 | 0.33일 | 상 | `6.5.1` |
| `6.5.3` 창 닫힘·종료·시작 잔여 정리·트레이 모두 풀기 | 0.33일 | 중 | `6.5.1`·`6.5.2` |
| **묶음 합계** | **1.00일** | **상** | 단계 2 완료(끝남) · P1이므로 일정 초과 시 후순위 |
