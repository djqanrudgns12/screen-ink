# [G18] 디자인 대조·관문 보고서 (`0.4` · `1.6` · `7.7`)

- **작성일**: 2026-11-27
- **기준 문서**:
  - 디자인 시스템 1.0: [docs/prd/10-design-system.md](../../prd/10-design-system.md)
  - 화면 명세(S01~S17): [docs/prd/04-screens.md](../../prd/04-screens.md)
  - 시안 v3 원본: [docs/mockups/v3/canvas.json](../../mockups/v3/canvas.json) · [contact.png](../../mockups/v3/png/contact.png) · `F01~F18`
  - 2차 고도화 결정(D1~D7 · 디자인 시스템 2.0): [docs/plans/2026-11-27/PRD-2차-고도화.md](../../plans/2026-11-27/PRD-2차-고도화.md)
- **검증 환경**: `@screen-ink/ui` Vite 미리보기(`FakeInkClient`), Chromium `1248`, 뷰포트 `1280×820`, 168장 스냅숏(`macos/windows × ko/en × light/dark`)

---

## 1. `0.4` 하드코딩 색 정적 검사 및 토큰화 결과

### 1.1 정적 검사 시험
- **추가 파일**: [hardcodedColors.test.js](../../../packages/ink-ui/src/lib/a11y/hardcodedColors.test.js)
- **검사 대상**: `packages/ink-ui/src/**/*.svelte`, `*.css`
- **탐지 규칙**: `#RGB` · `#RGBA` · `#RRGGBB` · `#RRGGBBAA`, `rgb(...)` · `rgba(...)`, `hsl(...)` · `hsla(...)`
- **허용 목록**:
  - `src/lib/tokens.css`: 디자인 토큰 원본 정의 파일
  - `src/preview/**`: 개발용 가짜 클라이언트 미리보기 하네스(배경 목업·샘플 HEX 포함, 실제 앱 번들 미포함)
  - 주석(`<!-- -->`, `/* */`, `<script>` 내 `//`) 및 SVG 내부 참조(`url(#...)`, `href="#..."`)
- **결과**: 단위 탐지기 검증 및 전체 소스 검사 **위반 0건** 통과.

### 1.2 치환 및 토큰 확장 요약
- [tokens.css](../../../packages/ink-ui/src/lib/tokens.css)의 `:root, [data-theme='light']`, `[data-theme='dark']`, `@media (prefers-color-scheme: dark)` 세 블록에 유리 알약·색/굵기 패널·글꼴 고르기·트레이 반투명·캔버스 커서/HUD·미니맵·확대 후광용 `--ink-*` 토큰을 추가하고, [contrast.test.js](../../../packages/ink-ui/src/lib/a11y/contrast.test.js)의 다크 테마 일치 규칙을 유지했다.
- 기존 컴포넌트 14개에 흩어져 있던 색 리터럴을 전부 `var(--ink-*)`로 치환했다(치환 뒤 `scripts/check-contrast.mjs` **146쌍 전부 통과**, 글자 최소 `4.71:1`, UI 최소 `3.01:1`).

| 대상 파일 | 치환 내용 |
|---|---|
| [AnnotateLayer.svelte](../../../packages/ink-ui/src/lib/components/AnnotateLayer.svelte) | 지우개 링 `--ink-pen-W`, 텍스트 점선 `--ink-annot-outline`, 링 그림자 `--ink-annot-ring-shadow` |
| [AnnotateOptions.svelte](../../../packages/ink-ui/src/lib/components/AnnotateOptions.svelte) | 색 점 테두리 `--ink-island-dot-ring` |
| [CaptureToolbar.svelte](../../../packages/ink-ui/src/lib/components/CaptureToolbar.svelte) | 유리 알약 `--tb-*`를 `--ink-tb-*` · `--ink-tip-*` · `--ink-on-accent` · `--ink-glass-solid`로 매핑 |
| [ColorWidthPanel.svelte](../../../packages/ink-ui/src/lib/components/ColorWidthPanel.svelte) | 패널 `--cw-*` · 스와치 링 · 무지개 휠 · 슬라이더 썸 · 말풍선 토큰 매핑 |
| [DrawCanvas.svelte](../../../packages/ink-ui/src/lib/components/DrawCanvas.svelte) | 캔버스 바탕 `--ink-island`, 텍스트/링/복원 칩/스포이트 힌트 HUD 토큰 매핑 |
| [DrawToolbar.svelte](../../../packages/ink-ui/src/lib/components/DrawToolbar.svelte) | 그리기 막대 `--dtb-*` · 스타일 점 링 · 툴팁 그림자 토큰 매핑 |
| [FontPicker.svelte](../../../packages/ink-ui/src/lib/components/FontPicker.svelte) | 글꼴 팝오버 `--fp-*` · 활성 탭/칩 글자 `--ink-on-accent` · 스크롤바 썸 토큰 매핑 |
| [Island.svelte](../../../packages/ink-ui/src/lib/components/Island.svelte) | 흰색 스와치·펜 점·HEX 칩 테두리 토큰 매핑 |
| [MiniMap.svelte](../../../packages/ink-ui/src/lib/components/MiniMap.svelte) | 현재 뷰포트 강조 바탕 `--ink-minimap-view-bg` |
| [TrayPopover.svelte](../../../packages/ink-ui/src/lib/components/TrayPopover.svelte) | 라이트·다크·macOS/Windows 반투명 재질 및 그림자 토큰 매핑 |
| [CaptureOverlay.svelte](../../../packages/ink-ui/src/lib/screens/CaptureOverlay.svelte) | 크기 배지·단계 안내·근거 태그 외곽선·글꼴 알림 HUD 토큰 매핑 |
| [DrawZoomPage.svelte](../../../packages/ink-ui/src/lib/screens/DrawZoomPage.svelte) | 칠판 미리보기 흰/검은 카드 글자·테두리 토큰 매핑 |
| [TrayPopoverScreen.svelte](../../../packages/ink-ui/src/lib/screens/TrayPopoverScreen.svelte) | 녹화 중 타일 강조 `--ink-danger-soft-bg` · `--ink-danger-soft-border` |
| [ZoomHud.svelte](../../../packages/ink-ui/src/lib/screens/ZoomHud.svelte) | 실시간 확대 후광 `--ink-zoom-halo-bg` · `--ink-zoom-halo-glow` |

---

## 2. `1.6` · `7.7` 시안 v3 ↔ 실제 화면 대조표

> 판정 기준: **맞음**(시안과 일치) · **결정으로 바뀜**(2차 고도화 D1~D7·디자인 시스템 2.0으로 의도 변경, 수정 안 함) · **고침**(이번 작업에서 수정) · **남음**(후속/실기 확인 필요)

| 화면 | 시안(v3) | 판정 | 시안과 차이(간격·글자·색·모서리·아이콘·배치·빠진 요소) | 조치 및 근거 |
|---|---|---|---|---|
| **S01 트레이 팝오버** | [F03](../../mockups/v3/png/F03-tray.png) | **결정으로 바뀜 + 고침** | ① 2차 D3(`tray-mock.png`)에 따라 폭 `252px`, 상단 2×2 큰 동작 타일 4칸(영역·창·전체·녹화), `Paperlogy` 글꼴, 슉쇽 맞춤 아이콘(D1) 적용됨(시안의 세로 9행+`288px`에서 의도 변경). ② 실제 화면 다크 모드 모서리 반지름이 `12px`로 고정되어 다크 아일랜드 `18px`(`--ink-r-popover`)과 어긋남 발견. ③ 반투명 배경색 리터럴 포함. | ①은 **결정으로 바뀜**(D1·D3·R2-UI-TRAY). ② [TrayPopover.svelte](../../../packages/ink-ui/src/lib/components/TrayPopover.svelte)에서 `border-radius: var(--ink-r-popover)`로 **고침**(라이트 12 / 다크 18). ③ 배경·그림자 토큰화 완료. |
| **S02 설정 · 단축키** | [F04](../../mockups/v3/png/F04-keys.png) | **결정으로 바뀜 + 고침** | ① 사이드바 `196px`, 창 `900×600`, 검색 `⌘K`/`Ctrl+K` 추가(R2-SET-1), 상단 `[PDF로 받기]`(R2-PDF-5) · 추천 묶음 3종(R2-KEY-6) · 사용 설명서 탭(R2-HELP) · `Freesentation`/`Paperlogy` · 맞춤 아이콘 적용. ② `.ink-page-title`에 `var(--ink-font-display)`가 빠져 본문 글꼴(`Freesentation`)로 렌더되던 어긋남 발견. | ①은 **결정으로 바뀜**(R2-SET-1·KEY-6·PDF-5·HELP·FONT). ② [ShortcutsPage.svelte](../../../packages/ink-ui/src/lib/screens/ShortcutsPage.svelte) 제목 글꼴 `var(--ink-font-display)`(`Paperlogy`)로 **고침**. |
| **S03 설정 · 캡처·녹화** | [F05](../../mockups/v3/png/F05-capture-set.png) | **결정으로 바뀜 + 고침** | ① 다크 테마 모서리(`14/12/18px`)·행 구성·파일 이름 실시간 주황 미리보기·녹화 30/60fps·캡처 기록 섹션 모두 시안과 일치. ② 제목 `.ink-page-title`에 `var(--ink-font-display)` 누락 발견. | ① **맞음** + D1·D4·R2-WORD("세부 영역까지 찾기", "자동 맞춤") 반영. ② [CapturePage.svelte](../../../packages/ink-ui/src/lib/screens/CapturePage.svelte) 제목 글꼴 `var(--ink-font-display)`로 **고침**. |
| **S04 설정 · 그리기·확대** | [F06](../../mockups/v3/png/F06-draw-set.png) | **결정으로 바뀜 + 고침** | ① 펜 색·굵기·보정·그리기·칠판 미리보기·실시간 확대 구성 일치, 쉬운 말("그리기·확대", "처음 확대 크기") 및 맞춤 아이콘 적용. ② 제목 `var(--ink-font-display)` 누락 + 칠판 미리보기 색 리터럴 3건 발견. | [DrawZoomPage.svelte](../../../packages/ink-ui/src/lib/screens/DrawZoomPage.svelte) 제목 글꼴 `Paperlogy` 적용 및 색 토큰화 **고침**. |
| **S05 설정 · 일반** | [F07](../../mockups/v3/png/F07-general.png) | **결정으로 바뀜 + 고침** | ① 상단 권한 점검(R2-ONB-6)·문제 보고용 기록 묶기(R2-DIAG-1)·업데이트(R2-UPD-4) 추가, 테마 말투("기기 설정·밝게·어둡게") 및 macOS/Windows 분기 적용. ② 영어 스냅숏(`settings-general` en 4조합)에서 `Bundle diagnostic logs` 보조 문구가 `+126px` 말줄임 잘림 발견. | [GeneralPage.svelte](../../../packages/ink-ui/src/lib/screens/GeneralPage.svelte)에서 `about.diag` 보조 문구 줄바꿈 허용으로 **고침**(잘림 4장 → 0장). 제목은 이미 `var(--ink-font-display)` 적용 상태(**맞음**). |
| **S06 첫 실행 (5장)** | [F08](../../mockups/v3/png/F08-first.png) | **결정으로 바뀜 + 고침** | ① 1장 카드에서 5장 온보딩 슬라이드(① 반가워요 + 찰떡쌤 + 언어 전환 · ② 화면 기록/단축키 겹침 · ③ 손쉬운 사용/시작 실행 · ④ 꼭 알아 둘 키 + 추천 묶음 · ⑤ 준비 끝 + 연습 카드)로 확장(R2-ONB-1~9). ② `first-step3` 영어 4조합에서 `Presenter`/`Presentation default` + `Active` 배지 줄이 `+20px` 말줄임 잘림 발견. | [ShortcutPresetPicker.svelte](../../../packages/ink-ui/src/lib/components/ShortcutPresetPicker.svelte) `is-compact` 카드 제목 줄바꿈/래핑 허용으로 **고침**(잘림 4장 → 0장). |
| **S07 F1 범위 고르기** | [F09](../../mockups/v3/png/F09-pick.png) | **맞음 + 고침** | 주황 1.5px 선택 테두리·네 귀 2.5px 꺾쇠·50% 어두운 바깥·상단 검은 알약 섬(경로 + 크기)·108×84 9배 확대경(`#FFFFFF` + 좌표 + 복사)·단계별 하단 안내 배지 일치. | [CaptureOverlay.svelte](../../../packages/ink-ui/src/lib/screens/CaptureOverlay.svelte)의 배지·안내·근거 태그 색 리터럴 토큰화로 **고침**. 실제 화면 위 멀티모니터·DPI 배율은 **남음**(실기 눈 확인). |
| **S08 F1 결과** | [F10](../../mockups/v3/png/F10-done.png) | **결정으로 바뀜 + 고침** | ① 유리 알약 툴바(R2-UI-TB)·16색+내 색+로그 슬라이더 색/굵기 패널(R2-ANN)·글꼴 고르기 팝오버(R2-TXT) 반영. 결과 섬은 썸네일+체크+크기+`[저장]`+`[그리기]`(+단색 HEX 칩)+진행선으로 일치. ② 툴바/패널/글꼴 팝오버 내 색 리터럴 100여 건을 토큰으로 치환. ③ **남음**: 상단 근처 선택 영역에서 `FontPicker` 팝오버를 열면 상단 섬과 일부 겹침(툴바 위치가 상단 근처일 때의 배치 보정 필요). | 툴바·패널·글꼴 팝오버 색 토큰화 **고침**. `FontPicker`와 상단 섬 겹침 회피 배치는 **남음**. |
| **S09 F1 드래그 · 자석** | (S07 체계) | **맞음** | 드래그 실시간 크기·주황 자석 안내선·`⌥`/`Alt` 자석 끔 상태가 S07 오버레이 체계 안에서 동작. | **맞음**(스냅숏 자동 검사 통과). |
| **S10 F2 창 캡처** | [F11](../../mockups/v3/png/F11-window.png) | **맞음** | 창 전체 강조·오른쪽 위 크기 꼬리표·상단 섬("창 캡처 · 앱 이름 · 한 번 더 = 지금 창") 구성 일치. | OS 실기 창 경계·둥근 귀 캡처는 **남음**(실기 눈 확인). |
| **S11 F3 녹화 중** | [F12](../../mockups/v3/png/F12-record.png) | **맞음** | 빨간 네 귀 꺾쇠·22% 어두운 바깥(`RecordFrame.svelte`)·녹화 섬(빨간 맥박 점·`00:42`·해상도·음소거 아이콘·`[■ 중지 F3]`) 일치. | 실기 영상 제외(`WDA_EXCLUDEFROMCAPTURE`)·네 귀 꺾쇠는 **남음**(실기 확인). |
| **S12 Alt+1 그리기(판서)** | [F13](../../mockups/v3/png/F13-draw.png) | **결정으로 바뀜 + 고침** | ① R2-DRAW-TB·R2-ANN-5 결정에 따라 하단 중앙 유리 도구 막대 + 색/굵기/글꼴 패널이 추가됨, 상단 작은 섬·펜 커서 링·6색 펼침 유지. ② `DrawCanvas.svelte`·`DrawToolbar.svelte` 하드코딩 색 리터럴 전부 토큰화. | ①은 **결정으로 바뀜**(R2-DRAW-TB·R2-ANN-5). ② 색 토큰화 **고침**. |
| **S13 Alt+2 실시간 확대** | [F14](../../mockups/v3/png/F14-zoom.png) | **맞음 + 고침** | 44px 주황 커서 후광(`ZoomHud.svelte`)·오른쪽 아래 `112×70` 위치 지도(`MiniMap.svelte`)·확대 섬(`×2.0` + 멈추고 그리기) 시안과 일치. | [ZoomHud.svelte](../../../packages/ink-ui/src/lib/screens/ZoomHud.svelte) · [MiniMap.svelte](../../../packages/ink-ui/src/lib/components/MiniMap.svelte) 색 리터럴 토큰화 **고침**. 실기 확대 화면 추적은 **남음**. |
| **S14 Alt+3 칠판** | [F15](../../mockups/v3/png/F15-board.png) | **맞음** | 흰/검은 칠판 바탕, 상단 칠판 섬(`칠판 · 흰색 · 1 / 1 · W K`), 그리기 도구 막대 공유. | **맞음**(`DrawCanvas` 색 토큰화 반영). |
| **S15 창 항상 위 고정** | [F16](../../mockups/v3/png/F16-pin.png) | **맞음** | `PinFrame.svelte` 주황 테두리(배지 없음)·`PinView.svelte`·고정 알림 섬 일치(Windows 전용, macOS 숨김). | Windows 실기 창 고정 테두리는 **남음**. |
| **S16 단축키 한 장(?)** | [F17](../../mockups/v3/png/F17-cheat.png) | **결정으로 바뀜 + 고침** | ① 다크 고정 3열 레이아웃(어디서나 · 지금 화면 주황 테두리 · 끌어서 그리기), 우상단 `[PDF로 받기]`(R2-PDF-5) + 닫기 단추(R2-FIX-13) 추가, 쉬운 말·OS별 키 적용. ② 제목 `.ink-cheat-title`에 `var(--ink-font-display)` 누락 발견. | [CheatSheet.svelte](../../../packages/ink-ui/src/lib/screens/CheatSheet.svelte) 제목 글꼴 `Paperlogy` 적용으로 **고침**. |
| **S17 섬 상태 모음** | [F18](../../mockups/v3/png/F18-island.png) | **결정으로 바뀜 + 고침** | ① 12종+ 변형(단색 HEX 칩·4초 물러남·지연 캡처·8색 펼침 등) 반영, 쉬운 말·OS별 키 적용. ② `Island.svelte` 스와치/칩 테두리 색 리터럴 3건 토큰화. | [Island.svelte](../../../packages/ink-ui/src/lib/components/Island.svelte) 색 토큰화 **고침**. |
| **신규: 사용 설명서 / 설정 찾기** | (2차 R2-HELP · R2-SET-1) | **맞음** | 설정 › 사용 설명서 카드 8장 + 도식 모달 2종, 사이드바 설정 찾기 팝오버 모두 `Paperlogy`/`Freesentation`·맞춤 아이콘·그래파이트/아일랜드 토큰을 따름. | **맞음**(스냅숏 24장 전부 넘침·잘림 0). |

---

## 3. 고친 뒤 재검사 결과 (`2026-11-27`)

| 검사 | 명령 | 결과 | 비고 |
|---|---|---|---|
| 단위 + 정적 검사 | `timeout 600 npm test` | **통과** (`ink-client` 36 + `ink-ui` 238 = **274건** 전부 통과) | 새 `hardcodedColors.test.js` 2건 포함 |
| Svelte/TS 타입 검사 | `npm run check` | **통과** (`@screen-ink/ui` 0 오류·0 경고, `@screen-ink/app` 0 오류·0 경고) | — |
| 색 대비 검사 | `node scripts/check-contrast.mjs` | **통과** (146쌍: 글자 최소 `4.71:1 ≥ 4.5`, UI 최소 `3.01:1 ≥ 3.0`) | `dark` ↔ `prefers-color-scheme: dark` 일치 확인 |
| 스냅숏 재검사 | `node scripts/qa-snapshots.mjs --only=snapshots` | **통과** (168장, 미발견 0, 콘솔/페이지 오류 0) | 아래 전후 비교 참조 |

### 스냅숏 전후 비교 (`G16` → `G18`)

| 지표 | 고치기 전 (`G16`) | 고친 뒤 (`G18`) | 변화 |
|---|---:|---:|---|
| 총 스냅숏 수 | 168장 | 168장 | 유지 |
| 넘침 (`overflows`) | 0장 | **0장** | 유지 |
| 잘림 (`truncations`, 말줄임) | 8장 (`settings-general` en 4, `first-step3` en 4) | **0장** | **-8장 (전부 해소)** |
| 경계 이탈 / 뷰포트 잘림 / 겹침 | 0 / 0 / 0 | **0 / 0 / 0** | 유지 |
| OS 표기 섞임 (화면 168장 + PDF 4조합) | 0건 | **0건** | 유지 |
| 단축키 PDF 검증 (4조합) | 4/4 `%PDF-` 정상 · OS 섞임 0 | **4/4 `%PDF-` 정상 · OS 섞임 0** | `macos/ko` 47,500 B · `macos/en` 35,991 B · `windows/ko` 46,028 B · `windows/en` 34,636 B |

---

## 4. 사람 눈으로 볼 것 / 남은 항목

1. **`FontPicker` 상단 섬 겹침 회피(선택 영역 상단 근처)**: 캡처 범위가 화면 위쪽에 있을 때 글꼴 팝오버가 상단 섬을 가리는 경우가 있어, 여백 부족 시 툴바 아래로 뒤집어 여는 위치 보정이 남아 있다.
2. **OS 실기 화면 확인(미리보기 범위 밖)**: 실제 투명/캡처 제외 창이 뜨는 S07 실캡처(멀티모니터·혼합 배율), S10 창 캡처 둥근 귀, S11 녹화 꺾쇠, S13 실시간 확대, S15 Windows 창 항상 위 고정은 실기 실행 시 눈으로 최종 확인이 필요하다.
