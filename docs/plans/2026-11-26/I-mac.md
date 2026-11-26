# I. macOS 어댑터 · 권한 흐름 · 서명 배포 구현 시안 (`M9` · `M10` · `M-sign` · `M3`~`M6` · `M8` · `M11`)

대상 항목(`progress.json`): `M9` AX 요소 색인(1.5일) · `M10` 권한 스마트 흐름(1.0일, 진행 중 → 남은 0.5일) · `M-sign` 자체 서명 인증서·dmg 빌드·업데이트 후 권한 유지 확인(0.5일, 진행 중 → 남은 0.25일) · 확인 남음 `M3` 커서 · `M4` 창 목록 · `M5` 알림 덮개(D23) · `M6` 클립보드 · `M8` 캡처 제외 · `M11` 시스템 사건·지역 시각.
근거 문서: [docs/handoff/2026-11-25-mac-windows.md](../../handoff/2026-11-25-mac-windows.md) §2.

---

## 1. 목표 · 범위 (요구사항 ID)

| 요구사항 / 항목 | 우선 | 요약 |
|---|---|---|
| `M9` (`R-CAP-4·19·20`) | P0 | macOS Accessibility(`AXUIElement`) 기반 `ElementSource` 색인 일꾼 — Safari·Finder 등 위에서 요소 강조, UI 스레드 대기 0, 500 ms×2 느린 앱 자동 강등 |
| `M10` (`R-FIRST-2`) | P0 | **권한 스마트 흐름** (`2026-11-26` 사용자 결정): 사용자가 시스템 설정을 찾아다니지 않고 단추 1회로 요청 + 정확한 설정 칸 열기 + 1초 폴링 + 자동 재시작(화면 기록) / 필요 순간 칩(손쉬운 사용) + 업데이트 후 권한 풀림 감지 |
| `M-sign` | P0 | 무료 자체 서명 코드 서명 인증서로 고정 서명 `.app`/`.dmg` 빌드 → 재빌드·업데이트 뒤에도 화면 기록·손쉬운 사용 권한 유지 검증 + `"그래도 열기"` 안내 그림 1장 |
| `M3`~`M6`·`M8`·`M11` | P0 | 이미 코드가 들어간 맥 어댑터 6종의 실기 확인 완료 및 누락 발견 시 수정 |

---

## 2. 정확한 형식

### 2.1 `M9` AX 요소 색인 (`crates/ink-platform-macos/src/ax.rs`)

- **포트 계약 (`ink_app::ElementSource`, Windows `uia.rs`와 1:1 동일)**:
  - `request(&self, window: u64, exe: &str)`: `(window: CGWindowID, pid)`로 전용 일꾼 스레드에 작업 큐잉.
  - `elements_at` / `elements` / `rects`: 메모리 `HashMap<u64, Vec<ElementCand>>`에서만 즉시 조회(0 블로킹).
  - `clear(&self)`: `gen: AtomicU64` 증가 + 캐시/대기열 비우기.
- **색인 알고리즘 (`ax.rs`)**:
  1. 손쉬운 사용 권한 확인: `AXIsProcessTrusted()`가 `false`이면 즉시 빈 결과 반환 + 권한 필요 신호(`on_ax_untrusted` 콜백 1회 → `M10` 섬 칩).
  2. `CGWindowID → pid` 매핑은 `RawWindow.pid`(또는 `windows::pid_of(window)`)에서 얻는다.
  3. `AXUIElementCreateApplication(pid)` 생성 → `AXUIElementSetMessagingTimeout(app_ax, 0.4)` (400 ms, D17 멈춤 방지).
  4. `kAXWindowsAttribute`에서 대상 `CGWindowID`와 일치하는 창 찾기 (`_AXUIElementGetWindow` 사설 심볼 대신 **창 위치·크기 매칭**(CG 포인트 `±2 pt` 이내)을 1안으로 써 AppKit/OS 버전 의존 최소화; 겹치면 앞쪽 창).
  5. BFS로 하위 트리 순회: `kAXChildrenAttribute` + `kAXRoleAttribute` + `kAXPositionAttribute` + `kAXSizeAttribute`.
     - **상한**: 깊이 `MAX_DEPTH = 8`, 방문 상한 `MAX_VISIT = 4000`, 저장 상한 `KEEP_ELEMENTS = 1500`, 창당 총 예산 `250 ms`(예산 초과 시 그 시점까지 모은 결과 반환).
  6. **좌표 변환**: AX의 `AXPosition`·`AXSize`는 CG 전역 포인트(왼쪽 위 원점)다 → 반드시 `crates/ink-platform-macos/src/coords.rs`의 `space.map_rect(...)` 한 함수만 거쳐 계약 물리 px `PhysRect`로 변환.
  7. **역할 매핑 (`crates/ink-domain/src/recog.rs` 추가)**:
     ```rust
     impl Control {
         /// macOS AXRole 문자열 → Control (순수 함수 + 단위 시험)
         pub fn from_ax(role: &str) -> Control {
             match role {
                 "AXButton" | "AXPopUpButton" | "AXMenuButton" => Control::Button,
                 "AXTextField" | "AXTextArea" | "AXSearchField" => Control::Edit,
                 "AXStaticText" => Control::Text,
                 "AXImage" => Control::Image,
                 "AXList" => Control::List,
                 "AXTable" | "AXOutline" => Control::Table,
                 "AXRow" | "AXCell" => Control::DataItem,
                 "AXTabGroup" => Control::Tab,
                 "AXRadioButton" => Control::TabItem, // AXSubrole=="AXTabButton" 우선 체크
                 "AXMenu" => Control::Menu,
                 "AXMenuItem" => Control::MenuItem,
                 "AXToolbar" => Control::ToolBar,
                 "AXWebArea" | "AXDocument" => Control::Document,
                 "AXLink" => Control::Hyperlink,
                 "AXCheckBox" => Control::CheckBox,
                 "AXComboBox" => Control::ComboBox,
                 "AXGroup" | "AXScrollArea" => Control::Group,
                 "AXSplitGroup" | "AXSheet" => Control::Pane,
                 _ => Control::Other,
             }
         }
     }
     ```
- **플러그인 연결 (`crates/tauri-plugin-screen-ink/src/lib.rs`)**:
  - `element_source()`에 `#[cfg(target_os = "macos")]` 분기 추가 → `ink_platform_macos::ax::AxIndex::spawn(...)`.

### 2.2 `M10` 권한 스마트 흐름 (`2026-11-25-mac-windows.md §2.5-3`)

#### Rust 상태 · 설정 · 명령

```rust
// crates/ink-app/src/state.rs — InkState에 맥 권한 상태 추가(Windows는 항상 true/None)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MacPermissions {
    pub screen_recording: bool,
    pub accessibility: bool,
    /// 이전 실행에는 권한이 있었는데 지금 없어짐(업데이트 후 권한 풀림 감지)
    pub screen_revoked_after_update: bool,
    pub ax_revoked_after_update: bool,
}
```
- **설정 키 (`crates/ink-domain/src/settings.rs` 추가)**:
  - `mac.hadScreenPerm` (`Bool`, 기본 `false`)
  - `mac.hadAxPerm` (`Bool`, 기본 `false`)
  - 시작 시 `prev == true && now == false`이면 `*_revoked_after_update = true`로 세팅 → `"업데이트 뒤 권한을 다시 켜야 해요"` 문구 자동 선택.
- **macOS 어댑터 (`crates/ink-platform-macos/src/permissions.rs`, 신규)**:
  - `preflight_screen() -> bool` (`CGPreflightScreenCaptureAccess`)
  - `request_screen_and_open_settings()`: ① `CGRequestScreenCaptureAccess()` ② `open "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"`
  - `preflight_ax() -> bool` (`AXIsProcessTrusted`)
  - `request_ax_and_open_settings()`: ① `AXIsProcessTrustedWithOptions({ kAXTrustedCheckOptionPrompt: true })` ② `open "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"`
- **Tauri 명령 & `InkAction` 확장**:
  - `'mac.perm.screen'` → 요청 + 설정 칸 열기 + 1초 폴링 시작(권한 감지 또는 사용자 확인 시 `[다시 시작]` 활성)
  - `'mac.perm.ax'` → 요청 + 손쉬운 사용 칸 열기 + 1초 폴링(켜지는 순간 칩 자동 닫기, 재시작 불필요)
  - `'app.relaunch'` → `app.restart()` (화면 기록 권한 반영용 자동 재시작)
- **화면 문구 (`04` 말투)**:
  - 첫 실행 시트 / 화면 기록 없음: `[화면 기록 켜기]` · 연 뒤 보조 단추 `[켰으면 다시 시작]` · 업데이트 후 풀림: `"업데이트 뒤 화면 기록 권한을 다시 켜야 해요"`.
  - 손쉬운 사용 없음(처음 F1 커서 이동 시 1회 섬 칩): `"요소까지 잡으려면 손쉬운 사용 켜기"` (누르면 `mac.perm.ax`, 거절해도 W·V 근거로 정상 캡처).
  - 맥 F키 안내: 첫 실행 시트에 `"맥 키보드에서 F1이 안 눌리면 fn+F1 또는 키보드 설정의 표준 기능 키를 켜세요"`.

### 2.3 `M-sign` 자체 서명 인증서 · dmg 빌드

- **스크립트**: `scripts/mac-build-dmg.sh` (신규).
  1. `APPLE_SIGNING_IDENTITY` 환경변수 확인(없으면 키체인 인증서 생성 안내 출력 후 중단).
  2. `npm run app:build -- --config apps/screen-ink/src-tauri/tauri.macos.conf.json` 실행.
  3. 산출 `.app`의 `codesign -dr - <app>` designated requirement 출력·기록.
  4. 두 번째 빌드(버전 +1 패치) 후 동일 `codesign -dr -` 일치 확인 + 기존 설치 `.app` 덮어쓰기 뒤 화면 기록·손쉬운 사용 권한 유지 실기 확인.
- **사용자 안내 그림**: `docs/handoff/mac-first-open-guide.md`(또는 `site/support/`용 그림 1장) — 우클릭 → 열기 / 시스템 설정 › 개인정보 보호 및 보안 › `"확인 없이 열기"` 2단계.

### 2.4 확인 남음 6종 (`M3`~`M6` · `M8` · `M11`) 체크 형식

| ID | 실기 확인 절차 | 합격 기준 |
|---|---|---|
| `M3` | F1 → `` ` `` 키로 커서 포함 켜기 → 클릭 확정 → `Ctrl+V` | 결과 이미지에 macOS 커서 스프라이트가 핫스팟 오차 ≤ 1 px로 합성됨 |
| `M4` | Finder·Safari 창 위에서 F2 | 커서 아래 창만 강조, 메뉴 막대·Dock이 창 후보로 잡히지 않음 |
| `M5` | macOS 알림 배너 띄운 상태에서 F1 | 알림 덮개가 강조되지 않고 아래 창/요소가 잡힘 (실패 시 `filter_windows`에 D23 규칙 추가) |
| `M6` | F1 → Enter → 미리보기(`⌘N`)·Keynote·Pages에 `⌘V` | 크기·색(sRGB) 일치, 투명/깨짐 없음 |
| `M8` | 섬을 띄운 채 시스템 스크린숏(`⇧⌘3`) 촬영 | 결과 스크린숏에 섬·팝오버 0 px |
| `M11` | 외장 모니터 뽑기·꽂기 + 파일 저장 이름 시각 확인 | 재시작 없이 오버레이 재정렬, 파일 이름 시각이 KST 지역 시각 |

---

## 3. 알고리즘 · 결정 (대안과 고른 이유)

- **AX 창 매칭에 비공개 심볼 `_AXUIElementGetWindow`를 피하는 이유**: macOS 업데이트(특히 macOS 27 등 최신판)에서 사설 C 심볼 변경 위험이 있다. `CGWindowList`의 창 경계(CG pt)와 `AXWindow`의 `AXPosition`/`AXSize`를 비교하면 공개 API만으로 99% 정확히 매칭된다(동일 PID·동일 경계 창이 2개일 때만 `kAXMainAttribute` 우선).
- **손쉬운 사용(AX) 지연 요청**: 첫 실행부터 두 권한을 한꺼번에 요구하면 사용자 이탈이 크다. 화면 기록만 먼저 받고, 손쉬운 사용은 요소 인식이 실제로 필요한 순간 섬 칩으로 부드럽게 유도한다(`2026-11-26` 사용자 결정).
- **자체 서명 유지**: Apple Developer Program($99/년) 비용을 피하면서도 임시(ad-hoc) 서명의 권한 초기화 문제를 막기 위해 고정 자체 서명 인증서를 쓴다.

---

## 4. 시험 · 합격 기준

| 시험 | 이름 예시 | 합격 기준 |
|---|---|---|
| Rust 단위 (`ink-domain::recog`) | `ax_역할문자열_Control_매핑_전수` | 20종 AXRole → `Control`, 알 수 없는 역할 → `Other` |
| Rust 단위 (`ink-platform-macos`) | `ax_사각형_혼합배율_좌표변환_일치` | Retina 2× + FHD 1×에서 `coords::map_rect` 오차 0 px |
| Rust 단위 (`ink-app`) | `맥_권한_업데이트후_풀림감지_및_칩1회표시` | `hadScreenPerm=true` + `now=false` → 복구 문구, AX 미허용 시 칩 1회만 |
| 맥 실기 (`M9`) | `capture.evidence=true`로 Safari·Finder 위 F1 | 초록 `A` 근거 마디 표시, 이동 질의 ≤ 5 ms, UI 굳음 0, 500 ms 초과 앱 자동 강등 |
| 맥 실기 (`M10`) | `tccutil reset ScreenCapture` 후 첫 실행 → 단추 → 켜기 → 재시작 | 사용자가 설정 앱을 직접 탐색하지 않고 완료 |
| 맥 실기 (`M-sign`) | dmg 1차 설치·권한 부여 → 2차 빌드 덮어쓰기 | `codesign -dr -` 동일, 화면 기록 권한 유지 |
| 맥 실기 (`M3`~`M6`·`M8`·`M11`) | §2.4 6항목 전부 실행 | 6/6 통과 + `docs/handoff/2026-11-25-mac-windows.md` 갱신 |

---

## 5. 위험 · 대응

| 위험 | 발생 신호 | 대응 |
|---|---|---|
| **자체 서명 덮어쓰기 뒤 macOS TCC가 화면 기록 권한을 그래도 초기화** | `M-sign` 2차 빌드 후 권한 풀림 | `M10`의 "업데이트 뒤 권한 풀림 감지"가 즉시 안내하도록 안전망 유지 + 장기 사용자 확대 시 Developer ID($99) 재검토 |
| **Safari/Chrome 웹 콘텐츠가 AX 트리를 비워 둠** | `M9`에서 탭·주소창만 `A`로 잡힘 | 정상 동작(`08 §4.6`) — 웹 본문은 기존 V 엔진(`ink-vision`)이 주 근거로 처리 |
| **개발 중(`npm run app:dev`) 권한이 터미널/Claude Code에 붙음** | dev 테스트와 `.app` 번들 테스트 결과 불일치 | `M10` 최종 검증은 반드시 빌드된 `.app` 번들을 Finder에서 직접 실행해 확인 |
| **혼합 배율에서 캡처 핀(`3h-pin`) 위치 어긋남 (`handoff §2.6`)** | 4K+FHD에서 핀 창이 엉뚱한 곳에 뜸 | `pins.rs`의 `#[cfg(target_os = "macos")]` 분기에서 `window::place`로 포인트 직접 배치(오버레이와 동일) |

---

## 6. 작업일 · 난이도 · 먼저 끝나야 할 것

| 항목 | 작업일(남은 일) | 난이도 | 선행 |
|---|---:|---|---|
| `M10` 권한 스마트 흐름 | 1.00 (0.50) | 중 | M2·M7 (끝남) |
| `M9` AX 요소 색인 | 1.50 (1.50) | 상 | M1·M4 (끝남), `M10` 연계 |
| `M-sign` 자체 서명 dmg · 권한 유지 | 0.50 (0.25) | 중 | `M10` |
| `M3`·`M4`·`M5`·`M6`·`M8`·`M11` 확인 | 2.00 (0.40) | 하 | 맥 실기 |
| **묶음 합계** | **5.00일 (2.65일)** | **상** | Windows 작업과 완전 병렬 가능 |
