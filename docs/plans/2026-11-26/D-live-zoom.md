# D. 단계 5 실시간 확대 구현 시안 (`5.1`~`5.5`)

대상 항목(`progress.json`): `5.1` `Magnifier`(Magnification API)·M6 · `5.2` 커서 따라가기·휠 단계·배지 · `5.3` `Alt+1` 전환 · `5.4` 4K 30 fps 미달 시 2안 판단 · `5.5` 확대 섬·후광·`MiniMap`(원본 제외).

---

## 1. 목표 · 범위 (요구사항 ID)

| 요구사항 | 우선 | 요약 |
|---|---|---|
| `R-ZOOM-1` | P0 | Windows 1안 = **Magnification API** 돋보기 창(움직이는 화면·영상 그대로 확대, 시스템 커서 표시). M6 목표: **1080p ≥ 30 fps** (관문 G6) |
| `R-ZOOM-2` | P0 | 휠 배율 `1.25 · 1.5 · 2 · 2.5 · 3 · 4 · 6 · 8`, 커서 따라가기(`smooth`: 0.35 + 10% 죽은 영역 / `instant`: 즉시), `Esc`·`Alt+2` 종료 |
| `R-ZOOM-3` | P0 | 실시간 확대 중 `Alt+1` → **그 순간 확대 화면·배율 그대로 정지 판서로 전환 ≤ 150 ms** |
| `R-ZOOM-4` | P0 | `MagSetWindowFilterList(MW_FILTERMODE_EXCLUDE)`로 자기 창(섬·위치 지도·후광 창)을 돋보기 원본에서 제외(무한 거울 0) |
| `R-ZOOM-6` | P1 | 커서 주황 후광(44 px) + 오른쪽 아래 `MiniMap`(112×70, 모니터 이름), 설정 `zoom.minimap`으로 끌 수 있음 |
| `R-KEY-6` | P0 | 확대 화면 안 키 1칸(`z-exit` Esc) 설정값 반영 — **2026-11-26 사용자 결정: `z-in`·`z-out`(↑↓) 칸 삭제**, 배율은 휠로만(Ctrl+휠도 배율, Shift+휠은 아래 앱), 방향키는 아래 앱(PPT 등)으로 |

> 후속(1차 제외): `R-ZOOM-5`(움직이는 화면 위 LiveDraw 판서), macOS `SCStream → Metal` 실시간 확대.

---

## 2. 정확한 형식

### 2.1 데이터 모양 (Rust 도메인 · 앱 상태)

```rust
// crates/ink-domain/src/zoom.rs
pub const LIVE_ZOOM_STEPS: &[f64] = &[1.25, 1.5, 2.0, 2.5, 3.0, 4.0, 6.0, 8.0];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveZoomView {
    pub monitor_id: u32,
    pub monitor_bounds: PhysRect,
    pub scale: f64,           // 배율 (1.25 ~ 8.0)
    /// 돋보기가 비추는 원본 사각형(가상 화면 물리 px, 모니터 안에 가둠)
    pub source: PhysRect,
    /// 현재 추적 중심(물리 px 실수, 부드러운 추적 보간 상태)
    pub center_x: f64,
    pub center_y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FollowMode { Smooth, Instant }

// 순수 계산 함수
pub fn next_zoom_step(current: f64, dy: i32, steps: &[f64]) -> f64;
pub fn step_follow(view: &mut LiveZoomView, cursor: PhysPoint, mode: FollowMode, dt_sec: f64);
/// 실시간 확대 상태 → 판서 초기 변환(DrawViewTransform) 환산 (5.3 Alt+1 전환용)
pub fn to_draw_transform(view: &LiveZoomView, monitor_scale: f64) -> (f64, f64, f64); // (s, tx, ty)
```

```rust
// crates/ink-domain/src/mode.rs 추가
Mode::LiveZoom { session: u64, monitor_id: u32, started_ms: u64 }

// crates/ink-app/src/state.rs 추가 (UI 브로드캐스트 `screen-ink://zoom`)
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveZoomState {
    pub session: u64,
    pub monitor_id: u32,
    pub monitor_label: String, // "모니터 1" (창 제목 없음)
    pub scale: f64,            // 2.0 → 섬 "×2.0"
    /// MiniMap용 0..1 정규화 사각형
    pub viewport: NormRect,    // { x, y, w, h }
    /// 확대 화면 위 커서 위치(CSS px — 44px 주황 후광 배치용)
    pub cursor_css: Option<CssPoint>,
    pub show_minimap: bool,
}
```

### 2.2 포트 (`Magnifier`) · 어댑터 · 파일 위치

```rust
// crates/ink-app/src/ports.rs 추가
pub trait Magnifier: Send {
    /// 대상 모니터에 돋보기 창을 띄우고 `exclude_hwnds`(섬·HUD·팝오버)를 원본에서 제외.
    fn start(&mut self, monitor: PhysRect, initial_source: PhysRect, scale: f64, exclude_hwnds: &[u64]) -> Result<(), String>;
    /// 프레임마다 원본 사각형·배율 갱신.
    fn update(&mut self, source: PhysRect, scale: f64) -> Result<(), String>;
    fn stop(&mut self);
}
```

| 파일 위치 | 책임 |
|---|---|
| `crates/ink-domain/src/zoom.rs` | 확대 단계, `0.35` 부드러운 추적 + `10%` 죽은 영역, 모니터 가두기, `to_draw_transform` |
| `crates/ink-app/src/live_zoom.rs` | `LiveZoomState`, 진입(`begin_zoom`), 60Hz 틱(`tick_zoom`), 휠/키 입력, `Alt+1` 전환(`handoff_to_draw`), 종료 |
| `crates/ink-platform-windows/src/magnifier.rs` | `WinMagnifier`: `MagInitialize`, 호스트 창(`WS_EX_TOPMOST \| WS_EX_LAYERED \| WS_EX_TRANSPARENT \| WS_EX_TOOLWINDOW`), 자식 `WC_MAGNIFIER`(`MS_SHOWMAGNIFIEDCURSOR`), `MagSetWindowTransform`, `MagSetWindowSource`, `MagSetWindowFilterList(MW_FILTERMODE_EXCLUDE)` |
| `crates/tauri-plugin-screen-ink/src/zoom_hud.rs` | 실시간 확대용 투명 클릭 통과 HUD(`ink-zoom-hud` 또는 기존 `ink-overlay-<id>` 재사용, `WDA_EXCLUDEFROMCAPTURE` + `MagSetWindowFilterList` 제외 목록 포함) |
| `packages/ink-ui/src/lib/screens/ZoomHud.svelte` | 커서 주황 후광(44px 원) + 오른쪽 아래 `MiniMap`(112×70, `size="md"`, `label="모니터 1"`) |
| `packages/ink-client/src/index.js`, `tauri.js`, `fake.js` | 이벤트 `screen-ink://zoom`, 명령 `zoom_input` (`{ type: 'wheel', dy } \| { type: 'key', ... }`) |

### 2.3 화면 상태 · 입력 경로 · 문구 (`04 S13`)

- **중요한 입력 제약**: 실시간 확대 중에는 사용자가 **원본 화면의 앱을 그대로 클릭·스크롤·타이핑**할 수 있어야 한다(클릭 통과). 따라서 돋보기 창과 HUD 창은 `WS_EX_TRANSPARENT`로 마우스 클릭을 통과시킨다.
- **그럼 휠 확대와 `Esc`·`Up`/`Down`은 어떻게 받는가?**
  - **방식**: 실시간 확대가 **활성화된 동안에만** 임시로 `z-in`/`z-out`/`z-exit` 전역 단축키(또는 모드 활성 중에만 다는 짧은 마우스 휠 감시 — 단, 저수준 키보드 훅 금지 규칙 `R-KEY-1`은 엄수!).
  - **휠 처리 결정**: Windows에서 클릭 통과 창은 일반 `WM_MOUSEWHEEL`을 못 받는다. 대안 두 가지:
    1. **1안 (권장 — `Ctrl+휠` 또는 돋보기 활성 중 일시적 `WH_MOUSE_LL` 휠만 읽기, 키보드 훅 X)**: `R-KEY-1`이 금지하는 것은 저수준 *키보드* 훅이며, 확대 중 마우스 휠을 가로채려면 `WH_MOUSE_LL`에서 `WM_MOUSEWHEEL`만 소비(또는 `Ctrl` 조합 없이 확대 중 모든 휠을 배율로, `Alt+2` 종료 즉시 `UnhookWindowsHookEx`로 0 ms 해제)하는 것이 ZoomIt·아이캔 줌잇과 같은 유일한 표준 경로다. `[확인 필요]` 확대 중 일반 웹페이지 스크롤을 원할 때 `Shift+휠`로 통과시킬지 여부.
    2. **2안 (훅 완전 0)**: `z-in`(`Up`)·`z-out`(`Down`)·`z-exit`(`Esc`)를 모드 활성 중 임시 `RegisterHotKey`로만 받고, 휠 확대는 `Shift+휠` 등 불가 → **R-ZOOM-2("휠 = 배율 1.25~8배") 미충족**. 따라서 1안(확대 활성 중에만 `WH_MOUSE_LL` 휠 수신 + `z-in`/`z-out`/`z-exit` 임시 등록)을 채택한다.
- **2026-11-26 결정(구현 반영)**: 확대 중 잠시 잡는 전역 키는 `z-exit`(Esc) 하나. ↑↓는 잡지 않는다. 휠은 `WH_MOUSE_LL`(확대 모니터 안·Shift 없을 때만 먹음).
- **확대 섬 (`04 S13`)**:
  - 1줄: 확대 아이콘 · **실시간 확대** · `움직이는 화면 그대로` · `×2.0` (주황 고정폭)
  - 2줄(처음 3회·`?`): `휠` 배율 · `Alt 1` 멈추고 쓰기 · `Esc` 끝

---

## 3. 알고리즘 · 결정 (대안과 고른 이유)

### 3.1 커서 따라가기 다듬기 (`02 §4`)

```rust
// 프레임당(60Hz 기준 dt=1/60)
let src_w = (monitor.w as f64 / scale).round();
let src_h = (monitor.h as f64 / scale).round();
let dead_x = src_w * 0.05; // 중심 기준 ±5% = 전체 폭의 10% 죽은 영역
let dead_y = src_h * 0.05;
if (cursor.x as f64 - view.center_x).abs() > dead_x || (cursor.y as f64 - view.center_y).abs() > dead_y {
    let alpha = match mode {
        FollowMode::Instant => 1.0,
        // 60Hz에서 0.35와 동등한 프레임레이트 독립 지수 감쇠
        FollowMode::Smooth => 1.0 - (1.0 - 0.35_f64).powf(dt_sec * 60.0),
    };
    view.center_x += (cursor.x as f64 - view.center_x) * alpha;
    view.center_y += (cursor.y as f64 - view.center_y) * alpha;
}
// 원본 사각형을 모니터 안에 가두기
let sx = (view.center_x - src_w / 2.0).clamp(monitor.x as f64, monitor.right() as f64 - src_w);
let sy = (view.center_y - src_h / 2.0).clamp(monitor.y as f64, monitor.bottom() as f64 - src_h);
```

### 3.2 `5.3` `Alt+1` 실시간 확대 → 판서 전환 (≤ 150 ms)

1. `Alt+1` 수신(현재 `Mode::LiveZoom`):
2. 현재 `LiveZoomView { monitor_id, source, scale }` 복사.
3. `magnifier.stop()`으로 돋보기 창 즉시 숨김(자기 확대 화면이 캡처되지 않게).
4. **같은 순간** 커서 모니터 1장 정지 캡처(`ScreenSource::capture`) — 약 35~50 ms.
5. `to_draw_transform(&view, monitor.scale)`로 판서 초기 변환 계산:
   - `tx = -(source.x - monitor.x) as f64 / monitor.scale * scale`
   - `ty = -(source.y - monitor.y) as f64 / monitor.scale * scale`
   - `s = scale`
6. `begin_draw_with_frame(monitor_id, frame, DrawViewTransform { s, tx, ty })` 실행 → 사용자가 보던 확대 장면 그대로 멈추며 즉시 빨간 펜 대기(총 70~110 ms, 목표 ≤ 150 ms 충족).

### 3.3 `5.4` 4K 30 fps 미달 시 2안 판단 절차

- 학교 PC + 4K(3840×2160) 모니터(또는 4K 가상 해상도)에서 1080p 유튜브/로컬 영상 재생 중 PresentMon으로 `Magnifier` 호스트 창 FPS를 60초 측정한다.
- **판정 기준 (관문 G6, `07 §3`)**:
  - 1080p ≥ 30 fps & 4K ≥ 30 fps → Magnification API 확정.
  - 1080p ≥ 30 fps & 4K < 30 fps → 결정 메모 `bench/results/2026-11-xx-m6-zoom.md` 작성: 1차는 1080p 기준 마감, 2안(**WGC `CreateForMonitor` → Direct3D11/wgpu 텍스처 셰이더 확대**)을 2차로 이관.
  - 1080p < 30 fps → 즉시 2안 프로토타입 착수.

---

## 4. 시험 · 합격 기준

| 시험 / 측정 | 이름 예시 | 합격 기준 |
|---|---|---|
| Rust 단위 (`ink-domain::zoom`) | `커서_죽은영역_10퍼센트_미이동_및_부드러운_추적` | 중심 ±5% 안 이동 시 `center` 불변, 바깥 이동 시 0.35 수렴 |
| Rust 단위 (`ink-domain::zoom`) | `원본사각형_모니터_경계_가두기_및_음수좌표` | 왼쪽 음수 모니터(`x=-1920`)에서도 밖으로 나가지 않음 |
| Rust 단위 (`ink-domain::zoom`) | `실시간확대_판서_초기변환_좌표일치` | 확대 좌상단이 판서 `viewToDoc(0, 0)`와 0.5 px 이내 일치 |
| Rust 통합 (`ink-platform-fake`) | `확대중_Alt1_전환_및_잠금시_자동종료` | `LiveZoom → Draw` 배율 유지, `Lock` → `magnifier.stop()` 호출 |
| **관문 G6 (M6)** | PresentMon 60초 측정(영상 재생 중 S9) | 1080p **≥ 30 fps**, 4K 수치 기록 |
| 눈·캡처 검사 | `MagSetWindowFilterList` 제외 확인 | 확대 화면 안에 섬·`MiniMap`·후광이 **2중으로 찍히지 않음** |
| 전환 시간 측정 | `Alt+2` → `Alt+1` 20회 | 전환 `p95 ≤ 150 ms` |

---

## 5. 위험 · 대응

| 위험 | 발생 신호 | 대응 |
|---|---|---|
| **Magnification API 초기화 실패 / 스레드 제약** | `MagInitialize`가 백그라운드 스레드에서 실패하거나 창이 안 보임 | 돋보기 호스트 창 생성·`MagSetWindowSource` 호출을 **전용 UI 메시지 펌프 스레드 1개** 또는 메인 스레드 16ms 타이머로 고정 |
| **다중 모니터·혼합 배율에서 `MagSetWindowSource` 좌표 어긋남** | 150% 보조 모니터에서 확대 위치가 밀림 | 프로세스 Per-Monitor V2 상태에서 `MagSetWindowSource`는 물리 px 가상 화면 좌표를 받음 — 실기에서 100%+150% 검증 후 어긋나면 `SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` 명시 |
| **자기 창 무한 거울** | 확대 화면 안에 섬·`MiniMap`이 재귀 확대됨 | `start()` 시 `ink-island`, `ink-zoom-hud`, `ink-popover`의 HWND 전체를 `MagSetWindowFilterList(MW_FILTERMODE_EXCLUDE)`에 등록 + 창 생성/재생 시 리스트 갱신 |
| **DRM/전체 화면 독점 게임 검은 화면** | 확대 화면이 검게 나옴 | Magnification API 한계 — 1차 범위 외(안내 문구 필요 시 `04` 말투로 표시) |
| **확대 중 `WH_MOUSE_LL` 미해제** | 앱 크래시 시 휠이 먹통 | Windows는 프로세스 종료 시 훅 자동 해제 + `Drop` 구현 + 1초 하트비트 누락 시 자동 언훅 |

---

## 6. 작업일 · 난이도 · 먼저 끝나야 할 것

| 항목 | 작업일 | 난이도 | 선행 |
|---|---:|---|---|
| `5.1` `Magnifier`(Magnification API) · M6 | 0.40일 | 상 | 단계 2 |
| `5.2` 커서 따라가기·휠 단계·배지 | 0.40일 | 중 | `5.1` |
| `5.3` `Alt+1` 전환 | 0.40일 | 중 | `5.1` + **A** (`4.2`·`4.4`) |
| `5.4` 4K 30 fps 판단 기록 | 0.40일 | 중 | `5.1` |
| `5.5` 확대 섬·후광·`MiniMap`(원본 제외) | 0.40일 | 중 | `5.1`·`5.2` |
| **묶음 합계** | **2.00일** | **상** | 단계 2 (완료), `5.3`은 **A** 선행 |
