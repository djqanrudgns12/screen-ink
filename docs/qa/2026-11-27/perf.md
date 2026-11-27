# 2차 고도화 성능 예산 측정 보고서 (`R2-PERF-1`)

- **기준 문서**: [docs/plans/2026-11-27/PRD-2차-고도화.md](../../plans/2026-11-27/PRD-2차-고도화.md) §7 (`R2-PERF-1`) 및 1차 [docs/PRD.md](../../PRD.md) §5
- **측정 일시**: 2026-11-27 16:02~16:03 KST
- **측정 스크립트**: [scripts/qa-snapshots.mjs](../../../scripts/qa-snapshots.mjs) (`--only=perf`로 단독 재실행 가능)
- **측정 환경**:
  - 호스트: Apple Silicon Mac mini (macOS), `resource-guard --profile test` 단일 작업자(`--workers=1`)
  - 브라우저: `chromium-1248` (`@playwright/test@1.64.1`, headless; Windows WebView2 엔진 계열 근사)
  - WebKit(`webkit-2370`, macOS WKWebView 계열): 1회 시도 중 초기 내비게이션 컨텍스트 재초기화 오류(`Execution context was destroyed, most likely because of a navigation`)로 **미실행(측정 미완료)** — 아래 §4에 기록
  - 대상 서버: `@screen-ink/ui` Vite dev 서버 (`127.0.0.1` 로컬 루프백, `FakeInkClient`)
  - 계측 방식: Playwright IPC 왕복 지연을 배제하기 위해 브라우저 내부 `page.evaluate` 안에서 `performance.now()` · `PerformanceResourceTiming` · `document.fonts.ready` · `MutationObserver` + `requestAnimationFrame`으로 직접 측정

---

## 1. 2차 성능 예산 대조표 (`R2-PERF-1`)

| 항목 | PRD 예산 | 측정 조건 | DOM 준비 (`p50 / max`) | 첫 프레임·표시 (`p50 / p95 / max`) | 판정 | 비고 |
|---|---:|---|---:|---:|---|---|
| **글꼴 첫 표시 지연** (`font-display: block`) | **≤ 30 ms** | Cold (새 컨텍스트 캐시 비움, `n=5`) | `.woff2` 로드 max `12.1 / 12.4 ms` | **`17.0 / 17.4 / 17.4 ms`** *(FCP → `fonts.ready`)* | **통과** | 6개 face 모두 `display: block` 확인 |
| **글꼴 첫 표시 지연** (웜 캐시) | **≤ 30 ms** | Warm (같은 컨텍스트 캐시 유지, `n=5`) | `.woff2` 로드 max `0.8 / 2.7 ms` | **`4.3 / 18.4 / 18.4 ms`** | **통과** | 로컬 번들/창 재열기 근사 |
| **색·굵기 패널 열기** | **≤ 16 ms** | Cold 첫 열기 (페이지당 첫 1회, `n=5`) | **`3.9 / 6.3 ms`** | **`7.8 / 8.8 / 8.8 ms`** *(클릭 → `rAF1`)* | **통과** | `rAF2`(2프레임 합성) p50 `24.6 ms` |
| **색·굵기 패널 열기** (반복) | **≤ 16 ms** | Warm 재열기 (같은 페이지, `n=10`) | **`1.8 / 3.5 ms`** | **`10.9 / 11.8 / 11.8 ms`** *(클릭 → `rAF1`)* | **통과** | `rAF2` p50 `27.2 ms` |
| **글꼴 패널 첫 열기** | **≤ 120 ms** | Cold 첫 열기 (페이지당 첫 1회, `n=5`) | **`1.6 / 3.0 ms`** *(`.ink-fp[data-ready]`)* | **`8.0 / 8.7 / 8.7 ms`** *(클릭 → `rAF1`)* | **통과 (단서 있음)** | 아래 §3 단서 참고 (`systemFonts` 미로드) |
| **글꼴 패널 열기** (반복) | **≤ 120 ms** | Warm 재열기 (같은 페이지, `n=10`) | **`1.0 / 3.8 ms`** | **`11.8 / 15.2 / 15.2 ms`** *(클릭 → `rAF1`)* | **통과** | `rAF2` p50 `28.2 ms` |

> **프레임 지표 해석**: `domMs`는 클릭 이벤트 직후 Svelte가 대상 패널 DOM을 마운트·배치 완료(`data-ready="true"`)할 때까지의 순수 JS·DOM 시간이고, `raf1Ms`는 다음 VSync 첫 페인트 프레임 도달 시각(60 Hz 기준 1프레임 ≤ 16.67 ms 예산 대조 기준)이다. `raf2Ms`는 페인트 직후 다음 프레임 완료 시각(16.67 ms VSync 1주기가 추가로 더해짐)으로 참고용으로 함께 수집했다.

---

## 2. 세부 측정 데이터 (Chromium `1248`)

### 2.1 글꼴 파일별 로드 시간 (Cold 1회차 기준, 총 6개 `.woff2` · 약 562 KB)

| 파일 | 글꼴·굵기 | `font-display` | 전송 크기 | 로드 시간 (`duration`) |
|---|---|---|---:|---:|
| `Freesentation-4Regular.woff2` | Freesentation 400 | `block` | 138,308 B | 1.3 ms |
| `Freesentation-5Medium.woff2` | Freesentation 500 | `block` | 139,300 B | 12.0 ms |
| `Freesentation-6SemiBold.woff2` | Freesentation 600 | `block` | 141,824 B | 11.8 ms |
| `Paperlogy-6SemiBold.woff2` | Paperlogy 600 | `block` | 112,368 B | 1.3 ms |
| `ibm-plex-mono-latin-400-normal.woff2` | IBM Plex Mono 400 | `block` | 15,008 B | 1.4 ms |
| `ibm-plex-mono-latin-500-normal.woff2` | IBM Plex Mono 500 | `block` | 15,188 B | 12.0 ms |

- 5회 Cold 실행의 FCP→`document.fonts.ready` 분포: `min 3.0 ms` · `p50 17.0 ms` · `p95/max 17.4 ms` · `avg 14.28 ms`.
- 부분 집합화(KS X 1001 + 카탈로그 문자) 덕분에 전체 전송 크기가 기존 IBM Plex 94조각(3.43 MiB) 대비 약 84% 축소되어 Vite dev 서버 환경에서도 Cold 17 ms로 30 ms 예산 안에 여유 있게 들어왔다.

### 2.2 색·굵기 패널 열기 (`ColorWidthPanel`, `.ink-cw-panel`)
- **Cold 첫 열기 (`n=5`)**:
  - `domMs`: min `2.3` · p50 **`3.9`** · p95/max `6.3` · avg `3.84` ms
  - `raf1Ms`: min `6.6` · p50 **`7.8`** · p95/max `8.8` · avg `7.68` ms
  - `raf2Ms`: min `22.9` · p50 `24.6` · p95/max `25.2` · avg `24.12` ms
- **Warm 재열기 (`n=10`)**:
  - `domMs`: min `1.3` · p50 **`1.8`** · p95/max `3.5` · avg `2.06` ms
  - `raf1Ms`: min `7.0` · p50 **`10.9`** · p95/max `11.8` · avg `10.51` ms
  - `raf2Ms`: min `23.8` · p50 `27.2` · p95/max `28.4` · avg `27.0` ms

### 2.3 글꼴 패널 열기 (`FontPicker`, `.ink-fp[data-ready="true"]`)
- **Cold 첫 열기 (`n=5`)**:
  - `domMs`: min `1.5` · p50 **`1.6`** · p95/max `3.0` · avg `1.88` ms
  - `raf1Ms`: min `7.6` · p50 **`8.0`** · p95/max `8.7` · avg `8.02` ms
  - `raf2Ms`: min `23.6` · p50 `24.4` · p95/max `25.0` · avg `24.36` ms
- **Warm 재열기 (`n=10`)**:
  - `domMs`: min `0.9` · p50 **`1.0`** · p95/max `3.8` · avg `1.63` ms
  - `raf1Ms`: min `3.2` · p50 **`11.8`** · p95/max `15.2` · avg `10.56` ms
  - `raf2Ms`: min `16.1` · p50 `28.2` · p95/max `28.8` · avg `25.07` ms

---

## 3. 참고 지표 및 예산 초과 위험 · 원인 짐작

### 3.1 미리보기 캡처 오버레이 준비 시간 (`overlayReady` 참고치 — 1차 M1 대비 주의)
- **실측 (`n=5`, `nativeFrame=false` 웹 이미지 경로)**:
  - 1회차(콜드): `불러오기 113.9 ms · 그리기 113.8 ms`
  - 2~5회차(웜): `불러오기 1.7~6.3 ms · 그리기 108.6~117.0 ms`
- **예산 대조**: 1차 PRD §5 `M1 캡처 진입` 목표는 `p50 ≤ 80 ms / p95 ≤ 150 ms`이며, 이번 미리보기 측정의 `그리기(paintMs)`는 108.6~117.0 ms로 80 ms p50 목표를 넘는다.
- **원인 짐작**:
  1. 측정 구간 도중 `resource-guard`가 학생 1명 접속을 감지해 프로세스를 낮은 우선순위(`background`)로 낮췄고, headless Chromium의 소프트웨어 합성 환경에서 전체 화면 `<img>` 디코드 + `backdrop-filter: blur(22px)`(툴바·배지·힌트) 합성 비용이 크게 부풀려졌을 가능성이 높다.
  2. `CaptureOverlay.svelte`는 `img load` 뒤 `requestAnimationFrame`을 2회 기다린 후 `paintMs`를 보고하므로, 60 Hz 환경에서 기본 33 ms 이상의 프레임 대기 시간이 포함된다.
  3. 실제 앱(Windows `nativeFrame=true` 투명 바탕 + 로컬 `ink://` 프로토콜 / macOS 하드웨어 가속 WKWebView)과 경로가 다르므로, **1차 M1 최종 합격 여부는 반드시 릴리스 빌드 실기 로그(`overlay ready [N ms]`, `n≥100`)로 판정**해야 한다.

### 3.2 글꼴 패널 첫 열기 실기 위험 (시스템 글꼴 다수 환경)
- 이번 미리보기 측정에서 글꼴 패널 첫 열기가 `8.0 ms`(예산 120 ms 대비 매우 낮음)로 나온 데에는 **중요한 단서**가 있다:
  - `packages/ink-ui/src/preview/Preview.svelte`의 `overlayPreviewClient` 래퍼가 `systemFonts()` 메서드를 위임하지 않아, 이번 측정에서는 내장 3종 글꼴만 렌더되었다([snapshots.md](snapshots.md) 문제 5).
- **원인 짐작 및 실기 주의점**:
  - 실제 macOS(`CoreText`)·Windows(`DirectWrite`) PC에는 시스템 글꼴이 보통 100~300종 설치되어 있다.
  - `CaptureOverlay.svelte`는 진입 시 `client.systemFonts()`를 미리 읽어 두지만(`systemFontsLoaded`), `FontPicker.svelte`가 처음 열릴 때 수백 개 행을 한 번에 DOM에 올리고 각 행의 이름을 **해당 시스템 글꼴(`style:font-family`)로 렌더링**하므로, 글꼴 수가 많은 PC에서는 첫 페인트가 120 ms 예산에 근접하거나 초과할 수 있다.
  - 대응 확인 필요: `overlayPreviewClient`에 `systemFonts` 위임을 추가한 뒤, 시스템 글꼴 150~250종 목업 조건과 실기 앱에서 `FontPicker` 첫 열기를 한 번 더 재측정할 것을 권장한다.

### 3.3 단축키 PDF 생성 시간 (참고)
- Node 환경 `buildShortcutSheetPdf` 실측:
  - 첫 호출(내장 TTF 5종 로드·파싱 포함, `macos/ko`): **64.74 ms** (47.5 KB)
  - 이후 호출(폰트 캐시, `macos/en`, `windows/ko`, `windows/en`): **22.06~28.07 ms** (34.6~46.0 KB)

---

## 4. 미실행·제한 사항

| 항목 | 상태 | 사유 |
|---|---|---|
| `webkit-2370` (WKWebView 계열 근사) 성능 측정 | **미실행 (실패 중단)** | `page.goto(..., { waitUntil: 'load' })` 직후 `Execution context was destroyed, most likely because of a navigation` 오류 발생. 향후 `networkidle` 대기 후 측정하도록 보완 필요 |
| 실제 Tauri 릴리스 빌드 `M1~M10` 실기 측정 | **미실행** | 본 작업은 가짜 클라이언트 브라우저 미리보기 범위만 다룸 (실기 앱 조작·실제 화면 캡처 금지 규칙) |
| 시스템 글꼴 200종 부하 조건 `FontPicker` 첫 열기 | **미실행** | `Preview.svelte`의 `overlayPreviewClient`에 `systemFonts` 위임 누락(화면 코드 수정 금지 범위) |
