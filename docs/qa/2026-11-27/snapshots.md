# 미리보기 화면 스냅숏 자동 검사 보고서 (`R2-QA-1`)

- **실행 일시**: 2026-11-27 16:00~16:03 KST
- **실행 명령**: `/Users/bodyking/.local/bin/resource-guard run --profile test --wait-seconds 900 -- bash -c 'timeout 900 node scripts/qa-snapshots.mjs'`
- **검사 엔진**: Chromium `1248` (`@playwright/test@1.64.1`), 뷰포트 `1280×820`, `FakeInkClient` 기반 `@screen-ink/ui` 미리보기
- **원칙**: 이번 QA 작업은 화면 코드(`packages/ink-ui/src/**`, `apps/**`, `crates/**`)를 수정하지 않고 발견된 결함을 재현 조건·수치·추정 원인과 함께 목록으로 남긴다.

---

## 1. 전체 집계

| 지표 | 결과 |
|---|---:|
| 총 스냅숏 수 | **168장** |
| 대상 미발견 / 스크린숏 실패 | **0건** |
| 콘솔 오류 (`console.error`) / 페이지 예외 (`pageerror`) | **0건 / 0건** |
| **OS 표기 섞임** (화면 168장 + PDF 모델 4조합) | **0건** |
| 컨테이너 경계 이탈 / 뷰포트 잘림 / 직계 형제 겹침 (자동) | **0건 / 0건 / 0건** |
| **가로 넘침** (`scrollWidth > clientWidth`, `overflow: visible`) | **12장** (2개 패턴) |
| **잘림(말줄임)** (`overflow: hidden` + `text-overflow: ellipsis`) | **20장** (2개 패턴) |
| **눈 검수 추가 발견** (자동 규칙 밖) | **3건** |

---

## 2. 발견된 화면 문제 목록 (수정 대상)

### 문제 1. `[P0 · i18n 누락 / 넘침·잘림]` 단축키 추천 묶음 원시 키 노출 (`page.shortcuts.preset.*`)
- **영향 범위 (16장)**:
  - `first-step3` (8조합 전체: `macos/windows × ko/en × light/dark`)
  - `settings-shortcuts` (8조합 전체: `macos/windows × ko/en × light/dark`)
- **실측 증거**:
  - `first-step3` 넘침:
    - `button.ink-preset-card`: `scrollWidth 170 px > clientWidth 157 px` (**+13 px** 넘침)
    - `span.ink-preset-top`: `scrollWidth 161 px > clientWidth 139 px` (**+22 px** 넘침, 초록 배지 `page.shortcuts.preset.current`가 옆 카드 경계를 침범)
  - `first-step3` 말줄임 잘림:
    - `span.ink-preset-name("page.shortcuts.preset.snipaste")`: `171 px > 139 px` (**+32 px** 잘림)
    - `span.ink-preset-name("page.shortcuts.preset.lefthand")`: `170 px > 139 px` (**+31 px** 잘림)
  - `settings-shortcuts` 말줄임 잘림:
    - `span.ink-preset-name("page.shortcuts.preset.presentation")`: `202 px > 29 px` (**+173 px** 잘림 → `"pa..."`로 축약)
- **원인 추정**: 병렬 작업 중인 `R2-KEY-6` 단축키 추천 묶음 UI가 참조하는 `page.shortcuts.preset.*` 키들이 `packages/ink-ui/src/lib/i18n/{ko,en}.json` 합본에 아직 등록/합쳐지지 않아 원시 키 문자열이 그대로 렌더되면서 긴 문자열이 카드 폭을 초과함.
- **참고 그림**: `output/qa/2026-11-27/first-step3__macos_ko_light.png`, `output/qa/2026-11-27/settings-shortcuts__macos_en_light.png`

### 문제 2. `[P0 · 다국어 미적용]` 영어 설정 › 단축키 상단 충돌 배너에 한국어 잔존 *(눈 검수 발견)*
- **영향 범위**: `settings-shortcuts` 영어(`lang=en`) 4조합 (`macos/windows × light/dark`)
- **현상**: 화면 전체가 영어(`Shortcuts`, `Save as PDF`, `How to turn off`, `Change key`)인데 상단 충돌 배너 문구만 **`Snipaste에서 F1·F3 등 3개를 쓰고 있어요`** 로 한국어가 그대로 노출됨.
- **원인 추정**: `packages/ink-ui/src/lib/screens/ShortcutsPage.svelte` L132에서 `formatConflictBanner(conflicts, platform)` 호출 시 세 번째 인자 `lang`(`locale.lang`)을 전달하지 않아 기본값 `'ko'`로 동작하는 것으로 보임 (`shortcutDisplay.js` L90 시그니처는 `formatConflictBanner(conflicts, platform = 'windows', lang = 'ko')`).
- **참고 그림**: `output/qa/2026-11-27/settings-shortcuts__macos_en_light.png`

### 문제 3. `[P1 · 넘침]` macOS 트레이 팝오버 `[전체]` 타일 키캡 +3 px 넘침
- **영향 범위 (4장)**: `tray` macOS 4조합 (`ko/en × light/dark`)
- **실측 증거**:
  - `span.ink-tray-tile-top("⌃⌥F1")`: `scrollWidth 96 px > clientWidth 93 px` (**+3 px** 넘침)
- **현상·원인 추정**: 2×2 타일 중 왼쪽 아래 `[전체]`(Full screen) 타일 상단 줄에 모니터 아이콘(18 px)과 3키 조합 키캡(`⌃`, `⌥`, `F1`)이 나란히 들어가며 타일 안쪽 너비(93 px)를 3 px 초과함(키캡 `F1`이 타일 오른쪽 테두리에 거의 밀착). Windows는 `Ctrl F1` 2키 조합이라 넘치지 않음.
- **참고 그림**: `output/qa/2026-11-27/tray__macos_ko_light.png`

### 문제 4. `[P1 · 잘림 + 미리보기 겹침]` 결과 섬(`Island` `variant="result"`) 보조 문구 말줄임 잘림
- **영향 범위 (4장)**: `island-gallery` 4조합 (`macos/windows × ko/en`, `dark`)
- **실측 증거**:
  - 단색 HEX 칩 포함 결과 섬(`120×120` + `[저장]` + `[그리기]` + `[#FF9F0A 복사]`):
    - `macos / ko`: `"120×120 · 문서·발표 자료에 붙여 넣기 · ⌘V"` → `234 px > 198 px` (**+36 px** 잘림)
    - `macos / en`: `"120×120 · Paste into documents · ⌘V"` → `235 px > 193 px` (**+42 px** 잘림)
    - `windows / ko`: `"120×120 · 문서·발표 자료에 붙여 넣기 · Ctrl+V"` → `256 px > 198 px` (**+58 px** 잘림)
    - `windows / en`: `"120×120 · Paste into documents · Ctrl+V"` → `257 px > 193 px` (**+64 px** 잘림)
  - 기본 결과 섬(칩 없음, Windows만):
    - `windows / ko`: `"718×40 · 문서·발표 자료에 붙여 넣기 · Ctrl+V"` → `250 px > 237 px` (**+13 px** 잘림)
    - `windows / en`: `"718×40 · Paste into documents · Ctrl+V"` → `251 px > 240 px` (**+11 px** 잘림)
- **눈 검수 추가 관찰**:
  - 실제 앱에서 결과 섬의 `max-width: 520px` 제한 안에 썸네일 + 버튼 2개 + HEX 칩이 모두 들어가면 보조 문구 공간이 193~198 px로 좁아져 단축키 끝부분(`⌘V`/`Ctrl+V`)이 말줄임으로 잘려 버린다(`R2-FIX-4` 문구 길이·우선순위 조정 검토 필요).
  - 또한 미리보기 갤러리(`Preview.svelte`의 `.pv-island-grid`)가 1280 px 화면에서 3열 균등 배치되어 각 열 폭이 약 390 px에 불과하므로, 520 px 결과 섬 두 개가 옆 칸으로 겹쳐 보이고 오른쪽 칩이 화면 밖으로 잘린다(미리보기 레이아웃 조정 필요).
- **참고 그림**: `output/qa/2026-11-27/island-gallery__macos_ko_dark.png`, `output/qa/2026-11-27/island-gallery__windows_en_dark.png`

### 문제 5. `[P2 · 미리보기 연동 누락]` `Preview.svelte` 오버레이 래퍼에서 `systemFonts` 미위임 *(눈 검수 발견)*
- **영향 범위**: `font-picker` 미리보기 전체 (8조합)
- **현상**: `CaptureOverlay.svelte` L1135가 `client.systemFonts()`를 호출하지만, `Preview.svelte` L79~115의 `overlayPreviewClient` 래퍼에 `systemFonts` 메서드 위임이 빠져 있어 미리보기 글꼴 고르기에 이 컴퓨터 글꼴(가짜 5종)이 표시되지 않고 앱 내장 3종(`Freesentation`, `Paperlogy`, `IBM Plex Mono`)만 보인다.
- **참고 그림**: `output/qa/2026-11-27/font-picker__macos_ko_dark.png`

### 참고 관찰 (경미·미리보기 한정)
- `island-gallery`의 `지연 캡처` 영어 섬: 원형 숫자 배지 `3` 뒤에 `s · Area capture`가 이어져 `3`과 `s` 간격이 넓어 보임.
- `island-gallery` macOS 화면에 Windows 전용 `창 맨 위에(Always on top)` 샘플 섬이 함께 노출됨(미리보기 갤러리 한정; 실제 트레이·설정 화면에서는 macOS에서 정상 숨김 확인).
- 캡처 오버레이 실행 시 `FakeInkClient`가 내보내는 `CAPTURE_ISLAND` 안내 문구(`범위 캡처 · 휠 넓히기·좁히기`)는 `packages/ink-client/src/fake.js`의 한국어 고정 목업 문자열임(영어 미리보기 오버레이 상단 섬에 한국어 목업 표시).

---

## 3. 화면별 22종 전체 검수표

> 조합 수: 양 테마 화면 20종 × 8조합(`macos/windows × ko/en × light/dark`) = 160장, 다크 고정 화면 2종 × 4조합(`macos/windows × ko/en × dark`) = 8장 → **합계 168장**.

| # | 화면 ID | 화면 이름 | 촬영 수 | 넘침 | 잘림(말줄임) | OS 섞임 | 판정 |
|---|---|---|---:|---:|---:|---:|---|
| 1 | `tray` | 트레이 팝오버 | 8 | **4** (mac) | 0 | 0 | 주의 (문제 3) |
| 2 | `settings-shortcuts` | 설정 · 단축키(전역) | 8 | 0 | **8** | 0 | 실패 (문제 1·2) |
| 3 | `settings-shortcuts-inapp` | 설정 · 단축키(화면 안) | 8 | 0 | 0 | 0 | 통과 |
| 4 | `settings-capture` | 설정 · 캡처·녹화 | 8 | 0 | 0 | 0 | 통과 |
| 5 | `settings-draw` | 설정 · 그리기·확대 | 8 | 0 | 0 | 0 | 통과 |
| 6 | `settings-general` | 설정 · 일반 (권한/점검·업데이트) | 8 | 0 | 0 | 0 | 통과 |
| 7 | `settings-help` | 설정 · 사용 설명서 (카드 8장) | 8 | 0 | 0 | 0 | 통과 |
| 8 | `settings-info` | 설정 · 정보 (라이선스 고지) | 8 | 0 | 0 | 0 | 통과 |
| 9 | `settings-search` | 설정 · 찾기 팝오버 | 8 | 0 | 0 | 0 | 통과 |
| 10 | `help-modal-capture` | 사용 설명서 모달 · 찍기 | 8 | 0 | 0 | 0 | 통과 |
| 11 | `help-modal-troubleshoot` | 사용 설명서 모달 · 문제 해결 | 8 | 0 | 0 | 0 | 통과 |
| 12 | `first-step0` | 첫 실행 ① 반가워요 | 8 | 0 | 0 | 0 | 통과 |
| 13 | `first-step1` | 첫 실행 ② 화면 기록 / 단축키 겹침 | 8 | 0 | 0 | 0 | 통과 |
| 14 | `first-step2` | 첫 실행 ③ 손쉬운 사용 / 시작 실행 | 8 | 0 | 0 | 0 | 통과 |
| 15 | `first-step3` | 첫 실행 ④ 꼭 알아 둘 키 세 개 | 8 | **8** | **8** | 0 | 실패 (문제 1) |
| 16 | `first-step4` | 첫 실행 ⑤ 준비 끝 + 연습 카드 | 8 | 0 | 0 | 0 | 통과 |
| 17 | `capture-toolbar` | 캡처 오버레이 · 유리 알약 툴바 | 8 | 0 | 0 | 0 | 통과 |
| 18 | `color-width-panel` | 색·굵기 패널(펜) | 8 | 0 | 0 | 0 | 통과 |
| 19 | `font-picker` | 글꼴 고르기 팝오버 | 8 | 0 | 0 | 0 | 통과 *(문제 5 단서)* |
| 20 | `draw-toolbar` | 그리기 도구 막대 · 색·굵기 패널 | 8 | 0 | 0 | 0 | 통과 |
| 21 | `cheat-sheet` | 단축키 한 장(S16) *(다크 전용)* | 4 | 0 | 0 | 0 | 통과 |
| 22 | `island-gallery` | 섬 알림 16종 모음 *(다크 전용)* | 4 | 0 | **4** | 0 | 주의 (문제 4) |

---

## 4. 단축키 PDF 생성·OS 표기 검증 (PDF 미리보기 대체)

- 설정 안 인라인 PDF 미리보기 화면(`R2-PDF-7`, P1)은 현재 코드에 미구현 상태다(`ShortcutsPage.svelte`와 `CheatSheet.svelte`에 `[PDF로 받기]` 다운로드 버튼만 존재).
- 대신 `scripts/qa-snapshots.mjs`가 `buildShortcutSheetModel` · `extractTextFragments` · `buildShortcutSheetPdf`를 4조합으로 직접 실행해 PDF 헤더(`%PDF-`), 용량, 생성 시간, 모든 텍스트 조각의 OS 표기 섞임을 검사했다.

| OS | 언어 | `%PDF-` 헤더 | 쪽 수 | 상단 큰 타일 | 텍스트 조각 | 파일 크기 | 생성 시간 | OS 표기 섞임 |
|---|---|---|---:|---:|---:|---:|---:|---:|
| `macos` | `ko` | 정상 | 1쪽 | 6 | 188 | 47,502 B | 64.74 ms *(폰트 첫 로드 포함)* | **0건** |
| `macos` | `en` | 정상 | 1쪽 | 6 | 188 | 35,991 B | 28.07 ms | **0건** |
| `windows` | `ko` | 정상 | 1쪽 | 6 | 190 | 46,029 B | 22.69 ms | **0건** |
| `windows` | `en` | 정상 | 1쪽 | 6 | 190 | 34,636 B | 22.06 ms | **0건** |

---

## 5. 고친 뒤 (`G16` 재검사)

- **실행 일시**: 2026-11-27 17:21~17:24 KST
- **실행 명령**: `/Users/bodyking/.local/bin/resource-guard run --profile test --wait-seconds 900 -- bash -c 'timeout 900 node scripts/qa-snapshots.mjs --only=snapshots'`

### 전후 집계 비교

| 지표 | 고치기 전 (`R2-QA-1`) | 고친 뒤 (`G16`) | 변화 |
|---|---:|---:|---|
| 총 스냅숏 수 | 168장 | 168장 | 유지 |
| 대상 미발견 / 스크린숏 실패 | 0건 / 0건 | 0건 / 0건 | 유지 |
| 콘솔 오류 / 페이지 예외 | 0건 / 0건 | 0건 / 0건 | 유지 |
| OS 표기 섞임 (화면 168장 + PDF 4조합) | 0건 | 0건 | 유지 |
| 경계 이탈 / 뷰포트 잘림 / 직계 형제 겹침 | 0건 / 0건 / 0건 | 0건 / 0건 / 0건 | 유지 |
| **가로 넘침** | **12장** | **0장** | **-12장 (전부 해소)** |
| **잘림(말줄임)** | **20장** | **8장** | **-12장** |

### 항목별 확인 결과

- **문제 1 (`first-step3` · `settings-shortcuts` 원시 키)**: G13 합본 반영으로 원시 키 노출 해소(`settings-shortcuts` 8장·`first-step3` 한국어 4장 모두 통과, 넘침 8장 → 0장).
- **문제 2 (`settings-shortcuts` 영어 충돌 배너 한국어)**: `8cb4a9e`로 해소.
- **문제 3 (`tray` macOS `[전체]` 타일 `⌃⌥F1` +3 px 넘침)**: **4장 → 0장**. 타일 좌우 패딩(`9px → 8px`)·윗줄 간격(`4px → 3px`)·타일 전용 키캡 치수(`18px`, `10px`, `gap 2px`) 조정으로 `⌃⌥F1`이 테두리 안쪽에 여유 있게 들어가며 다른 3개 타일 모양 유지(`tray__macos_ko_light.png` 눈 검수 확인).
- **문제 4 (`island-gallery` 결과 섬 보조 문구 잘림 + 미리보기 3열 겹침)**: **4장 → 0장**.
  - 실제 섬(`Island.svelte`): 결과 카드 썸네일(`64→56px`)·간격(`8→6px`)·단추 패딩(`11→9px`)을 다듬고 `.ink-island-result-meta`에 두 줄 줄바꿈(`white-space: normal; word-break: keep-all`)을 허용했다. Windows 기본 결과 섬은 한 줄에 다 들어가고, 단색 HEX 칩 포함 결과 섬은 두 줄로 접혀 `⌘V`/`Ctrl+V` 끝까지 잘림 없이 표시된다.
  - 미리보기(`Preview.svelte`): `.pv-island-grid`를 `minmax(min(540px, 100%), 1fr)`(1280 px 기준 2열)로 조정해 520 px 결과 섬의 칸 겹침·화면 밖 잘림 해소(`island-gallery__macos_ko_dark.png`, `island-gallery__windows_en_dark.png` 눈 검수 확인).
- **문제 5 (`font-picker` `systemFonts` 미위임)**: `Preview.svelte`의 `overlayPreviewClient`에 `systemFonts` 위임을 추가해 이 컴퓨터 글꼴(가짜 5종) 정상 표시 확인(`font-picker__macos_ko_dark.png`).
- **정보 › 글꼴 고지(`settings-info`)**: `Freesentation · Paperlogy · IBM Plex Mono · Pretendard (SIL OFL 1.1)` 반영 및 넘침·잘림 0건 확인(`settings-info__macos_ko_light.png`).
- **PDF 4조합 재검증**: `%PDF-` 정상 · OS 표기 섞임 0건 (`macos/ko` 47,500 B, `macos/en` 35,990 B, `windows/ko` 46,028 B, `windows/en` 34,635 B).

### 잔여 잘림 8장 (G16 범위 밖 · 후속 참고)

1. **`first-step3` 영어 4조합** (`macos/windows × light/dark`): `span.ink-preset-name("Presentation default")` `111 px > 91 px` (**+20 px** 잘림) — 영어 카드에서 `Current` 배지와 나란히 놓일 때 이름 말줄임 잔존.
2. **`settings-general` 영어 4조합** (`macos/windows × light/dark`): `span.ink-row-sub("Saves logs, system info, and settings as a zip ·")` `401 px > 275 px` (**+126 px** 잘림) — G14 추가 항목(`about.diag.sub`) 영어 보조 문구 한 줄 말줄임.
