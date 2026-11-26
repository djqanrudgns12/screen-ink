# C. 단계 3 인식·검증 마무리 구현 시안 (`3b-dpi` · `3b-msaa` · `3a-rb1516` · `3f-*` · `3c-g3` · `3a-ci` · `3e-paste`)

대상 항목(`progress.json`): `3b-dpi` DPI 모르는 창 좌표 보정(D12) · `3b-msaa` MSAA 대체 경로 · `3a-rb1516` RB-15·16 자동 통과 · `3f-docs` 시험 문서 RB-14·Win32 시험 앱 RB-04 · `3f-frozen` 멈춘 시험 앱 RB-20 · `3f-win-cases` Windows 사례 뜨기(RB 17종)·기준선 · `3f-snipaste` Snipaste 대조 하네스·측정·G4 보고서 · `3c-g3` G3 판정 · `3a-ci` 재생 시험 CI 상시 실행 · `3e-paste` 한글·PPT 붙여 넣기 확인.

---

## 1. 목표 · 범위 (요구사항 ID)

| 요구사항 | 우선 | 요약 |
|---|---|---|
| `R-CAP-4` | P0 | UIA가 비거나 마감을 넘긴 구형 Win32/Qt 앱에서 **MSAA 대체 경로(M 근거)** 동작 |
| `R-CAP-17`·`D12` | P0 | DPI-unaware / System-aware 창이 돌려주는 가상화 좌표를 **물리 px로 보정** (`LogicalToPhysicalPointForPerMonitorDPI`) |
| `R-CAP-21`·`D1`·`D4` | P0 | RB-15(최대화·작업 표시줄·바탕 화면)·RB-16(투명 덮개 3종) 자동 통과 |
| `R-CAP-18`·**G3** | P0 | V 엔진 혼자 RB-01(Chrome)·RB-03(WebView2) **Top-1 ≥ 80%** 통과 |
| `R-CAP-19`·`D17` | P0 | 멈춘 시험 앱 **RB-20** 위에서 UI 멈춤 100 ms 초과 **0회** |
| `R-CAP-23`·**G4** | P0 | RB-01~20 전체 시험대 + Snipaste 2.11.3 대조 하네스: **Top-1 ≥ 90% · Top-3 ≥ 98% · 경계 ≤ 1 px · Snipaste 이상 + Snipaste 실패 점 ≥ 70% 맞춤** |
| `R-CAP-7` | P0 | 클립보드(DIB+PNG)를 한글(HWP)·PowerPoint에 붙여 넣어 크기·색 오차 없음 확인 |

---

## 2. 정확한 형식

### 2.1 `3b-dpi` DPI 모르는 창 좌표 보정 (D12)

- **위치**: `crates/ink-platform-windows/src/dpi.rs` (신규) + `uia.rs` / `msaa.rs` 호출.
- **Win32 API**:
  - `GetWindowDpiAwarenessContext(hwnd)` → `GetAwarenessFromDpiAwarenessContext(ctx)`
  - `DPI_AWARENESS_UNAWARE` (0) 또는 `DPI_AWARENESS_SYSTEM_AWARE` (1)일 때만 보정 대상 (`DPI_AWARENESS_PER_MONITOR_AWARE` (2)는 이미 물리 px).
  - 보정 함수:
    ```rust
    /// DPI 가상화된 사각형(MSAA/구형 UIA)을 물리 px로 변환.
    /// 1순위: `LogicalToPhysicalPointForPerMonitorDPI(hwnd, &mut pt)` (좌상단·우하단 두 점).
    /// 2순위(API 실패 시 순수 폴백): 창의 모니터 scale / system_scale 비율 적용.
    pub fn to_physical_rect(hwnd: u64, rect: PhysRect, awareness: DpiAwareness, win_monitor_scale: f64, sys_scale: f64) -> PhysRect
    ```
- **도메인 순수 함수**: 폴백 산술 `scale_unaware_rect(rect, win_origin, scale_ratio)`는 `ink-domain::geometry`에 두고 단위 시험.
- **검증 조건**: 보정 후 사각형이 DWM 창 경계(`RawWindow.rect`) 밖으로 2 px 넘게 벗어나면 기존 `recog::build_chain`의 창 경계 클립·검증이 안전망으로 걸러 낸다.

### 2.2 `3b-msaa` MSAA 대체 경로 (M 근거)

- **위치**: `crates/ink-platform-windows/src/msaa.rs` (신규), `uia.rs`에서 호출, `ink-domain/src/recog.rs`에 `Control::from_msaa(role: i32)` 추가.
- **호출 조건 (`08 §4.2 M`)**:
  - UIA `index_window`가 **빈 목록(`len() <= 1`)**이거나 **실패/시간 초과**를 반환한 창에 한해, `UiaIndex` 일꾼 스레드에서 MSAA 색인을 시도한다(커서 이동 시에는 여전히 메모리만 조회 → D18 준수).
  - 진입 시 커서 아래 지점 `p`에 대해 **즉시 1회** `AccessibleObjectFromPoint` + 부모 체인(`get_accParent`, 최대 깊이 8)으로 빠른 M 후보를 채우고, 예산 20 ms 안에 자식 탐색(`AccessibleChildren`, 깊이 4, 상한 300개)을 수행한다.
- **데이터 모양**:
  ```rust
  // crates/ink-domain/src/recog.rs
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct ElementCand {
      pub rect: PhysRect,
      pub control: Control,
      /// true = MSAA(M 근거), false(기본) = UIA(A 근거). 기존 JSON 사례 호환 위해 #[serde(default)]
      #[serde(default)]
      pub from_msaa: bool,
  }
  ```
  `recog.rs`에서 `from_msaa == true`이면 근거 비트를 `src::M`으로 표기해 근거 보기(`capture.evidence`)에서 청록(`M`)으로 구분된다.
- **MSAA 역할 매핑 (`Control::from_msaa`)**: `ROLE_SYSTEM_PUSHBUTTON(43) → Button`, `TEXT(42) → Edit`, `STATICTEXT(41) → Text`, `GRAPHIC(40) → Image`, `LIST(33) → List`, `LISTITEM(34) → ListItem`, `TABLE(24) → Table`, `ROW(28)/CELL(29) → DataItem`, `PAGETAB(37) → TabItem`, `MENUITEM(12) → MenuItem`, `LINK(30) → Hyperlink`, `TITLEBAR(1) → TitleBar`, `PANE(16)/GROUPING(20) → Pane/Group`.
- **멈춤 방지 (D17)**:
  - MSAA 호출 전 `responsive(hwnd)` (`SendMessageTimeoutW(SMTO_ABORTIFHUNG, 50 ms)`) 필수.
  - MSAA 일꾼이 마감 시간 **100 ms**를 넘기면 호출 측 채널 대기를 끊고, **새 일꾼 스레드를 스폰**해 교체한다(기존 멈춘 스레드는 앱 종료 시 정리 — `R-CAP-19`).

### 2.3 시험 앱 · 시험 문서 · 자동 하네스 (`3a-rb1516`, `3f-docs`, `3f-frozen`, `3f-snipaste`)

| 항목 | 파일 위치 | 형식 · 동작 |
|---|---|---|
| `3a-rb1516` | `crates/ink-platform-windows/tests/rb15_16.rs` (또는 바이너리 `bench/recog/tools/rb-win-fixtures`) | ① **RB-16**: 투명 덮개 창 3종(`WS_EX_TRANSPARENT`, `WS_EX_LAYERED` alpha=0, 모니터 전체 `WS_EX_TOPMOST \| WS_EX_LAYERED` alpha=1)을 띄우고 아래에 시험 창을 둔 뒤 `top_windows()` + `filter_windows()` 실행 → 덮개 3종이 `window_at(center, true)`에 0건 잡힘 확인. ② **RB-15**: 최대화 창이 작업 영역(`m.work`)으로 잘리고, 전체 화면 창은 `m.bounds` 유지됨을 실제 Win32 창으로 검증 |
| `3f-docs` RB-14 | `bench/recog/pages/rb14.html` | WebGL/2D Canvas(버튼 모양 도형·차트 카드·영상 프레임 16:9 영역) + 페이지 로드 시 `window.__rb14Truth()`가 CSS px 사각형을 콘솔/JSON으로 내보내고 `recog-truth`가 `Case.points` 자동 생성 |
| `3f-docs` RB-04 | `bench/recog/tools/rb04-win32/` (작은 순수 Rust `windows-sys` 바이너리) | Win32 표준 컨트롤(Button, Edit, ListView, Tab, TreeView, GroupBox, Static) 창을 띄우고 각 자식 컨트롤의 물리 `GetWindowRect`를 JSON(`rb04-truth.json`)으로 자동 출력 |
| `3f-frozen` RB-20 | `bench/recog/tools/rb20-frozen/` (작은 Rust 바이너리) | 인자 `--mode hung \| infinite-msaa`: ① `hung`: 창 생성 뒤 메시지 루프를 10초 차단 ② `infinite-msaa`: `WM_GETOBJECT`에 커스텀 `IAccessible`을 돌려주며 `accHitTest`가 무한 대기하고 자식을 무한 보고 → screen-ink F1 진입·커서 이동 중 UI 프레임 간격 `> 100 ms` 0회 확인 |
| `3f-snipaste` | ① 순수 판독: `crates/ink-vision/src/diff_rect.rs` ② 실행 스크립트: `bench/recog/compare-snipaste.ps1` | ① `detect_snipaste_highlight(before_bgra, after_bgra, w, h) -> Option<PhysRect>`: Snipaste F1 전후 화면을 비교해 어두워지지 않은(화소 차이 `≤ 3`) 최대 직사각형 바운딩 박스를 찾는 순수 함수 + 단위 시험 ② PS1 스크립트가 동일 `(x, y)` 점 목록에 대해 Snipaste F1과 screen-ink 재생 결과를 `bench/results/recog-<날짜>.csv`로 나란히 기록 |
| `3a-ci` | `.github/workflows/ci.yml` | `on:`에 `push: branches: [feat/foundation]` 추가 + `recog-replay` 실행 후 기준선(`baseline.json`) 대비 하락 시 워크플로 실패(exit 1) |

### 2.4 `3f-win-cases` · `3c-g3` · `3e-paste` 절차 형식

- **`3f-win-cases`**: 모든 모니터에 시험 문서(RB-01~19 중 Windows 17종, 개인정보 0)만 띄우고 `SCREEN_INK_CASE_DIR=bench/recog/cases`로 사례 뜨기 → `bench/recog/cases/baseline.json` 갱신 → `recog-replay` CSV 저장.
- **`3c-g3`**: `recog-replay --vision-only --filter RB-01,RB-03` 플래그를 `crates/ink-platform-fake/src/bin/recog-replay.rs`에 추가해 V 엔진 단독 Top-1 측정 → `≥ 80%` 판정(관문 G3).
- **`3e-paste`**: F1로 시험 패턴(격자 + `#FF9F0A` / `#2F6FED` / `#2E9E6A` 색 패치, 정확히 400×200 px) 캡처 → 한글(HWP)과 PowerPoint에 `Ctrl+V` → 다시 PNG 내보내기/화면 확인으로 크기 비율·색 차이 0 확인 후 `bench/results/2026-11-xx-paste-check.md` 기록.

---

## 3. 알고리즘 · 결정 (대안과 고른 이유)

- **MSAA는 UIA 실패 시에만 + 일꾼 격리**: MSAA는 구형 COM 프록시가 대상 앱 UI 스레드와 동기 통신해 멈춤 위험이 가장 크다(Snipaste #116). 따라서 UIA 성공 창에서는 아예 호출하지 않고, 호출 시에도 별도 일꾼 + 100 ms 마감 교체로 메인 스레드와 UIA 일꾼을 모두 보호한다.
- **Snipaste 대조 판독을 순수 Rust 함수(`ink-vision::diff_rect`)로 분리**: PowerShell 픽셀 루프는 느리고 테스트가 어렵다. 순수 Rust 함수로 두면 맥·리눅스 CI에서도 합성 이미지로 판독 정확도를 단위 시험할 수 있다.
- **V 단독 G3 미달 시 대응(`07 §3`)**: V 문턱값(`grad_thresh`, `min_run=24`, `support=0.6`)을 재생 시험으로 최대 1일 튜닝한다. 그래도 80% 미달이면 V를 경계 맞춤 전용으로 내리고 웹/WebView2 목표를 75%로 재조정(관문 규칙).

---

## 4. 시험 · 합격 기준

| 시험 / 측정 | 이름 예시 | 합격 기준 |
|---|---|---|
| DPI 보정 단위 | `dpi_unaware_150퍼센트_좌표_물리변환` | 논리 `(100,100,200,100)` → 물리 `(150,150,300,150)` 오차 0 px |
| MSAA 매핑 단위 | `msaa_역할코드_Control_매핑_및_근거비트_M` | 주요 역할 12종 변환, `sources & src::M != 0` |
| RB-15·16 자동 | `rb15_최대화_작업영역_rb16_투명덮개_3종_제외` | 투명 덮개 강조 0건, 최대화 창 = `m.work` |
| RB-20 멈춤 시험 | `rb20_멈춘앱_UI_굳음_0회` | 진입·커서 이동 50회 동안 UI 프레임 지연 > 100 ms **0회** |
| Snipaste 판독 단위 | `diff_rect_어두운덮개_밖_강조사각형_추출` | 합성 전/후 프레임에서 사각형 오차 0 px |
| **관문 G3** | `recog-replay --vision-only --filter RB-01,RB-03` | Top-1 **≥ 80%**, V 전처리 4K **≤ 40 ms** |
| **관문 G4** | `recog-replay` + `compare-snipaste.ps1` | RB 전체 Top-1 **≥ 90%**, Top-3 **≥ 98%**, 경계 **≤ 1 px**, Snipaste 이상 & Snipaste 실패 점 **≥ 70%** 맞춤 |
| CI | `.github/workflows/ci.yml` on `push` | 푸시마다 `cargo test` + `recog-replay` 기준선 통과 |

---

## 5. 위험 · 대응

| 위험 | 발생 신호 | 대응 |
|---|---|---|
| **MSAA 무한 대기 스레드 누수** | RB-20 반복 시 스레드 수 증가 | 동일 `hwnd`가 1회 시간 초과하면 해당 세션 동안 재질의 금지 + 2회 초과 시 `windowOnly` 자동 강등(`R-CAP-15`), 대기 중 포기한 일꾼은 최대 4개 상한 |
| **학교 PC에서 시험 앱 빌드/실행 시 백신 경고** | `rb04`·`rb20` 로컬 실행 차단 | `target/debug` 로컬 실행 + 시험용 바이너리는 릴리스 패키지에 미포함(`bench/recog/tools/` 별도) |
| **한글(HWP) 클립보드 DIB 상하 반전/색 왜곡** | `3e-paste`에서 이미지 뒤집힘 또는 투명 검은색 | `CF_DIB`의 `biHeight` 부호(양수 bottom-up 표준)와 알파 없는 24/32bpp RGB 확인(`ink-platform-windows/src/clipboard.rs` 점검) |
| **개인정보가 사례 PNG에 찍힘** | 작업 표시줄·배경 창에 메신저/이름 노출 | 사례 뜨기 전 모든 모니터를 시험 문서로 덮고, 작업 표시줄 알림 끄기, 커밋 전 사례 PNG 눈 검사 필수 |

---

## 6. 작업일 · 난이도 · 먼저 끝나야 할 것

| 항목 | 작업일(남은 일) | 난이도 | 선행 |
|---|---:|---|---|
| `3e-paste` 한글·PPT 붙여 넣기 확인 | 0.05 (0.05) | 하 | 없음 |
| `3b-dpi` DPI 모르는 창 보정(D12) | 0.20 (0.20) | 상 | 없음 |
| `3b-msaa` MSAA 대체 경로 | 0.30 (0.30) | 상 | `3b-dpi` |
| `3a-rb1516` RB-15·16 자동 시험 | 0.20 (0.20) | 중 | 없음 |
| `3f-docs` RB-14 + Win32 RB-04 | 0.50 (0.25) | 중 | 없음 |
| `3f-frozen` 멈춘 시험 앱 RB-20 | 0.20 (0.20) | 중 | `3b-msaa` |
| `3f-win-cases` Windows 사례 17종 + 기준선 | 0.40 (0.40) | 중 | `3f-docs`·`3b-msaa`·`3b-dpi` |
| `3c-g3` G3 V 단독 판정 | 0.30 (0.30) | 중 | `3f-win-cases` |
| `3f-snipaste` Snipaste 대조·G4 보고서 | 0.40 (0.40) | 중 | `3f-win-cases` |
| `3a-ci` 재생 시험 CI 상시 실행 | 0.10 (0.05) | 하 | `3f-win-cases`(기준선 고정) |
| **묶음 합계** | **2.65일 (2.35일)** | **상** | 단계 3a~3e 완료(끝남) |
