# 인수인계 — 단계 3 이후, 맥 / Windows 나눔

- 기준: 브랜치 `feat/foundation`, 커밋 `61e2450`(2026-11-25). 맥 작업 반영 2026-11-26 아침(§2.1 진행 표·§2.5·§3.0·§4)
- **학교에서 바로 할 일은 §3.0**에 순서대로 적었다.
- **진척도(%)·남은 일 전체 목록은 [docs/progress/README.md](../progress/README.md)**(원본 `docs/progress/progress.json`). 항목 상태를 바꾸면 JSON을 고치고 `npm run progress`로 보고서를 다시 만든다. 커밋 전 `npm run progress -- --check`.
- 배포·스토어·서명 방침은 [docs/prd/11-distribution.md](../prd/11-distribution.md)(진척도 `D1`~`D7`).
- 범위: 남은 일을 **맥에서 할 것(§2)** 과 **Windows에서만 할 수 있는 것(§3)** 으로 나눈다. 두 쪽 공통 규칙은 §1.
- 표기: **[완료]** 이미 있음 · **[할 일]** 새로 할 것 · **[확인 필요]** 아직 실제로 확인하지 않은 가정

---

## 0. 지금 상태 (한눈에)

| 단계 | 상태 | 근거 |
|---|---|---|
| 0 뼈대·디자인 시스템 | [완료] | `d73122a`, `a2c4698` |
| 1 팝오버·설정·단축키·저장소 | [완료] | 같음 |
| 2 정지 스냅숏·모니터별 오버레이·섬 | [완료] Windows 진입 p50 60 ms · p95 69 ms | `57afa28`, `bench/results/2026-11-25-m1-overlay.md` |
| 3 F1 스마트 캡처 v2 | **부분 완료** | 아래 |

단계 3에서 된 것:
- **[완료]** 인식 근거: W(창 목록)·H(자식 창)·A(UIA 색인)·V(이미지, `crates/ink-vision`)
- **[완료]** 융합(`ink-domain::recog`): 검증·경계 맞춤·그릇 빼기·중복 합치기·점수·첫 강조·깊이 기억·제목 표시줄 규칙
- **[완료]** 고르기(`ink-domain::selection`): 휠·Tab·드래그 자석·방향키·확정·R과 `,` `.` 기록·커서 포함(`` ` ``)
- **[완료]** 결과: 복사(DIB+PNG)·저장·기록·결과 섬(썸네일·[저장])
- **[완료]** 재생 시험 틀(08 §5.2): `ink_app::case`, `ink-platform-fake::replay`, `recog-replay` 보고서, 사람 정답 도구(`SCREEN_INK_CASE_DIR`)
- **[완료]** CI 워크플로(`.github/workflows/ci.yml`, PR과 수동 실행만)

단계 3에서 안 된 것은 §2.3(맥에서 가능)과 §3.1(Windows 전용)에 나눠 적었다.

---

## 1. 공통 규칙 (맥·Windows 둘 다 지킬 것)

1. **작업 전에 받고, 푸시 전에 다시 받는다.** 두 컴퓨터가 같은 브랜치를 쓴다.
   ```
   git pull --rebase origin feat/foundation
   ```
2. **플랫폼 코드는 자기 자리에만 둔다.**
   - Windows: `crates/ink-platform-windows/**`, 플러그인의 `#[cfg(windows)]` 블록
   - 맥: 새 `crates/ink-platform-macos/**`, 플러그인의 `#[cfg(target_os = "macos")]` 블록
   - 맥 작업자는 Windows 크레이트의 동작을 바꾸지 않는다. 반대도 같다. 예외는 컴파일이 안 될 때 `#[cfg(...)]`를 붙이는 것뿐이고, 그때는 커밋 메시지에 적는다.
3. **공통 계약은 함부로 바꾸지 않는다.** 바꾸면 두 플랫폼 빌드가 다 되게 같이 고치고 이 문서에 적는다.
   - 포트(trait): `crates/ink-app/src/ports.rs` — `ScreenSource`, `WindowCatalog`, `MonitorProbe`, `ElementSource`, `VisionSource`, `Clipboard`, `FileSink`, `Clock`, `OverlayHost`, `Notifier`, `SystemProbe`
   - 좌표: **가상 화면 물리 px, 왼쪽 위 원점, y는 아래로**. `PhysRect{x, y, w, h}`. 모든 근거·강조·자르기가 이 좌표 하나만 쓴다(R-DPI-1).
   - 사례 형식: `ink_app::case::Case`(`schema: 1`). 필드를 바꾸면 `CASE_SCHEMA`를 올리고 읽기 쪽을 맞춘다.
   - UI ↔ 앱: `packages/ink-client`(`InkClient`)와 이벤트 `screen-ink://state|island|overlay|capture|first-run`
   - 오버레이: `OverlaySession.nativeFrame` — `true`는 네이티브 창이 정지 화면을 그림(Windows GDI·맥 CALayer), `false`는 웹뷰가 이미지와 50% 어둡게를 그림(네이티브 창을 못 띄웠을 때의 대체 경로)
4. **공통 검사는 커밋 전에 한 번씩.** 두 OS 모두에서 통과해야 한다.
   ```
   cargo test -p ink-domain -p ink-app -p ink-vision -p ink-store -p ink-platform-fake
   cargo clippy -p ink-domain -p ink-app -p ink-vision -p ink-store -p ink-platform-fake --all-targets -- -D warnings
   npm test
   npm run check
   ```
5. **개인정보**
   - 사례 PNG에는 화면 전체가 담긴다. 그러니 **우리가 만든 시험 문서만 띄운 화면**에서만 뜬다.
   - 금지: 사용자 이름, 메신저, 메일, 학생·교직원 정보, 실제 업무 화면.
   - 로그에는 창 제목·파일 이름·사용자 이름을 쓰지 않는다(05 §9).
6. **로그로 확인하기**
   - 앱 로그 파일: Windows `%LOCALAPPDATA%\kr.screenink.desktop\logs\screen-ink.log`, 맥 `~/Library/Logs/kr.screenink.desktop/screen-ink.log`(**확인됨** 2026-11-25, 맥미니). 로그 줄의 시각은 UTC다.
   - `SCREEN_INK_LOG=debug`로 띄우면 강조가 바뀔 때마다 `highlight … path 창 > 상자 > …`와 `vision full monitor … ms`가 남는다.
7. **F1 직후에는 모니터 전체가 강조된다.** 커서를 **4 px 이상** 움직여야 요소 강조로 바뀐다. 설계대로이며 버그가 아니다. 자동 조작 스크립트는 반드시 4 px 넘게 움직인다.

---

## 2. 맥에서 할 일

### 2.0 첫 확인 — 맥에서 빌드한 적이 없다

1. 준비: Xcode Command Line Tools, Rust stable(1.80 이상), Node 22 이상
2. 실행
   ```
   npm install
   cargo test -p ink-domain -p ink-app -p ink-vision -p ink-store -p ink-platform-fake
   cargo check -p tauri-plugin-screen-ink -p screen-ink
   npm run app:dev
   ```
3. 결과별 대처
   - `cargo test --workspace`가 `ink-platform-windows` 때문에 깨질 때: 그 크레이트 안의 Windows API 사용을 `#[cfg(windows)]`로 막는 것만 한다. 지금 `clipboard·events·uia·freeze·screen` 모듈과 Windows 의존성은 이미 막혀 있다. **확인됨**(2026-11-25 맥: `cargo check -p ink-platform-windows --all-targets` 통과, 고칠 것 없음)
   - 맥 빌드 설정
     - 지금 `bundle.targets`는 `["nsis"]`뿐이다. 맥 설치본(`app`·`dmg`)은 Windows 설정을 건드리지 않게 플랫폼별 설정 파일(`tauri.macos.conf.json`)로 더하는 것을 권장한다. **반영함**(`apps/screen-ink/src-tauri/tauri.macos.conf.json`: `app`·`dmg`, 최소 macOS 14 — ScreenCaptureKit 스크린숏 API). 맥 설치본 빌드는 아직 안 해 봄 **[확인 필요]**
     - macOS 투명 창(섬·팝오버)에는 Tauri `app.macOSPrivateApi: true`와 `tauri` 크레이트의 `macos-private-api` 기능이 필요하다. 넣은 뒤 Windows 빌드가 깨지지 않는지도 확인한다. **반영함**(`tauri.conf.json` `app.macOSPrivateApi: true` + 앱 크레이트 `tauri` 기능 `macos-private-api`. tauri-build가 설정과 기능이 같은지 보므로 플랫폼별로 나누지 않고 공통에 넣었다). Windows 빌드 확인 **[확인 필요]** → §3.2
4. **지금 맥에서 보이는 동작** — 아래는 정상이다(아직 빈 어댑터라서).

   | 동작 | 이유(코드) |
   |---|---|
   | 트레이·팝오버·설정·단축키 화면은 뜬다 | 플랫폼 무관 |
   | F1 → 섬에 "화면을 가져오지 못했어요" | `NullScreen::capture`가 `Err` (`tauri-plugin-screen-ink/src/lib.rs`) |
   | 창 목록·모니터 배치가 비어 있음 | `NullScreen` |
   | 요소(접근성) 근거 없음 | `NoElements` |
   | Enter → "클립보드를 쓸 수 없어요" | `TauriClipboard`가 맥에서 `Err` (`adapters.rs`) |
   | 저장 파일 이름의 시각이 UTC | `SystemClock::local` 맥 대체 코드 |
   | 잠금·절전·디스플레이 변경 사건 없음 | `listen_system_events`가 Windows 전용 |

5. 맥 기본 단축키도 F1·F2·F3이다(`docs/shortcuts/defaults-v1.json`의 `mac` 열). 맥 키보드는 F 키가 기본으로 밝기 같은 특수 기능이라, 시험할 때는 fn을 같이 누르거나 "F1, F2 등의 키를 표준 기능 키로 사용"을 켠다. 이 안내를 첫 실행 시트에 넣을지는 맥에서 판단한다. **[확인 필요]**

### 2.1 맥 어댑터 — 맥의 모든 일의 선행 조건

새 크레이트 `crates/ink-platform-macos`를 만든다(03 §2: screencapturekit-rs, axuielement-rs 제안).
- 플러그인 `Cargo.toml`에 `[target.'cfg(target_os = "macos")'.dependencies]`로 붙인다.
- 플러그인에서 맥 분기를 채울 함수: `screen_ports()`, `element_source()`, `probe()`, `TauriClipboard`, `exclude_from_capture()`, `listen_system_events()`, `SystemClock::local()`
- Windows 쪽 같은 일을 하는 파일을 본보기로 쓴다: `ink-platform-windows/src/{screen,uia,clipboard,events}.rs`

권장 순서는 아래 표 위에서부터다. 한 줄씩 끝날 때마다 완료 기준을 확인하고 커밋한다.

| # | 포트 | 계약(어긋나면 인식이 틀어짐) | 맥 구현 단서 | 완료 기준 |
|---|---|---|---|---|
| M1 | `MonitorProbe` | `layout()`: 모니터마다 `Monitor{id, bounds(물리 px), work(메뉴 막대·Dock 뺀 영역), scale, primary}`, 서로 **겹치지 않게**<br>`cursor()`: 물리 px<br>`set_cursor`(WASD 1 px), `alt_down`(⌥) | `NSScreen.frame/visibleFrame/backingScaleFactor`, `CGWarpMouseCursorPosition`, `CGEventSourceFlagsState` | 좌표 변환 함수가 **한 곳**에만 있음 + 단위 시험(아래 "좌표 주의") |
| M2 | `ScreenSource::capture` | 모니터마다 `Frame{monitor_id, width, height, bgra}`<br>BGRA는 **위→아래, 물리 px, 알파 255**<br>`width·height` = 그 모니터 `bounds.w·h`<br>창 목록과 같은 순간에(서비스가 둘을 나란히 부름) | ScreenCaptureKit `SCScreenshotManager` 또는 `CGDisplayCreateImage`<br>**우리 앱 창은 빼고 찍기**(`SCContentFilter` 제외 앱) | F1 → 정지 화면이 보이고 바깥이 50% 어두움 |
| M3 | `ScreenSource::cursor_sprite` | `CursorSprite{x, y = 스프라이트 왼쪽 위(핫스팟 뺀 위치, 물리 px), w, h, bgra(곧은 알파)}`<br>숨은 커서면 `None` | `NSCursor.currentSystemCursor` 이미지 + `hotSpot` | `` ` `` 키를 켜고 확정 → 결과에 커서 |
| M4 | `WindowCatalog::windows` | `RawWindow` 앞→뒤 순서<br>`handle`(CGWindowID), `pid`, `exe`(소문자 프로세스 이름), `class`, `rect`(물리 px)<br>`visible`, `minimized`, `cloaked`(다른 Space·화면 밖), `layered_alpha`(창 알파×255), `topmost`(층 > 0), `click_through`, `tool`<br>창 제목은 **읽지 않는다** | `CGWindowListCopyWindowInfo(optionOnScreenOnly)`의 `kCGWindowNumber·OwnerPID·Bounds·Layer·Alpha`<br>`class`에 무엇을 넣을지(번들 ID 권장) 정해 이 문서에 적기 | 창 위에서 F2 → 그 창 강조, 메뉴 막대·Dock이 창으로 잡히지 않음 |
| M5 | 덮개 거르기(D23) | 알림 센터처럼 화면을 덮는 투명 창이 강조되면 실패 | M4에서 `topmost`·`layered_alpha`를 정확히 채우면 도메인 `filter_windows`의 덮개 규칙(D1)이 걸러 냄. 부족하면 **도메인 규칙에 시험을 더해** 고친다 | 알림을 띄운 채 F1 → 알림 덮개가 강조되지 않음 |
| M6 | `Clipboard` | `set_image(w, h, bgra)`: PNG(+TIFF), `set_text` | `NSPasteboard` | Enter → 미리보기·키노트·Pages에 붙여 넣으면 크기·색이 같음 |
| M7 | 오버레이 | `nativeFrame=false` 경로가 지금도 있다: 웹뷰가 `ink://localhost/frame/<세션>/<모니터>`를 그림<br>오버레이 창은 **메뉴 막대·Dock 위**, 모든 Space, 전체 화면 앱 위 | Tauri 창 + NSWindow 층(예: screen saver 층)·`collectionBehavior` 조정 **[확인 필요]** | 로그 `overlay ready [N ms]`로 맥 M1 측정(목표 p50 ≤ 80 · p95 ≤ 150 ms). `bench/m1-overlay.ps1`의 맥판 스크립트 새로 작성 |
| M8 | 캡처 제외 | 섬·팝오버는 스크린숏에 찍히지 않는다(Windows `WDA_EXCLUDEFROMCAPTURE`와 같은 뜻) | `NSWindow.sharingType = .none` + M2의 제외 앱 필터 | 섬을 띄운 채 시스템 스크린숏 → 섬 없음 |
| M9 | `ElementSource`(AX) | `request(window, exe)`: **일꾼 스레드**에서 그 창 하위 트리를 한 번 색인(상한 1500개·깊이 8), 끝나면 콜백 `(window, exe, ms)` → 메인 스레드 `on_elements_ready`<br>`elements_at` / `elements` / `rects`는 **메모리에서만**<br>`clear`는 세대 번호를 올림<br>UI 스레드는 다른 앱을 절대 기다리지 않는다(D17) | `AXUIElementCreateApplication(pid)` → 창 → `AXPosition·AXSize·AXRole`<br>`AXUIElementSetMessagingTimeout`(0.4초 권장)<br>역할 → `Control` 변환은 **도메인에 `Control::from_ax(role)`로 더하고 단위 시험**(`from_uia` 옆) | 사파리·Finder 위에서 요소 강조, 느린 앱이어도 화면이 굳지 않음, 150 ms 넘으면 느린 앱 강등이 동작 |
| M10 | 권한 | 화면 기록·손쉬운 사용 권한이 없으면 **조용히 실패하지 말고** 섬과 첫 실행 시트에서 안내 | `CGPreflightScreenCaptureAccess`·`CGRequestScreenCaptureAccess`, `AXIsProcessTrustedWithOptions`<br>`app:dev`로 띄우면 권한이 터미널 앱에 붙을 수 있음 **[확인 필요]** | 권한을 끈 채 F1 → 안내 문구, 켜면 바로 동작 |
| M11 | `SystemProbe`·사건·시계 | `running_apps`(소문자 이름), `live_bindings` → `Some(빈 목록)`(PowerToys는 Windows 전용), `secure_desktop`(화면 잠김)<br>사건: 잠금·해제·절전·깨어남·디스플레이 변경·테마<br>`Clock::local` = 지역 시각 | `NSWorkspace` 알림, `NSApplicationDidChangeScreenParametersNotification`, `localtime_r` | 모니터를 바꾸면 오버레이가 다시 맞춰짐, 파일 이름이 지역 시각 |

**맥 진행 상황 (2026-11-25, 맥미니 4K@2 + FHD@1, macOS 27)** — 코드는 `crates/ink-platform-macos`

| # | 상태 | 확인한 것 | 남은 확인 |
|---|---|---|---|
| M1 | **완료** | `coords::tests` 8개, 실제 배치 `0,0 3840x2160` + `3840,0 1920x1080`(겹침 없음) | 세로 배치·3대는 시험으로만 |
| M2 | **완료** | ScreenCaptureKit, 우리 앱 제외 필터, sRGB·BGRA, 크기 = bounds. 확인됨 2026-11-25: 터미널 앱에서 띄우고 화면 기록을 켠 뒤 F1 → 정지 화면·50% 어둡게, 전체 화면 앱 위에서도 그 자리 | — |
| M3 | 코드 완료 | 시험: 커서 스프라이트 56x80(배율 2). `NSCursor.currentSystemCursor`는 사용 중단 예고(나중에 nil이면 None) | **[확인 필요]** `` ` `` 켜고 확정 → 결과에 커서 |
| M4 | 코드 완료 | 시험: 창 목록(경로 없는 이름). 권한 없이도 경계·소유자는 읽힘 | **[확인 필요]** 창 위에서 F2 → 그 창 강조 |
| M5 | 코드 완료(도메인 규칙 그대로) | — | **[확인 필요]** 알림을 띄운 채 F1 |
| M6 | **완료(앱 붙여 넣기 빼고)** | 시험: 임시 판에 TIFF(크기·색)·PNG·글 | **[확인 필요]** 미리보기·키노트·Pages에 붙여 넣기 |
| M7 | 코드·연결 완료 | 오버레이 층 1000(메뉴 막대·Dock 위), 모든 Space·전체 화면 앱 위, 포인트로 직접 놓기. 웹뷰 정지 화면일 때 실측 145·193 ms(캡처 56·110, ui load 58·38, paint 27·38) → 정지 화면을 네이티브 패널(`freeze.rs`, 층 999)로 옮김. 측정 스크립트 `bench/m1-overlay-mac.sh`. **확인됨 2026-11-26**: 네이티브 정지 화면 처음 30회 p50 92.5 · p95 95 ms(4K 패널 flush 24 ms = sRGB 그림을 디스플레이 색으로 CPU 변환) → 패널 창 색 공간을 sRGB로 맞춘 뒤 **30회 p50 64 · p95 74 · 최대 77 ms(통과)**, 캡처 p50 49, show p50 22, 4K flush 3.5 ms | **[확인 필요]** 색 공간을 바꾼 뒤 정지 화면 색이 실제 화면과 같은지 눈으로(창 서버가 sRGB→디스플레이로 바꾸므로 같아야 함). 방향·구멍·테두리는 사용자가 F1을 써 본 범위만 |
| M8 | 코드·연결 완료 | 섬·팝오버 `sharingType = none`, 섬 층 1001 | **[확인 필요]** 섬을 띄운 채 시스템 스크린숏 |
| M9 | 못 함 | — | 손쉬운 사용 권한도 필요 |
| M10 | 일부 | 화면 기록 권한이 없으면 첫 F1에서 시스템 요청 한 번 + 섬 안내("화면 기록 권한이 필요해요", 가짜 시험 1개) | **권한 스마트 흐름(§2.5-3, 2026-11-26 사용자 결정)**, 손쉬운 사용 권한 |
| M11 | 코드 완료 | 시험: 화면 잠김 판정. 디스플레이 변경(CG 콜백)·절전·깨어남(NSWorkspace)·잠금·해제·테마(분산 알림) → 서비스. 지역 시각 | **[확인 필요]** 모니터를 뺐다 꽂으면 오버레이가 다시 맞춰짐 |

- `npm run app:dev`로 띄우면 화면 기록·손쉬운 사용 권한은 **띄운 쪽 앱**에 붙는다. 터미널 앱에서 띄우면 터미널에 붙는다(확인됨 2026-11-25: 터미널에 화면 기록을 켜자 캡처가 됨).
- Claude 데스크톱 앱의 Code 탭에서 띄우면 권한은 Claude가 아니라 **"Claude Code"**(`com.anthropic.claude-code`, `~/Library/Application Support/Claude/claude-code/<버전>/<해시>/claude.app`)에 붙는다. 2026-11-26에 이 항목에 화면 기록·손쉬운 사용을 켰다. 화면 기록은 켠 뒤 **Claude 앱을 다시 시작해야** 반영됐다(확인됨). 이제 에이전트가 F1·Esc·마우스를 `bench/mac/*.swift`로 보내 직접 측정할 수 있다.
- 맥 앱은 Dock 아이콘 없이 메뉴 막대에만 둔다(`ActivationPolicy::Accessory`).

**좌표 주의 (M1)**
- 맥은 포인트 단위이고 Cocoa는 원점이 왼쪽 아래다. CG 전역 좌표는 왼쪽 위 원점이다. 변환은 **한 함수**에서만 한다.
- 모니터마다 배율이 다르면(Retina 2.0 + 외장 1.0) `포인트 × 배율`만으로는 물리 배치가 **겹치거나 틈이 생긴다.**
- 권장: 각 모니터의 물리 원점을 이웃 방향으로 밀어 겹침 없게 만든다(Windows 물리 배치와 같은 성질).
- 이 경우는 사례를 하나 떠서 재생 시험으로 고정한다.
- 이 결정은 이 문서 §4에 적는다.

### 2.2 맥 어댑터가 된 뒤 — 맥 사례 뜨기

- 시험 문서만 띄우고 실행한다.
  ```
  SCREEN_INK_CASE_DIR="$PWD/bench/recog/cases" npm run app:dev
  ```
- F1 → 정답 조각 → 휠·드래그로 정답을 맞추고 → Enter. 확정 1번에 사례 1개가 생긴다. 자세한 절차는 `bench/recog/README.md`.
- JSON의 `note`에 `"mac · RB-01 사파리"`처럼 **맥에서 뜬 것임을 적는다.** 파일 이름은 바꾸지 않는다(`frame` 필드와 짝).
- 재생:
  ```
  cargo test -p ink-platform-fake --test replay
  cargo run -p ink-platform-fake --bin recog-replay
  ```
- 결과가 좋아졌을 때만 `bench/recog/cases/baseline.json`(사례 id → Top-1로 맞힌 점 수)의 숫자를 올린다. 내리지 않는다.

### 2.3 단계 3의 남은 일 중 맥에서 해도 되는 것 (플랫폼 무관)

| # | 할 일 | 근거 | 지금 상태 | 완료 기준 |
|---|---|---|---|---|
| X1 | 결과 섬 **추천 칩**(단색이면 `#RRGGBB 복사`) 표시 | R-CAP-26·R-CAP-14 | 앱은 `IslandEvent.chip{label, action:"capture.copyHex"}`를 이미 보냄. `packages/ink-ui/src/lib/components/Island.svelte`가 아직 안 그림 | 단색 범위 확정 → 칩, 누르면 HEX가 클립보드로(`capture.copyHex`) |
| X2 | 고른 자리 **주황 잔상 0.4초** | R-CAP-26 | `SelectionFrame`에 `afterimage` 상태는 있음. 그런데 확정하면 오버레이를 바로 숨겨서 실제로는 안 보임 | 확정 직후 그 자리에 0.4초 잔상. 진입 시간 계측(M1)은 그대로 |
| X3 | 숨은 설정 **"인식 근거 보기"** | 08 §4.4~4.5, 07 3d | 없음 | 켜면 사슬 마디마다 근거 색(W 파랑·A 초록·M 청록·V 주황)과 점수. 기본은 꺼짐 |
| X4 | **RB-01 시험 HTML + 자동 정답** | 08 §5.1 | 없음 | 상자·표·버튼·그림·문단 페이지<br>페이지가 아는 배치(CSS px) × 배율 + 문서 영역 원점 → `Case.points`를 자동으로 만드는 도구<br>`bench/recog/`에 둠 |
| X5 | **RB-14** 캔버스·WebGL 시험 페이지 | 08 §5.1 | 없음 | 자동 정답 |
| X6 | Snipaste 대조 하네스의 **판독 코드** | 08 §5.4 | 없음 | "원본과 화소가 같은 영역 = 그 도구의 강조 사각형"을 찾는 순수 코드 + 시험. 실제 대조 측정은 Windows(§3.1 W9) |

### 2.4 단계 4 이후 중 맥에서 해도 되는 것

| 단계 | 맥에서 | Windows에 남김 |
|---|---|---|
| 4 판서·칠판 | 4.1 `inkEngine.js`(순수, `node --test`), 4.2 `DrawCanvas`, 4.3~4.5 키·도형·확대·글자, 4.8 판서 섬 UI | G5 펜 지연(M5)은 학교 PC에서 측정. 미달 때의 네이티브 렌더러(Direct2D)도 Windows |
| 5 실시간 확대 | UI(배지·`MiniMap`) | `Magnifier`(Magnification API), G6 |
| 6 F2·F3 | 바로 캡처 키 로직(6.2, 도메인·앱) | WGC 창 캡처(6.1), `Recorder`(6.3~6.5) |
| 6.5 창 고정 | — | 전부(맥 = `Unsupported`) |
| 7 안정화 | 디자인 관문 중 화면 비교(라이트/다크) | M1~M10 학교 PC, NSIS·포터블 ZIP, 릴리스 체크리스트 7.5 |

### 2.5 맥 다음 할 일 (2026-11-26 기준 우선순위)

1. **진입 시간(M7) — 통과(2026-11-26)**: 패널 창 색 공간 sRGB로 p50 64 · p95 74 ms. 아래는 더 줄일 때의 후보(캡처 49 ms가 이제 대부분)
   - 지금 구성
     - 캡처 p50 49 ms: 4K는 SCK 32 + 변환 7, FHD는 SCK 40. 두 모니터를 동시에 찍고 늦은 쪽을 기다린다.
     - 네이티브 패널 두 장을 띄우는 데 약 40 ms.
   - 후보(효과가 클 것 같은 순서)
     1. (했음) `freeze::show` 단계별 시간 디버그 로그 `freeze monitor N: frame, layers, flush, front ms`. 4K flush 24 ms의 원인이 색 변환이라 창 색 공간으로 해결.
     2. 모니터마다 캡처가 오는 대로 바로 그린다. 지금은 다 모은 뒤 그린다. 커서 모니터를 먼저 그린다.
     3. BGRA 변환(4K 7 ms)을 그리기 뒤로 미룬다. 정지 화면은 `CGImage`만 쓰고, BGRA는 인식(V)·자르기에만 쓰인다.
        - 다만 `ScreenSource::capture`가 `Frame(bgra)`를 돌려주는 것이 계약이다. 계약을 건드리지 않는 방법(나중에 채우기)을 먼저 찾는다.
        - 계약을 바꿔야 하면 먼저 보고한다.
   - 완료 기준: 30회 이상에서 p50 ≤ 80, p95 ≤ 150.
   - 측정: `sh bench/m1-overlay-mac.sh 30`
2. **듀얼 모니터: 모니터를 넘어 끌기 — 맥은 고침(2026-11-26)**
   - 고친 것
     - UI가 `pointerdown`에서 `setPointerCapture`를 건다.
     - 플러그인 `capture_input`(맥 전용 블록)이 자기 모니터 밖의 점을 실제 커서(`display::cursor`)로 바꾼다. 순수 함수는 `overlay::pointer_inside`이고 시험 1개가 있다.
   - 확인(자동 입력)
     - 4K pt(1500,500)에서 FHD pt(2020,300)로 끌면 범위가 `3000,300 940x686`이다. 끝점 (3940,300)이 커서와 정확히 맞는다.
     - FHD pt(2300,400)에서 4K pt(1700,700)로 끌면 범위가 `3400,400 820x1000`이다. 두 끝 모두 맞는다.
   - Windows는 UI 변경(`setPointerCapture`)만 받는다. §3.0-3에서 확인한다.
   - 아래는 고치기 전에 쓴 기록이다.
   - 지금 흐름(코드 확인)
     1. F1을 누른 순간 커서 위치를 읽는다.
     2. 모든 모니터를 한꺼번에 찍고, 모든 모니터에 정지 화면을 깐다.
     3. 처음 강조는 커서가 있는 모니터 전체다. F2는 커서 아래 창이다.
     4. Esc·Enter는 커서가 있는 모니터의 오버레이가 받는다. 결과 섬은 커서 모니터 위쪽 가운데에 뜬다.
     5. Ctrl+A를 두 번 누르면 두 모니터 전체가 잡힌다.
   - 확인된 것: 오른쪽 FHD 모니터에서 F1을 누르면 그 모니터가 강조되고 창·상자도 인식된다(사용자 07:36 로그).
   - 문제
     - 4K(pt 1500,500)에서 눌러 FHD(pt 2020,300)로 끌면, 범위가 x=3840(4K 오른쪽 끝)에서 멈추고 더 늘지 않는다.
     - 마우스를 놓은 뒤에도 갱신이 없다.
     - 원인 추정: 창 밖으로 나간 포인터 이벤트가 오버레이 웹뷰의 `onpointermove`에 오지 않는다. `setPointerCapture`가 없고, `pointerleave`에서 기다리던 이동을 버린다.
   - 고칠 길
     1. `pointerdown`에서 `setPointerCapture`를 불러 창 밖 이동도 받는다.
     2. 혼합 배율 환산을 고친다.
        - 지금 UI는 자기 모니터 배율로만 물리 px를 만든다(`clientToPhysical`).
        - 맥은 창 밖 좌표가 포인트 단위로 이어진다. 그래서 4K(2배) 밖에 있는 FHD(1배)의 점이 어긋난다. 100pt 넘어가면 약 100 px 어긋난다.
        - 권장: 자기 모니터 밖의 점은 플러그인(맥)에서 `display::cursor(&space())`(실제 커서, 변환 함수 하나)로 바꾼다.
     3. Windows는 창 밖 물리 좌표가 창 DPI에 대해 선형이라 지금 식이 맞을 가능성이 높다. **[확인 필요]** → §3.0-3
   - 완료 기준
     - 4K→FHD, FHD→4K로 끌면 두 모니터에 걸친 범위가 생긴다.
     - 끝점이 커서와 1 px 안에서 맞는다.
     - `swift bench/mac/drag.swift 1500 500 2020 300`으로 자동 확인한다.
   - 함께 볼 것
     - 결과 섬 위치: **고침(2026-11-26)**. 커서가 FHD에 있을 때 섬이 x 1640~2200으로 두 모니터에 걸쳐 떴다(Tauri 물리 좌표가 혼합 배율에서 어긋남). 맥은 `window::place_top_center`로 포인트에 직접 놓는다. 이제 FHD x 2600, 4K x 680으로 각 화면 가운데에 뜬다(창 목록으로 확인)
     - 커서를 다른 모니터로 옮긴 뒤에도 Esc가 먹는지 **[확인 필요]**
     - 전체 화면 앱이 한 모니터에만 있을 때 다른 모니터에서 F1 **[확인 필요]**
3. **권한 스마트 흐름(M10)** — 2026-11-26 사용자 결정: "사용자들이 편하게 승인하도록"
   - 원칙
     - 사용자가 설정 앱에서 길을 찾지 않게 한다.
     - 단추 하나를 누르면 정확한 설정 칸이 열린다. 사용자가 권한을 켜면 앱이 스스로 알아채고 다음 단계로 간다.
   - 흐름
     1. 첫 실행 시트에(또는 권한 없이 F1을 누르면) "화면 기록 켜기" 단추 하나를 둔다.
     2. 단추를 누르면 두 가지를 한다.
        - `CGRequestScreenCaptureAccess()`: 설정 목록에 앱을 올리고 시스템 창을 띄운다.
        - 설정의 정확한 칸을 연다: `x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture`
     3. 앱은 1초마다 `CGPreflightScreenCaptureAccess()`를 본다.
        - 화면 기록은 앱을 다시 시작해야 반영된다(위에서 확인). 켜진 것이 보이면 바로 "다시 시작" 단추를 띄우고, 누르면 앱이 스스로 다시 뜬다.
        - preflight는 다시 시작하기 전까지 false로 남을 수 있다. 설정 칸을 연 뒤에는 "켰으면 다시 시작을 눌러 주세요" 단추를 항상 함께 보인다.
     4. 손쉬운 사용(요소 인식 M9용)은 처음에 묻지 않는다.
        - 요소 인식이 처음 필요한 순간(F1 뒤 커서 이동)에 섬 칩을 띄운다: "요소까지 잡으려면 손쉬운 사용 켜기"
        - 칩을 누르면 `AXIsProcessTrustedWithOptions(prompt)`를 부르고 `?Privacy_Accessibility` 칸을 연다.
        - 손쉬운 사용은 다시 시작하지 않아도 바로 반영된다. 1초 검사에서 켜지는 순간 칩을 닫는다.
     5. 업데이트 뒤 권한이 풀린 것을 알아챈다.
        - 지난 실행에 권한이 있었는데(설정에 기록) 지금 없으면, "업데이트 뒤 권한을 다시 켜야 해요"로 같은 흐름을 탄다.
        - 서명이 고정되면 드문 일이어야 한다(§4).
     6. 맥의 F 키: 처음 F1이 안 먹으면(단축키 이벤트가 없음) 시트에 "fn+F1 또는 표준 기능 키 설정" 안내를 띄운다.
   - 완료 기준: 권한을 끈 상태의 첫 실행부터 단추 → 켜기 → 다시 시작까지, 사용자가 설정 앱을 직접 찾지 않는다. 섬·시트 문구는 `04-screens.md` 말투를 따른다.
4. **자체 서명 배포 준비**(§4 2026-11-26 결정)
   1. 인증서를 만든다: 키체인 접근 › 인증서 지원 › 인증서 생성, 종류 "코드 서명", 자체 서명 루트.
   2. 그 인증서 이름을 `APPLE_SIGNING_IDENTITY`로 넘겨 `tauri build`한다.
   3. 두 번 빌드해 `codesign -dr - <앱>`이 같은지 보고, 업데이트 뒤에도 화면 기록 권한이 남는지 본다. **[확인 필요]**
   4. 받는 사람에게 줄 "그래도 열기" 안내 그림 한 장을 만든다.
5. 그다음 순서: M9(AX 요소) → M3·M4·M5·M6·M8 남은 확인 → X1·X2 → 단계 4.

---

## 3. Windows에서 할 일 (학교 PC)

빌드 환경(학교 PC): C 드라이브가 좁다. 창마다 아래를 먼저 설정한다.
```
$env:CARGO_TARGET_DIR='D:\dev-cache\screen-ink\target'
$env:CARGO_HTTP_CHECK_REVOKE='false'
```

### 3.0 오늘(2026-11-26) 학교에서 할 일 — 순서대로

1. 받기: `git pull --rebase origin feat/foundation`
2. **§3.2 회귀 확인**
   - 맥 커밋이 플러그인·공통·앱 설정을 바꿨다(`61e2450` 이후).
     - `tauri-plugin-screen-ink`: `lib.rs`·`overlay.rs`·`adapters.rs`
     - `ink-app`: `service.rs`·`ports.rs`
     - `ink-domain`: `image.rs`
     - `ink-platform-fake`
     - 앱: `Cargo.toml`(tauri `macos-private-api` 기능), `tauri.conf.json`(`macOSPrivateApi: true`)
   - 특히 두 가지를 본다.
     - Windows 빌드가 `macos-private-api` 기능 때문에 깨지지 않는지
     - 진입 시간이 그대로인지(마지막 기록 60/69 ms)
   - Windows 블록(`#[cfg(windows)]`)은 동작을 바꾸지 않았다.
3. **듀얼 모니터**(학교 PC가 모니터 2대일 때)
   - 두 번째 모니터에서 F1 → 그 모니터가 강조되는지
   - 모니터를 넘어 끌기: 맥은 2026-11-26에 고쳤다(§2.5-2). UI에 `setPointerCapture`가 들어갔으니 Windows에서도 범위가 두 모니터에 걸치는지, 끝점이 커서와 맞는지 본다.
   - 결과 섬이 커서가 있는 모니터에 뜨는지
   - 배율이 다르면(100%+150%) 끝점이 커서와 맞는지
   - 결과는 이 문서에 적는다. Windows도 넘어 끌기가 안 되면, 고칠 곳이 같은 UI(`CaptureOverlay.svelte`의 포인터 처리)이므로 맥과 함께 고친다.
4. 그다음 §3.1 W1~W11. 먼저 W3 측정, W7 사례, W2 혼합 배율.

### 3.1 단계 3의 남은 일 중 Windows 전용

| # | 할 일 | 근거 | 완료 기준 |
|---|---|---|---|
| W1 | **MSAA 대체 경로**(UIA가 비거나 실패하는 창) | 07 3b, 08 §4.2 | UIA 없는 Win32 앱에서도 요소 강조 |
| W2 | **DPI를 모르는 창 좌표 보정**(D12) | 08 §3.3 | 혼합 배율(100%+150%)에서 Qt·LibreOffice 요소가 어긋나지 않음(RB-12) |
| W3 | Chrome **첫 강조 ≤ 진입 + 30 ms**, 커서 이동 질의 ≤ 5 ms 측정 | 07 3b | 측정값을 `bench/results/`에 기록 |
| W4 | **멈춘 시험 앱**(RB-20): 메시지 루프를 막고, `accHitTest`가 돌아오지 않고, 자식을 끝없이 보고하는 앱을 만든다 | 08 §5.1, D17·D19 | UI 굳음 100 ms 초과 0회 |
| W5 | **투명 덮개 시험 창**(RB-16): 클릭 통과 계층 창 · 투명도 0 · 모니터 전체 맨 위 | 07 3a | 덮개가 강조되지 않음(자동) |
| W6 | **Win32 표준 컨트롤 시험 앱**(RB-04) | 08 §5.1 | 컨트롤 사각형으로 자동 정답 |
| W7 | Windows 사례 뜨기(사람 정답): RB-01 Chrome · 02 Edge · 03 WebView2 · 05 탐색기 · 06 설정·계산기 · 07 PPT · 08 워드·엑셀 · 09 한글 · 10 Edge PDF · 11 메뉴·툴팁 · 12 혼합 배율 · 13 작업 관리자(관리자) · 15 최대화·작업 표시줄 · 17 시작 메뉴 · 18 다른 가상 데스크톱 · 19 Firefox | 08 §5.1 | 시험 문서만 띄우고 `SCREEN_INK_CASE_DIR`로 뜸. `baseline.json` 갱신 |
| W8 | **앱 가족 표 갱신**: 학교에서 자주 쓰는 앱(한글·PPT·엑셀·브라우저)을 개인정보 없는 시험 문서로 열어 UIA 실측 | 07 §5 위험표 | 08 §4.6 가족 표에 반영 |
| W9 | **Snipaste 대조 측정**(기본 설정, 창·요소 감지 켬). 판독 코드는 §2.3 X6 | 08 §5.4 | `bench/results/recog-<날짜>.csv`에 나란히 |
| W10 | 관문 판정: G3(V 혼자 RB-01·03 Top-1 ≥ 80%), G4(RB 전체 Top-1 ≥ 90% · Top-3 ≥ 98% · 경계 ≤ 1 px · 굳음 0 · Snipaste 이상) | 07 §3 | 결과 보고서 |
| W11 | 붙여 넣기 확인: 한글·PPT에 Ctrl+V → 크기·색 | 07 3e | 차이 없음 |

### 3.2 맥 변경을 받은 뒤 Windows 회귀 확인 (받을 때마다)

맥 커밋이 공통 크레이트(`ink-domain`·`ink-app`·`ink-vision`·`ink-platform-fake`)나 플러그인·UI를 건드렸으면, 학교 PC에서 아래를 한 번 돌린다.

1. 전체 시험과 clippy
   ```
   cargo test --workspace --exclude screen-ink
   cargo clippy --workspace --all-targets -- -D warnings
   npm test
   npm run check
   ```
2. `npm run app:dev` → F1 → 커서를 4 px 넘게 움직여 요소 강조 → Enter → 결과 섬 확인
3. 진입 시간 회귀 확인. 기준은 p50 ≤ 80 · p95 ≤ 150 ms(마지막 기록 60/69 ms).
   ```
   powershell -ExecutionPolicy Bypass -File bench\m1-overlay.ps1 -Count 100
   ```

### 3.3 단계 4 이후의 Windows 몫

§2.4 표의 오른쪽 열이 전부다. 순서는 07 마스터플랜을 따른다.

---

## 4. 결정 기록 (여기에 이어 적는다)

| 날짜 | 결정 | 이유 |
|---|---|---|
| 2026-11-25 | Windows 정지 화면은 네이티브 창(`freeze.rs`)이 그리고, 맥은 `nativeFrame=false`로 시작 | 웹뷰 BMP 전송 ~160 ms(03 §4) |
| 2026-11-25 | V 엔진은 `ink-app::VisionEngine`(앱은 배경 스레드, 재생은 `blocking`) | 재생 시험이 앱과 같은 코드를 쓰게 |
| 2026-11-25 | CI는 PR·수동 실행만, 순수 크레이트 + UI만(ubuntu) | OS 어댑터는 각 OS 개발 PC에서 |
| 2026-11-25 (맥) | 좌표 변환은 `ink-platform-macos/src/coords.rs`의 `map`·`unmap` 두 함수뿐. 화면 사실값은 CG 전역 포인트(왼쪽 위 원점)로 모으고(`NSScreen.visibleFrame`만 Cocoa → `cocoa_to_cg`), 물리 크기 = 포인트 × `backingScaleFactor`(`CGDisplayMode` 픽셀 폭 ÷ 폭) | Cocoa·CG·물리 세 좌표가 섞이면 어긋남을 찾기 어렵다 |
| 2026-11-25 (맥) | 혼합 배율 배치: 주 화면에서 시작해 **맞닿은 이웃을 맞닿은 채로 민다**(크기 = 그 화면 물리 px, 맞닿은 모서리를 따라 어긋난 만큼은 안쪽 모서리를 가진 화면 배율로 잼). 이웃으로 안 이어지거나 밀어도 겹치면 모든 원점을 `포인트 × 가장 큰 배율`(겹침 없음, 틈 생김). 창 사각형은 가운데가 있는 화면 배율로 잰다. 이 맥미니(4K@2 주 + 오른쪽 FHD@1): `0,0 3840x2160` + `3840,0 1920x1080` | Windows 물리 배치와 같은 성질(겹침 없음·맞닿음 유지). 시험 8개(`coords::tests`) |
| 2026-11-25 (맥) | 오버레이 창 층 1000(`NSScreenSaverWindowLevel`), 섬 1001, 둘 다 모든 Space·전체 화면 앱 위·창 순환 제외. 오버레이는 Tauri 물리 좌표가 아니라 `window::place`로 그 화면 포인트에 직접 놓는다 | 혼합 배율에서 Tauri 물리 좌표(포인트 × 창 배율)는 우리 배치와 다르다 |
| 2026-11-25 (맥) | 캡처 권한 오류는 `ink_app::NO_SCREEN_PERMISSION`으로 시작하는 문자열(포트 모양은 그대로) | 서비스가 일반 실패와 권한 없음을 나눠 안내(M10) |
| 2026-11-25 (맥) | 맥도 정지 화면을 네이티브 창이 그린다(`nativeFrame=true`). 모니터마다 활성화하지 않는 패널(층 999, 웹뷰 오버레이 바로 아래)에 SCK가 준 `CGImage`를 CALayer 내용으로 그대로 붙이고(복사 없음), 50% 어둡게는 그 위 검은 반투명 사각형 4개(강조 둘레)로. 패널을 못 띄우면 웹뷰 경로(`false`)로 돌아간다 | 웹뷰 경로는 이미지 로드·그리기만 65~96 ms라 M1(p50 ≤ 80)을 못 맞춘다. 층 그리기는 시험(`freeze::tests`)으로 방향·구멍 자리 확인 |
| 2026-11-25 (맥) | `RawWindow.class` = 번들 ID(없으면 빈 문자열), `exe` = 실행 파일 이름 소문자. `topmost`·`tool` = 층 > 0, `layered_alpha` = 창 알파 < 1일 때만. WindowServer(메뉴 막대)·Dock 소유 창은 목록에서 뺀다 | 맥에서 앱 가족은 번들 ID가 가장 안정적. 메뉴 막대·Dock은 앱 창이 아니라 화면 장식(M4 완료 기준) |
| 2026-11-26 (맥) | 맥 배포 서명은 **자체 서명 인증서(무료)**로 고정한다. Apple Developer Program(연 99달러)은 사용자가 늘 때 다시 검토한다. 받는 사람은 첫 실행 때 "그래도 열기"를 한 번 누른다 | 비용(사용자 결정). 임시(ad-hoc) 서명은 빌드마다 신원이 바뀌어 화면 기록·손쉬운 사용 권한이 업데이트마다 풀린다 → 고정 서명으로 막는다(실제로 유지되는지는 [확인 필요], §2.5-4) |
| 2026-11-26 (맥) | 권한은 스마트 흐름으로 받는다(§2.5-3): 단추 하나로 요청 + 정확한 설정 칸 열기 + 1초 검사. 화면 기록은 앱이 스스로 다시 시작하고, 손쉬운 사용은 필요한 순간에 묻는다 | 사용자가 설정 앱에서 길을 찾지 않게(사용자 요청) |
| 2026-11-26 | Windows 배포는 **Microsoft Store(MSIX, 스토어가 서명)** 가 기본. 파일 직접 공유는 저장소 공개 뒤 Certum 오픈소스 서명(€49/년) 또는 SignPath(무료)만. Azure Artifact Signing(한국 개인 불가)·일반 OV/EV(비쌈) 기각 | 경고 없음 + 유료 서명 없음(싸면 허용) 조건(사용자). 11-distribution |
| 2026-11-26 | 진척도는 `docs/progress/progress.json` 하나에서 작업일 가중으로 계산(완료 1 · 확인 남음 0.8 · 진행 중 0.5 · 안 함 0) | 두 컴퓨터가 같은 기준으로 % 보고 |
| 2026-11-26 (맥) | 맥 자동 입력 시험은 `bench/mac/keys.swift`·`drag.swift`(CGEvent)로 한다. osascript System Events는 쓰지 않는다 | System Events는 자동화 권한 창이 따로 뜬다. CGEvent는 손쉬운 사용 권한만으로 F1·Esc·끌기를 반복할 수 있다 |
| 2026-11-26 | **저장소를 공개한다**(SignPath 무료 서명용). 공개 전 점검: 비밀값 없음, 작성자 이메일은 이력에 남음, 라이선스 확정 대기(제안 MIT) | 무료 서명 조건(사용자). 11 §10 |
| 2026-11-26 | 개인정보 처리방침·지원·`.appinstaller`는 저장소 GitHub Pages, 주소는 쌤유니버스.com 하위 도메인 CNAME(본 사이트는 건드리지 않음). 첫 `.appinstaller` 릴리스는 도메인 연결 뒤 | `.appinstaller` 주소는 바꿀 수 없음. 11 §10.2 |
| 2026-11-26 | 업데이트 안내는 배너가 아니라 **팝업**. 클릭 한 번에 몇 초 안 설치·자동 재시작, 모드·전체 화면 발표 중엔 미룸. PRD R-NET-1 문장 수정 필요(원래 세션) | Tidy 배너는 사용자가 못 알아챔(사용자 경험). 11 §7.2 |

---

## 5. 파일 지도

| 무엇 | 어디 |
|---|---|
| 판정 로직(순수) | `crates/ink-domain/src/` — `recog.rs`(융합), `selection.rs`(고르기), `snapshot.rs`(창 거르기·기본 강조), `magnet.rs`, `image.rs`(자르기·썸네일·커서 합성) |
| 유스케이스 | `crates/ink-app/src/` — `service.rs`(진입), `capture.rs`(고르기·결과·사례 쓰기), `vision.rs`(V 엔진), `case.rs`(사례 형식), `ports.rs`(계약) |
| V 엔진 | `crates/ink-vision/src/` — `prep.rs`, `ladder.rs`, `flat.rs`, `tests.rs`(성능 측정 포함) |
| Windows 어댑터 | `crates/ink-platform-windows/src/` — `screen.rs`, `uia.rs`, `freeze.rs`, `clipboard.rs`, `events.rs` |
| Tauri 연결 | `crates/tauri-plugin-screen-ink/src/` — `lib.rs`(포트 조립·명령), `overlay.rs`(오버레이·`ink://`), `adapters.rs`(클립보드·파일·시계) |
| 가짜·재생 | `crates/ink-platform-fake/` — `src/replay.rs`, `tests/replay.rs`, `src/bin/recog-replay.rs` |
| UI | `packages/ink-ui/src/lib/screens/CaptureOverlay.svelte`, `components/Island.svelte`, `components/SelectionFrame.svelte` |
| 명세 | `docs/prd/07-plan.md`(마스터플랜), `08-smart-capture.md`(인식), `04-screens.md`(화면), `03-architecture.md`(포트) |
| 사례·측정 | `bench/recog/`, `bench/results/`, `bench/m1-overlay.ps1` |
