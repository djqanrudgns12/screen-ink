# 슉쇽 화면별 사용성 · 맥/Windows 표기 전역 감사 보고서

- 작성일: 2026-11-27
- 대상 브랜치: `feat/foundation`
- 기준 문서: [PRD v2.0](../PRD-2차-고도화.md) · 1차 UI(`packages/ink-ui`, `packages/ink-client`, `crates/ink-app`, `crates/ink-domain`, `crates/tauri-plugin-screen-ink`)
- 방법:
  1. Vite 미리보기(`npm run ui:dev`) + Playwright Chromium(2배 해상도 `deviceScaleFactor: 2`)으로 macOS / Windows 화면·상태별 스크린숏 39장 촬영(모든 PNG 32 KB ~ 316 KB, 400 KB 이하 준수)
  2. 촬영본 육안 검수 및 프론트엔드(`packages/ink-ui`, `packages/ink-client`)·백엔드(`crates/`) 코드 정밀 대조

---

## 1. 미리보기 실행법 및 `platform` 전환 시 주의점

`npm run ui:dev`(`packages/ink-ui`, 포트 `5180`)에서 쓸 수 있는 URL 쿼리 인자는 [main.js](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/preview/main.js#L8-L23), [Preview.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/preview/Preview.svelte#L35-L59), [DrawPreview.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/preview/DrawPreview.svelte#L28-L31)에서 읽는다.

| 인자 | 가능한 값 | 설명 |
|---|---|---|
| `?screen=` | `tray`, `settings`, `first`, `overlay`, `cheat`, `island`, `board` | 기본 `settings` |
| `?tab=` | `shortcuts`, `capture`, `draw`, `general` | `screen=settings`일 때 시작 탭 (기본 `shortcuts`, 참고: `info` 탭은 쿼리 미지원) |
| `?theme=` | `light`, `dark` | 기본 `first`/`cheat`는 `dark`, 나머지 `light` |
| `?platform=` | `windows`, `macos` | 기본 `windows`. `createFakeClient({ platform })`와 `DrawPreview`에 전달 |
| `?perm=` | `screen-off`, `screen-revoked`, `ax-off` | 맥 권한 시나리오(`M10`). `ax-off`는 현재 UI에 표시되지 않음 |
| `?firstRun=1` | `1` | `screen=settings` 위에서 첫 실행 시트를 함께 띄움 |
| `?chrome=0` | `0` | 상단 미리보기 툴바(`.pv-bar`) 숨김 |
| `?native=1` | `1` | `screen=overlay`에서 `nativeFrame=true` 전환 |
| `?evidence=1` | `1` | 숨은 `capture.evidence` 인식 근거 박스 표시 |
| `?draw=1` | `1` | `DrawPreview.svelte` 실행 (`&board=1`, `&apply=auto`, `&desync=0` 조합 가능) |

### `fake.js`의 `platform` 옵션 한계
`createFakeClient({ platform: 'macos' })`([fake.js:41](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L41))는 `shortcuts` 슬롯 조합을 `defaults-v1.json`의 `mac` 값으로 채우고 `windowsOnly`(`topmost`)를 제외할 뿐, 아래 항목들은 **`platform`을 무시하고 Windows 값을 그대로 반환**한다. 따라서 미리보기에서 `?platform=macos`로 찍으면 `설정 · 캡처·녹화`와 `설정 · 일반`의 경로·예외 앱·가져오기 미리보기·섬 이벤트가 Windows 그대로 나온다.
- `DEFAULT_SETTINGS['capture.excluded']`: `licecap.exe` ([fake.js:11](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L11))
- `DEFAULT_SETTINGS['output.folder']`: `C:\Users\…\Pictures\Shookshok` ([fake.js:12](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L12)), `folder.pick` 결과 `D:\수업자료\캡처` ([fake.js:580](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L580))
- `state().location.path`: `C:\Users\…\AppData\Roaming\Shookshok\settings.json` ([fake.js:113](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L113))
- `previewImport()` 경로·단축키: `C:\Users\…`, `Ctrl+Alt+F2` ([fake.js:123,268](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L123))
- `SUGGEST` 추천 키: `Ctrl+Alt+1` 등 Windows 조합만 존재 ([fake.js:7](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L7))
- 섬 이벤트 문구: `Alt 1`, `Ctrl+S`, `Ctrl+V`, `Alt+\`` 고정 ([fake.js:183,294,366,472](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L183))

---

## 2. 기지적 항목 3건 현황 확인

사용자가 이미 지목한 3건은 코드·스크린숏에서 아래와 같이 확인되었다(PRD v2.0 `R2-UI-TRAY`, `R2-UI-TB`, `R2-ANN`으로 반영 중).

| 기지적 항목 | 코드·화면 확인 결과 | 근거 |
|---|---|---|
| **트레이 팝오버가 가로로 김** | `.ink-popover` 너비가 `288px`로 고정되어 있음(PRD 목표 `248~260pt`). 특히 하단 푸터 `설정… ⌘ , \| 끝내기` 한 줄에 4개 요소가 몰려 좁아 보임 | [TrayPopover.svelte:116](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/components/TrayPopover.svelte#L116) · [tray-both-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/tray-both-mac.png) |
| **캡처 툴바 아이콘** | 14개 아이콘이 라벨 없이 나열되어 있고, 특히 `화면에 고정(핀)`·`모자이크` 아이콘이 처음 사용자에게 직관성이 약하며, 주석 8종+되돌리기/다시에는 툴팁 키 안내(`slot`)가 없음 | [CaptureToolbar.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/components/CaptureToolbar.svelte#L73-L127) · [overlay-confirmed-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/overlay-confirmed-mac.png) |
| **색 6개 · 굵기 3개뿐** | `ANNOT_COLORS` 6종(`R·Y·G·O·B·L`), `ANNOT_WIDTHS` 3종(`2·4·8px`), `MOSAIC_BLOCKS` 3종(`6·10·16px`)만 제공. 글자(T) 도구를 선택해도 pt 크기/글꼴 없이 선 굵기 알약만 표시됨 | [captureToolbar.js:145-155](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/captureToolbar.js#L145-L155) · [overlay-annotate-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/overlay-annotate-mac.png) |

---

## 3. 상위 15개 핵심 개선 사항 (기지적 3건 제외, 신규·심층 발굴)

| 순위 | 심각도 | 화면 | 문제 요약 | 고칠 안 | OS 해당 |
|---|---|---|---|---|---|
| **1** | **상** | S05 설정 · 일반 (`설정 위치`) | **`설정 위치 [열기]`를 누르면 설정 폴더가 아니라 캡처 저장 폴더(`folder.open`)가 열림** | `settingsFolder.open` 전용 액션을 분리해 `state.location.path`의 상위 폴더를 열도록 수정 | 공통 |
| **2** | **상** | S06 첫 실행 / S02 단축키 | **`[끄는 법]` 단추가 실제로는 아무 안내도 주지 않음**(`lib.rs`에서 `open_settings`만 호출 → 첫 실행 시트 안에서는 아무 변화 없음) | `snipaste.howto` 전용 안내 모달/시트(Snipaste 트레이 → Preferences → Hotkeys 끄기 단계 + 슉쇽 추천 키 원클릭 적용) 연결 | 공통 |
| **3** | **상** | S06 첫 실행 (macOS) | **손쉬운 사용(Accessibility) 권한 안내·요청이 누락됨**(`permissions.accessibility` 상태가 UI에 전혀 표시되지 않음, `screen`만 체크) | 맥 첫 실행 카드에 `화면 기록` + `손쉬운 사용` 두 권한을 각각 상태/요청 버튼으로 표시(PRD `R2-ONB-1~4` 반영) | macOS |
| **4** | **상** | S17 섬 (`result`) | **`max-width: 520px` 결과 카드에서 `한글·PPT에 Ctrl+V` 메타 글자가 오른쪽 `[저장]`/`[판서로]` 버튼 뒤로 겹쳐 잘림** | `.ink-island-result-body`에 `flex: 1; min-width: 0`, `.ink-island-result-meta`에 `overflow: hidden; text-overflow: ellipsis` 적용 + 버튼/칩 동시 3개일 때 여백 재조정 | 공통 |
| **5** | **상** | S03 캡처·녹화 (macOS) | **맥에서도 예외 앱 입력·검증이 `.exe`를 강제함**(`app.exe` 플레이스홀더, 미입력 시 자동 `.exe` 추가, Rust `settings.rs`에서도 `.exe` 미포함 시 오류) | 맥에서는 `.app`/번들 ID/프로세스명을 허용하고, 가능하면 실행 중 앱 목록에서 고르는 선택기 제공 | macOS |
| **6** | **상** | S02 설정 · 단축키 | **단축키 줄 클릭 후 포커스 링(`outline: 2px`)이 `.ink-group`(`overflow: hidden`)에 잘려 행 중간을 가로지르는 주황 실선처럼 보임** + 오류 시 추천 칩이 맥에서도 `Ctrl Alt 1`로 표시됨 | 포커스 링을 `inset` 링으로 바꾸고, 추천 칩을 `<Keycap {combo} {platform} />` 또는 `formatComboReadable(combo, platform)`로 렌더 | 공통 / macOS |
| **7** | **상** | S16 단축키 한 장 | **Windows `저장 · 빠른 저장` 라벨이 5개 키캡에 밀려 `저장 ...`으로 잘림**, 맥 `되돌리기 · 다시`는 키캡 5개(`⌘ Z ⇧ ⌘ Z`)가 구분선 없이 붙어 해독 불가 | 2개 조합이 한 줄에 들어갈 때 가운데 `·` 또는 `/` 구분자를 두고, 라벨 최소 너비 확보·짧은 이름("저장 · 빠른저장" 분리 등) 적용 | 공통 |
| **8** | **상** | S07·S08 캡처 오버레이 | **선택 범위 확정(`confirmed`) 후에도 상단 섬 힌트가 `휠 넓히기·좁히기 \| Tab 창`으로 그대로 남아 있고, 범위 모서리에 픽셀 크기 배지가 없음** | `confirmed` 진입 시 섬 힌트를 `방향키 1px · Enter 복사 · 도구 선택 · Esc 취소`로 전환하고, 선택 틀 좌상단에 `W × H` 배지 표시 | 공통 |
| **9** | **중** | S03·S04·S05·S06·S17·판서 | **박아 둔 OS 표기/문구 다수 혼재**(기존 5곳 외 `Realtek`, `트레이`, `F1·F2·F3 도움말·이름 바꾸기·찾기`, `PowerToys`, `UAC`, `PinView Ctrl+휠`, `DrawPreview` 도움말 등 총 34건) | `platform`별 카피/단축키 헬퍼(`spoken`/`t`)로 전면 통일하고 OS 표기 섞임 lint 도입 | macOS / Windows |
| **10** | **중** | S03 캡처·녹화 (`찾지 않을 앱`) | **`느림 · 창 단위 자동` 정적 뱃지가 바로 옆 삭제 가능한 칩(`licecap.exe ×`)과 같은 모양(`.ink-chip`)이라 클릭 가능해 보임** | 정적 안내 뱃지는 점선/무채색 텍스트 태그로 구분하고, "느림 · 창 단위 자동"의 의미(예: 120ms 초과 시 자동 전환)를 보조 설명으로 분리 | 공통 |
| **11** | **중** | S01 트레이 / S05 일반 | **트레이/설정에 `핀(고정한 캡처)`과 `창 항상 위(고정한 창)` 두 기능이 용어·아이콘 모두 겹쳐 혼동 유발**(트레이 둘 다 `pin` 아이콘, `모두 풀기` vs `모두 닫기`) | 캡처 이미지만 띄우는 기능은 `붙이기` 또는 `그림 핀` 아이콘 분리, 창 최상위 고정은 `창 맨 위에 고정`으로 명확히 구분 | Windows(특히)·공통 |
| **12** | **중** | S08 주석 옵션 (`AnnotateOptions`) | **옵션 알약이 선택한 도구(왼쪽)와 동떨어진 툴바 오른쪽 끝에 정렬**되고, 선 도구는 px 숫자가 없고 모자이크만 `10px`가 표시됨 | 옵션 알약 기준점을 선택한 도구 버튼 중앙 아래로 맞추거나 툴바 폭 전체에 맞게 정렬하고, 선/글자도 현재 px/pt 수치를 일관되게 표시 | 공통 |
| **13** | **중** | S12·S14 판서·칠판 (`DrawOverlay`) | **화면 위 클릭 가능한 도구/페이지/닫기 UI가 전혀 없음**(P·M·T·S·D·E, R·Y·G·O·B·L, Page, Esc를 외워야만 사용 가능) | 마우스로도 도구·색·굵기·쪽 이동·저장·복사·닫기를 누를 수 있는 보조 도구 막대 제공(PRD `R2-ANN-5`와 일치) | 공통 |
| **14** | **중** | S04 설정 · 판서·확대 | **설정 화면 내 상위-하위 종속 관계 및 인터랙티브 여부 불명확**: `도형 보정`이 `멈추면 도형으로`의 하위 옵션임이 시각적으로 안 드러나고, `칠판 미리 보기` 박스가 클릭 가능해 보임 | `도형 보정`을 `멈추면 도형으로` 바로 아래 들여쓰기/연결 표시로 묶고, `칠판 미리 보기`는 클릭으로 기본 칠판 바탕 선택 또는 `미리 보기` 라벨 명확화 | 공통 |
| **15** | **하** | S05 설정 · 일반 / S02 / S16 | **숨은 기능 발견성 & 조작 일관성 부족**: 설정 `모두 처음으로`만 버튼 없이 행 클릭, 단축키 `기본값` 버튼에 확인 시트 없음, S16 `단축키 한 장`에 마우스 `[×]` 닫기 버튼 없음, 캡처 중 `Shift` HEX/RGB 전환·`Alt` 자석 끔·`우클릭 손 모드` 등 숨은 조작이 초기 안내에서 안 보임 | `모두 처음으로`에 `[초기화]` 버튼 배치, `기본값` 복원은 1단 되돌리기 안내 또는 확인 시트 제공, `CheatSheet` 우측 상단 닫기 버튼 추가 | 공통 |

---

## 4. 화면별 사용성 감사 표

### 4.1 S01 트레이 팝오버 (`TrayPopoverScreen.svelte` · `TrayPopover.svelte` · `MenuRow.svelte`)
근거: [TrayPopoverScreen.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/TrayPopoverScreen.svelte), [TrayPopover.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/components/TrayPopover.svelte), [tray-both-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/tray-both-mac.png), [tray-both-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/tray-both-win.png)

| # | 문제 | 심각도 | 고칠 안 | OS 해당 |
|---|---|---|---|---|
| 1 | 하단 푸터 `.ink-foot-split`에서 `설정… ⌘ ,` 옆에 바로 `끝내기`가 같은 줄 30% 폭에 붙어 있어 좁고, `끝내기`가 설정 버튼의 보조 라벨처럼 보일 수 있음 | 중 | `설정…`과 `끝내기`를 두 행으로 분리하거나 구분선(`\|`)·정렬로 명확히 분리 | 공통 |
| 2 | `고정한 캡처`(`pinCount > 0`) 행이 클릭 시 `pins.closeAll`을 실행하는데, 바로 옆 `모두 닫기` 버튼과 동일한 동작을 중복 제공하며 개별 핀을 보거나 선택해 닫을 수 없음 | 중 | 행 클릭은 최근 핀 앞으로 가져오기 또는 목록 펼치기로 두고, 우측 버튼만 `모두 닫기`로 유지 | 공통 |
| 3 | Windows에서 `창 항상 위`와 `고정한 캡처`가 둘 다 `pin` 아이콘·비슷한 숫자 배지를 써서 헷갈림 (`tray-both-win.png`) | 중 | 창 고정 vs 캡처 이미지 붙이기 아이콘 분리 및 라벨 차별화 | Windows |
| 4 | 단축키가 꺼져 있을 때(`!hotkeysEnabled`) 키캡만 흐리게(`keyDim`) 될 뿐, 상단 `단축키 꺼짐` 상태 배지를 클릭해 바로 켤 수 없음(아래 스위치까지 시선 이동 필요) | 하 | 상단 `단축키 꺼짐` 배지 클릭 시 단축키를 바로 다시 켜도록 연결 | 공통 |
| 5 | 키보드 탐색 시 `ArrowUp`/`ArrowDown`만 지원하고 첫 글자 점프가 없음(한글 IME 보호 주석). 다만 `Switch`나 `모두 풀기`/`키 n개 겹침` 버튼으로의 Tab/화살표 이동 동선이 메뉴 행과 분리되어 있음 | 하 | 푸터 액션까지 일관된 키보드 순회 순서 보장 | 공통 |
| 6 | 맥에서 `저장 폴더 열기` 문구가 macOS 관습(`Finder에서 보기`)이 아니고 `설정…` 단축키 표기도 `mac`이 아닌 곳들과 어조 확인 필요 | 하 | 맥 `Finder에서 보기`, Windows `탐색기에서 열기` 분기 | macOS / Windows |

### 4.2 S02 설정 · 단축키 (`ShortcutsPage.svelte` · `Row.svelte` · `shortcutDisplay.js` · `fake.js`)
근거: [ShortcutsPage.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/ShortcutsPage.svelte), [Row.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/components/Row.svelte), [settings-shortcuts-global-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-shortcuts-global-mac.png), [settings-shortcuts-recording-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-shortcuts-recording-mac.png), [settings-shortcuts-inapp-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-shortcuts-inapp-mac.png)

| # | 문제 | 심각도 | 고칠 안 | OS 해당 |
|---|---|---|---|---|
| 1 | 행 클릭 시 `.ink-row-clickable.ink-focus`에 `:focus-visible` `outline: 2px solid var(--ink-accent)`가 적용되는데, 부모 `.ink-group`의 `overflow: hidden` 때문에 좌우 테두리가 잘려 행 아래에 **주황 가로줄이 그어진 것처럼** 보임 (`settings-shortcuts-recording-mac.png`) | 상 | `.ink-row-clickable` 포커스 링은 `outline-offset: -2px` 또는 `box-shadow: inset 0 0 0 2px var(--ink-accent)`로 변경 | 공통 |
| 2 | 겹침 오류 시 표시되는 추천 칩(`Row.svelte:130`)이 `combo.replaceAll('+', ' ')`만 써서 맥에서도 `Ctrl Alt 1`·`Ctrl Alt 2`로 표시됨 (`settings-shortcuts-recording-mac.png`) | 상 | 추천 칩에 `<Keycap {combo} {platform} />` 또는 `formatComboReadable(combo, platform)` 적용 | macOS |
| 3 | 상단 겹침 배너 `[끄는 법]` 버튼이 `client.run('snipaste.howto')`를 호출하지만 실제 구현(`lib.rs:1020`)은 `open_settings(&app)`만 실행해 이미 설정 창 안에 있는 사용자는 아무 반응이 없음 | 상 | 대체 앱(Snipaste·PowerToys 등)별 끄는 법 안내 시트/모달 구현 | 공통 |
| 4 | 단축키 녹화 대기 문구(`키를 누르세요 · Esc 취소`)에 **`Backspace로 비우기` 안내가 빠져 있어** 사용자가 단축키를 비우는 방법을 알 수 없음 (`shortcutDisplay.js:111`) | 중 | 녹화 중 보조 문구를 `새 키를 누르세요 · ⌫ 비우기 · Esc 취소`로 개선 (PRD `R2-KEY-REC-5`) | 공통 |
| 5 | 그룹 제목 우측 `[기본값]` 버튼 클릭 시 **확인 없이 즉시 해당 그룹의 모든 커스텀 단축키를 초기화**함(실수 시 되돌리기 안내 섬도 없음) | 중 | 그룹 초기화 시 확인 시트 또는 결과 섬 `[되돌리기]` 제공(`resetShortcutGroup`은 현재 `history`에 안 쌓이는 구현도 점검 필요) | 공통 |
| 6 | `화면 안` 탭에 41개 항목이 한꺼번에 스크롤되는데 검색/필터가 없고, `후속` 딱지(`later: true`)가 붙은 항목(`화면에 붙이기`, `핀 숨기기·보이기`, `타이머`)을 미리 설정해도 실제로는 `"준비 중이에요"`만 나와 혼동됨 | 중 | 상단 설정 검색 도입(PRD `R2-SET-1`) 및 `후속` 항목은 hover/안내에 출시 예정임을 명시하거나 비활성 구분 | 공통 |
| 7 | `어디서나` 탭의 `Snipaste가 F1·F3 키 3개를 먼저 쓰고 있어요` 배너는 행 옆 주황 점(`●`)으로만 충돌을 표시하고 행 자체에 툴팁/설명이 없어 왜 `직전 범위 바로 캡처(⇧F1)`에도 점이 찍히는지 즉시 이해하기 어려움(`bannerApp === owner`일 때 행 경고를 숨김 — `shortcutDisplay.js:115`) | 하 | 점(`●`)에 마우스 올릴 때 `Snipaste와 겹침` 툴팁 표시 | 공통 |

### 4.3 S03 설정 · 캡처·녹화 (`CapturePage.svelte` · `template.js`)
근거: [CapturePage.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/CapturePage.svelte), [settings-capture-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-capture-mac.png), [settings-capture-bottom-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-capture-bottom-mac.png)

| # | 문제 | 심각도 | 고칠 안 | OS 해당 |
|---|---|---|---|---|
| 1 | `찾지 않을 앱` 입력·검증이 맥에서도 `.exe`를 강제함(`placeholder="app.exe"`, `addExcluded()`에서 `.exe` 자동 붙임, Rust `settings.rs:198`에서도 `!lower.ends_with(".exe")` 에러) | 상 | 플랫폼 분기: Windows는 `.exe`, macOS는 `.app`·번들 ID·실행 파일 이름 허용 및 실행 중 앱 선택 드롭다운 제공 | macOS |
| 2 | `모서리 자석` 보조 문구가 `Alt를 누르는 동안 끔`으로 박혀 있음(맥도 `⌥`) | 중 | 맥 `⌥를 누르는 동안 끔`, Windows `Alt를 누르는 동안 끔` | macOS |
| 3 | `마이크 장치` 기본 예시 목록에 `내장 마이크(Realtek)`가 박혀 있음(`CapturePage.svelte:22`) | 중 | OS 오디오 입력 장치 API에서 실제 장치명을 읽거나 중립 예시(`내장 마이크`) 사용 | macOS |
| 4 | `느림 · 창 단위 자동` 뱃지가 바로 왼쪽 삭제 가능 칩(`licecap.exe ×`)과 동일한 `.ink-chip` 스타일이라 클릭하면 지워질 것처럼 보임 (`settings-capture-mac.png`) | 중 | 자동 전환 안내는 칩이 아닌 행 부제목(`sub`)으로 이동하거나 테두리 없는 정보 배지로 분리 | 공통 |
| 5 | `파일 이름` 입력 칸에 쓸 수 있는 치환 토큰(`{yyyy}`, `{MM}`, `{dd}`, `{HH}`, `{mm}`, `{ss}`, `{n}`, `{앱}`) 안내가 전혀 없어 사용자가 어떤 변수를 쓸 수 있는지 모름 | 중 | 입력 칸 아래 또는 `?` 도움말로 사용 가능한 토큰 칩(클릭 시 삽입) 제공 | 공통 |
| 6 | `마우스 커서도 찍기` 보조 문구 `캡처 중 \` 키`는 사용자가 `c-cursor` 단축키를 바꿨을 때도 반영되지 않는 하드코딩 문구임 | 하 | `state.shortcuts`의 `c-cursor` 현재 값을 읽어 동적 표기 | 공통 |
| 7 | `마이크 소리`를 켠 뒤에는 스위치를 껐다 다시 켜야만 장치 선택 시트(`MIC_DEVICES`)를 열 수 있어, 켜진 상태에서 마이크 장치를 바로 바꾸는 버튼이 없음 | 하 | 켜져 있을 때 장치 이름 옆 `[바꾸기]` 버튼 또는 드롭다운 노출 | 공통 |
| 8 | `캡처 기록` 행에 `20개 보관` 숫자만 있고 변경 컨트롤이 없으며, `기록 지우기`는 확인 없이 바로 지움 | 하 | 보관 개수 설정 또는 지우기 후 `[되돌리기]`/안내 강화 | 공통 |

### 4.4 S04 설정 · 판서·확대 (`DrawZoomPage.svelte` · `settingsMeta.js`)
근거: [DrawZoomPage.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/DrawZoomPage.svelte), [settingsMeta.js](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/settingsMeta.js), [settings-draw-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-draw-mac.png), [settings-draw-bottom-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-draw-bottom-mac.png)

| # | 문제 | 심각도 | 고칠 안 | OS 해당 |
|---|---|---|---|---|
| 1 | 같은 페이지 안에서 칠판 미리 보기(`⌥3`)는 맥 기호로 나오는데, 바로 위 `굵기`(`Ctrl+휠`), `나갈 때 보관`(`Alt+Shift+1로 다시 열기`), `멈추면 도형으로`(`Ctrl+Z = 원래 획`)는 Windows 키가 그대로 나옴 (`settings-draw-mac.png`) | 상 | 모두 `platform` + 현재 `shortcuts` 값(`d-wup`/`d-wdown`, `restore`, `d-undo`)을 읽어 표기 | macOS |
| 2 | `도형 보정` 행이 바로 위 `멈추면 도형으로` 스위치를 켜고 끄는 것에 종속되지만(끄면 흐려짐), 켜졌을 때는 독립 행처럼 보여 두 행의 관계가 직관적이지 않음 | 중 | `도형 보정`을 `멈추면 도형으로`의 하위 행(들여쓰기/라벨 `바꾸는 방식`)으로 묶거나 한 묶음 내 연결 표시 | 공통 |
| 3 | `칠판 미리 보기`의 `W 흰색` / `K 검은색` 박스가 버튼처럼 생겼으나 클릭해도 반응이 없음(단순 미리보기) | 중 | 클릭 시 칠판 기본 바탕을 흰/검으로 바꾸게 하거나, `미리 보기` 카드 형태(비대화형 프레임)로 명확히 구분 | 공통 |
| 4 | `처음 색` 6개가 판서 기본 색만 노출하고, `R·Y·G·O·B·L` 영문자만 적혀 있어 접근성·직관성이 떨어짐(PRD `R2-ANN-1`로 확장 예정) | 중 | 색 이름 툴팁 강화 및 확장 팔레트/내 색 연동 | 공통 |
| 5 | `실시간 확대 › 커서 따라가기` 옵션 이름 `마우스 위치대로` / `가운데로` / `바로`가 초보자에게 어떤 차이인지 감이 안 옴(`가운데로`와 `바로`가 무엇을 기준으로 다른지 불명확) | 하 | 짧은 부제목으로 차이 설명(예: `창 가장자리 가까이 가면 밀림` / `항상 커서가 화면 가운데`) | 공통 |

### 4.5 S05 설정 · 일반 및 정보 (`GeneralPage.svelte` · `SettingsWindow.svelte`)
근거: [GeneralPage.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/GeneralPage.svelte), [SettingsWindow.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/SettingsWindow.svelte), [settings-general-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-general-mac.png), [settings-general-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-general-win.png), [settings-general-dark-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-general-dark-mac.png), [settings-info-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-info-mac.png)

| # | 문제 | 심각도 | 고칠 안 | OS 해당 |
|---|---|---|---|---|
| 1 | **`설정 위치 [열기]` 버튼이 `folder.open`(캡처 저장 폴더)을 호출**해 설정 파일(`settings.json`) 위치가 아닌 사진 폴더를 여는 기능 버그 (`GeneralPage.svelte:185`, `lib.rs:1040`) | 상 | `settingsFolder.open` 액션 신설해 설정 JSON이 있는 폴더를 열도록 수정 | 공통 |
| 2 | 맥에서 `로그인하면 실행` 보조 문구가 `트레이에만 조용히`로 표기됨 (`GeneralPage.svelte:120`) | 중 | 맥 `메뉴 막대에만 조용히`, Windows `트레이에만 조용히`로 분기 | macOS |
| 3 | `모두 처음으로` 행만 오른쪽 버튼 없이 행 전체 클릭(`clickable={true}`)으로 동작해, 바로 위 3개 행(`열기`, `내보내기/가져오기`, `폴더 열기`)과 조작 일관성이 깨짐 | 중 | 오른쪽에 `[초기화]`(위험 스타일) 버튼을 두고 행 클릭과 함께 작동 | 공통 |
| 4 | 설정 창 내 `Cmd+Z` / `Ctrl+Z` 실행 취소(`SettingsWindow.svelte:105`)가 존재하지만 UI 어디에도 안내되지 않아 사용자가 발견하기 어려움(그리고 `resetShortcutGroup`은 undo 기록에 안 쌓임) | 중 | 변경 시 하단 토스트/섬 또는 헤더에 `⌘Z 되돌리기` 힌트 제공 | 공통 |
| 5 | 다크 테마(`settings-general-dark-mac.png`)에서 사이드바 활성 탭(`#2C2E35`)과 배경(`#16171A`) 대비가 라이트 테마(`#1A1A1A` on `#EAEAE6`)보다 약해 활성 탭 구분이 흐릿함 | 하 | 다크 테마 `--ink-nav-on-bg` 대비 보강 또는 왼쪽 주황 인디케이터 바 추가 | 공통 |
| 6 | `정보` 탭(`settings-info-mac.png`)의 `오픈소스 고지`가 `IBM Plex · Svelte · bits-ui · Tauri` 한 줄 텍스트뿐이고 클릭해 전문을 볼 수 없음(PRD `R2-FONT-2` 라이선스 고지 시 시트/모달 필요) | 하 | `오픈소스 고지 [보기]` 시트 추가 및 문제 보고용 기록 묶기(`R2-DIAG-1`) 배치 | 공통 |

### 4.6 S06 첫 실행 시트 (`FirstRunSheet.svelte`)
근거: [FirstRunSheet.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/FirstRunSheet.svelte), [firstrun-conflict-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/firstrun-conflict-mac.png), [firstrun-clean-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/firstrun-clean-mac.png), [firstrun-perm-screen-off-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/firstrun-perm-screen-off-mac.png)

| # | 문제 | 심각도 | 고칠 안 | OS 해당 |
|---|---|---|---|---|
| 1 | **맥 손쉬운 사용(Accessibility) 권한 카드/행 누락**: `permissions.accessibility` 상태가 false여도 `FirstRunSheet`에 전혀 나오지 않음(`needsScreen`만 체크) | 상 | PRD `R2-ONB-1~4` 5장 카드 구조에서 `화면 기록`과 `손쉬운 사용`을 각각 안내하고 1초 폴링으로 자동 체크 | macOS |
| 2 | **`[끄는 법]` 버튼이 실제로 동작하지 않음**: 첫 실행 시트 안에서 눌러도 `lib.rs:1020`이 `open_settings`만 실행하므로 뒤에 있는 설정 창만 열리고 시트에 가려 아무 일도 안 일어남 | 상 | `snipaste.howto` 클릭 시 해당 앱별 안내 시트를 바로 띄워줌 | 공통 |
| 3 | **맥에서 OS 어휘/맥락 부적합**: ① `트레이에 있다가`(`:100`), ② 겹침 없을 때 `F1·F2·F3을 쓰는 동안 다른 앱의 도움말·이름 바꾸기·찾기가 막혀요`(`:168`, Windows 탐색기 기준 설명), ③ `키보드 설정에서 '표준 기능 키'를 켜요`(`:175`, macOS 정식 경로 `시스템 설정 › 키보드 › 키보드 단축키 › 기능 키` 누락) | 상 | macOS 전용 문구로 분기(메뉴 막대, macOS 기능 키 경로, macOS 기본 키 안내) | macOS |
| 4 | 권한 꺼짐 + 단축키 겹침 동시 발생 시(`firstrun-perm-screen-off-mac.png`) 노란 경고 배너 2개가 겹쳐 떠 시선이 분산되고, Esc/바깥 클릭으로 시트를 실수로 닫아 권한 없이 넘어갈 위험 있음 | 중 | 단계형 카드(PRD `R2-ONB`)로 한 번에 하나씩 집중시키고 탈출구는 확인 한 번 뒤 허용 | macOS(특히) |
| 5 | `화면 기록 켜기` 클릭 후 `켰으면 다시 시작`으로 버튼 라벨만 바뀌고, 실제 macOS 시스템 설정에서 켠 뒤에도 사용자가 돌아와 직접 눌러야 하는지/자동 반영되는지 안내가 없음 | 중 | 1초 폴링 반영 + `켜고 나서 아래 [다시 시작]을 눌러 주세요` 1줄 안내 | macOS |

### 4.7 S07·S08 캡처 오버레이 · 선택 툴바 · 주석 옵션 (`CaptureOverlay.svelte` · `CaptureToolbar.svelte` · `AnnotateOptions.svelte`)
근거: [CaptureOverlay.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/CaptureOverlay.svelte), [AnnotateOptions.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/components/AnnotateOptions.svelte), [overlay-confirmed-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/overlay-confirmed-mac.png), [overlay-annotate-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/overlay-annotate-mac.png), [overlay-mosaic-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/overlay-mosaic-mac.png)

| # | 문제 | 심각도 | 고칠 안 | OS 해당 |
|---|---|---|---|---|
| 1 | **범위 확정(`confirmed`) 후에도 상단 섬 둘째 줄 힌트가 `휠 넓히기·좁히기 \| Tab 창`으로 그대로 남아 있음** — 확정 후엔 휠/Tab이 동작하지 않는데 잘못된 조작법을 안내(`overlay-confirmed-mac.png`) | 상 | `phase === 'confirmed'`일 때 섬 힌트를 `방향키 1px · 툴바 도구 · Enter 복사 · Esc 취소`로 동적 변경 | 공통 |
| 2 | **선택 틀(SelectionFrame) 모서리에 픽셀 크기 배지(`W × H`)가 없음** — 크기를 보려면 화면 맨 위 섬까지 시선을 옮겨야 함 | 상 | 범위 좌상단(화면 상단 밖이면 범위 안 좌상단)에 `581 × 301` 크기 배지 표시(PRD `R2-UI-TB-3`) | 공통 |
| 3 | **주석 도구 8종 + 실행 취소/다시 실행 버튼 툴팁에 단축키 안내가 없음**(`CaptureOverlay.svelte:1055`에서 `TOOLBAR_ACTIONS`와 `TOOLBAR_HISTORY`만 `toolbarTips`에 넣고 도구 8종은 누락, 게다가 `AnnotateOptions` 툴팁은 `Ctrl+휠` 고정) | 중 | 모든 도구 버튼에 이름 + 단축키(`P`, `M`, `T` 등) 0.4초 말풍선 표시 및 OS 맞춤 `⌘+휠`/`Ctrl+휠` | 공통 / macOS |
| 4 | **`AnnotateOptions` 알약이 툴바 오른쪽 끝에 고정 정렬**(`placeToolOptions`)되어 선택한 도구(왼쪽)와 멀리 떨어짐. 또한 선 도구는 px 숫자 표시가 없는데 모자이크만 `10px`가 붙어 불일치(`overlay-annotate-mac.png` vs `overlay-mosaic-mac.png`) | 중 | 옵션 패널을 선택한 도구 위치 아래/위로 맞추고, 선/형광펜/글자도 현재 px·pt 숫자와 미리보기 표시(PRD `R2-ANN-3`) | 공통 |
| 5 | **글자(T) 도구를 골라도 `AnnotateOptions`가 선 굵기 `2·4·8px` 알약을 띄움**(`annotOptionsKind`가 `mosaic` 외 모두 `stroke`로 분기 — `captureToolbar.js:190`) | 중 | 글자 도구 선택 시 글꼴 선택(Aa) + 글자 크기(pt) + 굵게 패널로 분기(PRD `R2-TXT-1~2`) | 공통 |
| 6 | **주석 도구가 꺼져 있는 확정(`confirmed`) 상태에서도 툴바 밖으로 마우스가 나가면 확대경이 계속 따라다님** — 이미 범위가 확정돼 픽셀 조정이 아닌 툴바/도구 클릭 직전에 확대경이 시야를 가림 | 하 | `confirmed` 상태에서는 모서리 핸들 드래그 중이거나 `C` 색 복사 의도가 아닐 때 확대경을 숨기거나 토글 | 공통 |
| 7 | **주석을 그린 뒤 `[취소]`/`Esc`를 눌러도 실수 방지(되돌리기)가 없음** — 공들여 그린 주석이 한 번의 Esc로 즉시 사라짐 | 하 | 주석 1개 이상일 때 닫기 시 섬에 `[되살리기]` 5초 제공(PRD `R2-UNDO-1`) | 공통 |

### 4.8 S12·S14 판서·칠판 (`DrawOverlay.svelte` · `DrawCanvas.svelte` · `DrawPreview.svelte`)
근거: [DrawOverlay.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/DrawOverlay.svelte), [DrawPreview.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/preview/DrawPreview.svelte), [draw-canvas-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/draw-canvas-mac.png), [draw-board-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/draw-board-mac.png)

| # | 문제 | 심각도 | 고칠 안 | OS 해당 |
|---|---|---|---|---|
| 1 | **실제 판서/칠판 화면(`DrawOverlay`)에 클릭 가능한 도구 막대가 없음**: 펜·형광펜·글자·고르기·지우개·모두 지우기·되돌리기·복사·저장·종료를 모두 단축키로만 조작해야 함(상단 섬 색 점 hover 시 색만 선택 가능) | 상 | 캡처 툴바와 동일한 부품을 공유하는 접이식/고정형 판서 도구 막대 제공(PRD `R2-ANN-5`) | 공통 |
| 2 | **`DrawPreview.svelte` `?` 도움말 바에 Windows 키가 하드코딩**되어 맥에서도 `Ctrl 사각 · Ctrl+Shift 화살표 · +Alt 점선 · Ctrl+휠 굵기 · Ctrl+Z/Y`로 표시됨 (`draw-canvas-mac.png`), 또한 `P M T S D E 도구` 가독성이 떨어짐 | 중 | `DrawPreview` 도움말을 `platform`에 맞춰 분기하거나 `CheatSheet` 컴포넌트를 그대로 재사용 | macOS |
| 3 | **칠판 모드(`draw-board-mac.png`)에서 페이지(`1 / 1`) 이동/추가 UI가 보이지 않음**: 상단 섬에 `1 / 1` 숫자만 뜨고 클릭으로 페이지를 넘기거나 새 칠판을 여는 버튼이 없음 | 하 | 칠판 섬/도구 막대에 `< 1 / 1 > +` 쪽 전환 컨트롤 노출 | 공통 |

### 4.9 S16 단축키 한 장 (`CheatSheet.svelte` · `cheatSheet.js`)
근거: [CheatSheet.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/CheatSheet.svelte), [cheatSheet.js](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/cheatSheet.js), [cheatsheet-draw-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/cheatsheet-draw-mac.png), [cheatsheet-capture-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/cheatsheet-capture-win.png)

| # | 문제 | 심각도 | 고칠 안 | OS 해당 |
|---|---|---|---|---|
| 1 | **Windows 캡처 한 장(`cheatsheet-capture-win.png`)에서 `저장 · 빠른 저장` 라벨이 `저장 ...`으로 말줄임 잘림 발생** — 우측 키캡 5개(`Ctrl S Ctrl Shift S`)가 행 폭을 과도하게 차지함 | 상 | `저장`과 `빠른 저장`을 두 줄로 분리하거나, 키캡 묶음 사이에 `/` 구분 + 라벨 폭 보장 | Windows(특히)·공통 |
| 2 | **맥 `되돌리기 · 다시`(`⌘ Z ⇧ ⌘ Z`)와 `저장 · 빠른 저장`(`⌘ S ⇧ ⌘ S`)에서 키캡 5개가 구분선 없이 붙어 어디서 끊어 읽어야 할지 모름** (`cheatSheet.js:40-50`의 묶음 최적화가 Windows 2키-2키에만 작동) | 상 | 두 키 조합이 한 줄일 때 키캡 그룹 사이에 작은 `·` 또는 `/` 구분자 삽입 | macOS(특히)·공통 |
| 3 | **키캡 표현 규칙 불일치**: `1px 늘리기`는 `[⌘] [방향키]`, `아래 앱 스크롤`은 `[⇧] [휠]`로 분리했는데, `확대 · 굵기`는 `[휠] [⌘ 휠]`로 한 키캡 안에 합침. `점선으로`는 `[+ ⌥]`가 한 키캡에 들어가 `+` 키를 누르는 것처럼 오독됨 | 중 | 수식키+입력은 `[⌘] [휠]`처럼 분리하고, `+ ⌥`는 키캡 밖 접두 텍스트 `+` + `[⌥]` 키캡으로 분리 | 공통 |
| 4 | **`흰 · 검은 칠판` 라벨(`shortLabel: '흰'`)이 어색하고, `색` 행(`R Y G O B L`)이 색상 점 없이 글자만 나열되어 직관성이 떨어짐** | 중 | 라벨 `흰 칠판 · 검은 칠판` 또는 `칠판 (흰 · 검)`으로 보정, `R Y G O B L` 키캡에 작은 색 점 추가 | 공통 |
| 5 | **우측 상단에 닫기(`[×]`) 버튼이 없어** 마우스 전용 사용자는 `아무 키나 누르면 닫혀요` 문구만 보고 빈 바깥 영역 클릭을 찾게 됨 | 하 | 헤더 오른쪽에 `[×]` 버튼 추가 | 공통 |

### 4.10 S17 섬 (`Island.svelte` · `IslandHost.svelte` · `islandQueue.js` · `Preview.svelte`)
근거: [Island.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/components/Island.svelte), [island-grid-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/island-grid-mac.png)

| # | 문제 | 심각도 | 고칠 안 | OS 해당 |
|---|---|---|---|---|
| 1 | **`result` 카드에서 `한글·PPT에 Ctrl+V` 텍스트가 오른쪽 `[저장]`/`[판서로]`/`[#FF9F0A 복사]` 버튼 뒤로 겹쳐 잘림**(`island-grid-mac.png` 상단 우측 및 좌측 두 번째) | 상 | `.ink-island-result-body`에 `flex: 1; min-width: 0; overflow: hidden;`과 `.ink-island-result-meta`에 `text-overflow: ellipsis` 적용 | 공통 |
| 2 | **`Preview.svelte`의 섬 전시 그리드 전체가 `platform` 인자를 무시하고 Windows 키로 하드코딩**되어 있음(`Alt F1`, `Alt 1`, `Ctrl+S`, `Ctrl+V`, `Alt Shift D`, `Alt \``, `UAC · 잠금`) | 중 | `Preview.svelte:398-498`에 `platform={initialPlatform}` 전달 및 OS별 문구 분기 | macOS |
| 3 | **다국어(영어) 확장 시 `max-width: 520px` 고정 안에서 버튼 2~3개 + 썸네일 + 메타 텍스트가 더 쉽게 넘침** (`Island.svelte:494`) | 중 | 결과 카드(`data-result-card`)에서는 `detail` 축약 또는 보조 버튼 키캡 생략 임계값 강화 | 공통 |

### 4.11 S08 캡처 핀 · S11 녹화 프레임 · S13 실시간 확대 HUD (`PinView.svelte` · `RecordFrame.svelte` · `ZoomHud.svelte`)
근거: [PinView.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/PinView.svelte), [RecordFrame.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/RecordFrame.svelte), [ZoomHud.svelte](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/ZoomHud.svelte)

| # | 문제 | 심각도 | 고칠 안 | OS 해당 |
|---|---|---|---|---|
| 1 | **`PinView.svelte:46`에서 투명도 조절이 `if (e.ctrlKey)`로만 동작**(`onKey`는 `e.ctrlKey \|\| e.metaKey` 지원). 맥 사용자는 `⌘+휠`이 안 먹고 `Control+휠`만 동작 | 중 | `if (e.ctrlKey \|\| e.metaKey)`로 양쪽 지원(단, macOS 트랙패드 핀치 줌 제스처가 `ctrlKey: true`로 오는 점 고려해 감도 확인) | macOS |
| 2 | **캡처 핀 조작법 발견성 부족**: 호버 시 1px 주황 테두리만 진해질 뿐 `더블클릭/Esc 닫기 · 휠 확대 · ⌘/Ctrl+휠 투명도 · 우클릭 메뉴` 힌트가 전혀 없어 초보자가 닫거나 크기를 조절할 줄 모름 | 중 | 핀 위 첫 호버 시 하단에 작은 닫기(`[×]`)·`더블클릭 닫기` 힌트 1.5초 표시 또는 우측 상단 호버 닫기 버튼 | 공통 |
| 3 | **`ZoomHud.svelte` 위치 지도(`MiniMap`)는 보이지만 실시간 확대 중 조작 안내가 진입 시 상단 섬에만 잠깐 뜸** — 특히 `Shift+휠 = 아래 앱 스크롤` 같은 중요 조작이 잊히기 쉬움 | 하 | 실시간 확대 중 `CheatSheet`(`?`) 또는 섬 힌트로 항상 재확인 가능하도록 안내 강화 | 공통 |

---

## 5. 화면 간 용어 불일치 및 숨은 기능 발견성

### 5.1 같은 기능인데 화면/코드마다 이름이 다른 곳

| 기능/개념 | 표기 A | 표기 B | 표기 C | 권장 통일안 (PRD v2.0 용어집 방향) |
|---|---|---|---|---|
| **고정한 캡처 이미지(S08 핀)** | `화면에 고정` (툴바 툴팁 [captureToolbar.js:19](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/captureToolbar.js#L19)) | `고정한 캡처` (트레이 [TrayPopoverScreen.svelte:172](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/TrayPopoverScreen.svelte#L172)) | `화면에 붙이기` (단축키 카탈로그 [catalog-v1.json:166](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/shortcuts/catalog-v1.json#L166)) | **화면에 고정** (또는 `화면에 붙이기`로 일관 통일, `창 항상 위`와 구분) |
| **실행 취소 / 되돌리기** | `실행 취소` (캡처 툴바 [captureToolbar.js:140](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/captureToolbar.js#L140)) | `되돌리기` (단축키 카탈로그 `d-undo`, S16 한 장, 섬 [IslandHost.svelte:271](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/IslandHost.svelte#L271)) | `모두 처음으로` / `되돌리기` (일반 초기화 시트 [GeneralPage.svelte:218,281](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/GeneralPage.svelte#L218)) | 작업 취소는 **되돌리기**, 전체 초기화는 **모두 초기화**로 구분 |
| **다시 실행 / 다시 하기** | `다시 실행` (캡처 툴바 [captureToolbar.js:141](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/captureToolbar.js#L141)) | `다시 하기` (단축키 카탈로그 `d-redo`) | `다시` (S16 한 장 [cheatSheet.js:196](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/cheatSheet.js#L196)) | **다시 실행** 또는 **다시 하기**로 통일 |
| **타원 / 원** | `타원` (캡처 툴바 [captureToolbar.js:126](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/captureToolbar.js#L126)) | `원` (S16 한 장 [cheatSheet.js:210](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/cheatSheet.js#L210)) | — | 자유 비율이면 **타원**(또는 도형 안내에 **원·타원**) |
| **창 ↔ UI 요소 전환** | `창 ↔ UI 요소 찾기 전환` (카탈로그 [catalog-v1.json:193](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/shortcuts/catalog-v1.json#L193)) | `UI 요소까지 찾기` (설정 캡처 [CapturePage.svelte:205](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/CapturePage.svelte#L205)) | `창 ↔ 요소 전환` (S16 [cheatSheet.js:92](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/cheatSheet.js#L92)) | 쉬운 말: `창` / `화면 조각`(또는 `세부 영역`)으로 통합(PRD `R2-WORD-0`) |
| **오른쪽 클릭 메뉴** | `트레이 메뉴` (트레이/오류 문구 [service.rs:320,564](file:///Users/bodyking/Desktop/Architecture/screen-ink/crates/ink-app/src/service.rs#L320)) | `슉쇽 메뉴` (aria-label [TrayPopoverScreen.svelte:59](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/TrayPopoverScreen.svelte#L59)) | — | 맥 `메뉴 막대 아이콘`, Windows `트레이 아이콘`으로 구분 |
| **화면 녹화** | `화면 녹화` (트레이 [TrayPopoverScreen.svelte:110](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/TrayPopoverScreen.svelte#L110)) | `화면 녹화 시작·중지` (단축키 카탈로그 `record`) | `녹화 범위` (오버레이/섬 [Preview.svelte:363](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/preview/Preview.svelte#L363)) | **화면 녹화**로 통일 |
| **직전 판서** | `직전 판서 열기` (트레이·단축키 `restore`·S16) | `직전 판서 불러오기` (카탈로그 `d-restore` [catalog-v1.json:454](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/shortcuts/catalog-v1.json#L454)) | `이전 판서` (판서 섬 힌트 [draw.rs:949](file:///Users/bodyking/Desktop/Architecture/screen-ink/crates/ink-app/src/draw.rs#L949)) | **이전 판서 열기**로 통일 |

### 5.2 숨은 기능 / 발견성이 떨어지는 기능 목록

| 기능 | 현재 접근 방법 | 발견성 문제 | 개선 안 |
|---|---|---|---|
| **단축키 비우기 (`Backspace`)** | 단축키 녹화 상태에서 `Backspace` 누름 | 화면 문구(`키를 누르세요 · Esc 취소`)에 비우기 안내가 없음 | `새 키를 누르세요 · ⌫ 비우기 · Esc 취소` 표기 |
| **설정 창 안 `⌘Z` / `Ctrl+Z` 실행 취소** | 설정 창 안에서 `⌘Z` / `Ctrl+Z` 누름 | UI 어디에도 되돌릴 수 있다는 표시/버튼 없음 | 설정 변경 시 토스트 또는 헤더 되돌리기 버튼/힌트 |
| **색 복사 & HEX↔RGB 전환 (`C` / `Shift`)** | 캡처 중 `Shift` = 포맷 전환, `C` = 복사 | 확대경에 `C 복사`만 있고 `Shift` 전환 안내는 전무 | 확대경 보조 줄 또는 캡처 `?` 한 장에 `Shift HEX/RGB` 추가 |
| **모자이크·선 굵기 미세 조절 (`⌘/Ctrl+휠`)** | 툴바 옵션 버튼 hover 툴팁에만 표기 | 터치/트랙패드/마우스 사용자가 눈으로 볼 수 없음 | 슬라이더 UI 도입(PRD `R2-ANN-2`) + `[` `]` 키 병행 |
| **판서 중 색 점 팔레트 펼치기** | 상단 섬의 작은 색 점에 마우스 올리기 | 판서 중 색 점에 마우스를 올릴 수 있다는 단서가 없음 | 판서 도구 막대(PRD `R2-ANN-5`)로 전면 노출 |
| **확대 중 아래 앱 스크롤 (`Shift+휠`)** | 실시간 확대 중 `Shift+휠` | 처음 3회 뒤 섬 둘째 줄이 사라지면 안내가 안 보임 | `?` 단축키 한 장 및 사용 설명서 탭(PRD `R2-HELP`)에 강조 |
| **캡처 결과 `[판서로]` 이어받기 (`capture.toDraw`)** | 결과 섬이 떠 있는 1.2초 안에 클릭 또는 `⌥1`/`Alt+1` | 1.2초 안에 누르지 않으면 사라지며 설정/도움말에 설명 없음 | 결과 섬 힌트 및 사용 설명서 카드에 소개 |
| **캡처 핀 투명도 & 크기 조절 (`⌘/Ctrl+휠`, `휠`)** | 고정한 캡처 위에서 휠 / 수정키+휠 | 핀 창에 아무런 컨트롤·툴팁이 없음 | 첫 호버 힌트 + 우클릭 메뉴 항목으로 제공 |

---

## 6. OS별 표기·동작 문제 전체 목록 (파일:줄)

> 기존에 알려진 5곳(`settingsMeta.js` `Ctrl+Z`, `DrawZoomPage.svelte` `Ctrl+휠`·`Alt+Shift+1`, `AnnotateOptions.svelte` `Ctrl+휠`, `CapturePage.svelte` `Alt`·`.exe`)을 포함해, 코드와 미리보기에서 새로 확인한 모든 OS 고정 표기·동작 차이를 모았다. **총 34건** (앱 코드/UI 24건 + 미리보기/fake 10건).

### 6.1 앱 UI · 모델 · Rust 백엔드 (24건)

| # | 파일:줄 | 현재 표기 / 동작 | 문제 (맥 / Windows) | 수정 안 |
|---|---|---|---|---|
| 1 | [settingsMeta.js:106](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/settingsMeta.js#L106) | `'0.5초 · Ctrl+Z = 원래 획'` | **(기지적)** 맥에서 `Ctrl+Z` 고정 표기 | `platform === 'macos' ? '⌘Z' : 'Ctrl+Z'` (또는 `d-undo` 현재 키) |
| 2 | [DrawZoomPage.svelte:85](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/DrawZoomPage.svelte#L85) | `<Row label="굵기" sub="Ctrl+휠">` | **(기지적)** 맥에서 `Ctrl+휠` 고정 표기 (실제는 `⌘+휠`) | 맥 `⌘+휠`, Windows `Ctrl+휠` 분기 |
| 3 | [DrawZoomPage.svelte:170](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/DrawZoomPage.svelte#L170) | `<Row label="나갈 때 보관" sub="Alt+Shift+1로 다시 열기">` | **(기지적)** 맥에서 `Alt+Shift+1` 고정 (기본값은 `⌥⇧1`, 사용자 변경 키도 미반영) | `restore` 슬롯의 현재 키를 `formatComboReadable`로 표시 |
| 4 | [AnnotateOptions.svelte:99,117](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/components/AnnotateOptions.svelte#L99) | `title={\`... · Ctrl+휠\`}` | **(기지적)** 맥에서도 툴팁에 `Ctrl+휠` 고정 (실제 핸들러는 `e.ctrlKey \|\| e.metaKey`) | `platform` prop 전달 후 맥 `⌘+휠` / Windows `Ctrl+휠` |
| 5 | [CapturePage.svelte:212](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/CapturePage.svelte#L212) | `<Row label="모서리 자석" sub="Alt를 누르는 동안 끔">` | **(기지적)** 맥에서 `Alt` 고정 표기 (`⌥`여야 함) | 맥 `⌥를 누르는 동안 끔`, Windows `Alt를 누르는 동안 끔` |
| 6 | [CapturePage.svelte:94,235-236](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/CapturePage.svelte#L94) | `trimmed.endsWith('.exe')` 강제 추가, `placeholder="app.exe"`, `aria-label="제외할 실행 파일 이름"` | **(기지적)** 맥에서 앱은 `.app`/번들명/프로세스명인데 `.exe`를 강제로 붙임 | `platform === 'macos'`면 `.exe` 강제 해제, placeholder `예: Safari` 등 분기 |
| 7 | [settings.rs:198-199](file:///Users/bodyking/Desktop/Architecture/screen-ink/crates/ink-domain/src/settings.rs#L198-L199) | `!lower.ends_with(".exe")` 검증 오류 `"‘{}’는 실행 파일 이름이 아니에요(예: hwp.exe)"` | **신규**: 백엔드 검증이 맥에서도 `.exe` 없으면 저장 거부 | 맥에서는 비어있지 않고 경로 구분자만 없으면 허용(또는 `.app`/실행명 허용) |
| 8 | [CapturePage.svelte:22](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/CapturePage.svelte#L22) | `{ id: 'built-in', label: '내장 마이크(Realtek)' }` | **신규**: Realtek는 Windows PC 사운드칩 브랜드명으로 맥에서 이질적 | OS 실제 장치 목록 연동 또는 `'내장 마이크'`로 중립화 |
| 9 | [GeneralPage.svelte:120](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/GeneralPage.svelte#L120) | `<Row label="로그인하면 실행" sub="트레이에만 조용히">` | **신규**: 맥은 트레이가 아니라 **메뉴 막대**임 | 맥 `메뉴 막대에만 조용히`, Windows `트레이에만 조용히` |
| 10 | [FirstRunSheet.svelte:100](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/FirstRunSheet.svelte#L100) | `트레이에 있다가 단축키를 누르면 나타나요` | **신규**: 맥 첫 실행 시트에 `트레이` 표기 노출 | 맥 `메뉴 막대에 있다가…`, Windows `트레이에 있다가…` |
| 11 | [FirstRunSheet.svelte:168](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/FirstRunSheet.svelte#L168) | `F1·F2·F3을 쓰는 동안 다른 앱의 도움말·이름 바꾸기·찾기가 막혀요` | **신규**: Windows 탐색기/앱(F1 도움말·F2 이름 바꾸기·F3 찾기) 기준 설명이 맥에 그대로 나옴 | 맥에서는 macOS 기능 키(`fn` 또는 표준 기능 키) 맥락으로 분리 |
| 12 | [FirstRunSheet.svelte:175](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/FirstRunSheet.svelte#L175) | `F 키는 fn과 함께 눌러요 · fn 없이 쓰려면 키보드 설정에서 ‘표준 기능 키’를 켜요` | **신규**: macOS Ventura+ 정식 메뉴 경로(`시스템 설정 › 키보드 › 키보드 단축키 › 기능 키`)가 불명확 | 정확한 경로(`시스템 설정 › 키보드 › 키보드 단축키 › 기능 키`) 표기 + 가능 시 바로 열기 버튼 |
| 13 | [FirstRunSheet.svelte:130-142](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/FirstRunSheet.svelte#L130-L142) | `needsScreen`만 확인 | **신규**: 맥 필수 권한 중 `손쉬운 사용(accessibility)` 카드/안내 누락 | 화면 기록 + 손쉬운 사용 두 권한 모두 단계별로 확인·요청 |
| 14 | [PinView.svelte:46](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/PinView.svelte#L46) | `if (e.ctrlKey)` (휠 투명도 조절) | **신규**: 키보드(`onKey`)는 `e.ctrlKey \|\| e.metaKey`인데 휠 투명도는 `ctrlKey`만 받아 맥 `⌘+휠` 미작동 | `e.ctrlKey \|\| e.metaKey` (맥 트랙패드 핀치 주의) |
| 15 | [Row.svelte:130](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/components/Row.svelte#L130) | `{combo.replaceAll('+', ' ')}` | **신규**: 단축키 오류 추천 칩이 맥 기호(`⌥⌘⇧⌃`) 변환 없이 원시 문자열(`Ctrl Alt 1` 또는 `Option Shift 1`) 노출 | `formatComboReadable(combo, platform)` 또는 `<Keycap {combo} {platform} />` 사용 |
| 16 | [settingsMeta.js:121](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/settingsMeta.js#L121) · [package.rs:135,145,155](file:///Users/bodyking/Desktop/Architecture/screen-ink/crates/ink-domain/src/package.rs#L135) | `MSIX 패키지` / `포터블` / `설치` | **신규**: Windows 배포 채널 용어가 공통 모델에 존재(맥은 `.app` / DMG / Sparkle) | 맥에서는 채널에 따라 `앱 번들` 등 맥 관습 용어 분기 |
| 17 | [settingsMeta.js:138](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/settingsMeta.js#L138) | `"에서 '항상 위'를 끄면 Alt+Shift+D를 써요"` | **신규**: PowerToys(`windowsOnly`) 전용이지만 `Alt+Shift+D` 문자열이 하드코딩 | `formatComboReadable('Alt+Shift+D', platform)` 일관 적용 |
| 18 | [TrayPopoverScreen.svelte:192](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/TrayPopoverScreen.svelte#L192) | `label="저장 폴더 열기"` | **신규**: PRD `R2-OSFIT-4` 기준 맥=`Finder에서 보기` / Windows=`탐색기에서 열기` 미반영 | OS 관습에 맞춰 라벨 분기(설정의 `열기`/`폴더 열기` 포함) |
| 19 | [shortcuts.rs:585,587](file:///Users/bodyking/Desktop/Architecture/screen-ink/crates/ink-domain/src/shortcuts.rs#L585) | `"수정 키(Ctrl·Alt 등) 없이..."`, `"Shift + 글자는..."` | **신규**: 맥에서 단축키 등록 오류 시 `Ctrl·Alt` / `Shift` 텍스트로 나옴(맥은 `⌘·⌥` / `⇧`) | `Platform::Mac`일 때 `⌘·⌥·⌃` / `⇧ + 글자`로 분기 |
| 20 | [service.rs:320,564](file:///Users/bodyking/Desktop/Architecture/screen-ink/crates/ink-app/src/service.rs#L320) | `"트레이 메뉴로 시작할 수 있어요"` | **신규**: 맥에서도 단축키 비움 시 섬/설명에 `트레이 메뉴` 표기 | `platform`별 `메뉴 막대 아이콘` / `트레이 메뉴` 분기 |
| 21 | [capture.rs:539](file:///Users/bodyking/Desktop/Architecture/screen-ink/crates/ink-app/src/capture.rs#L539) · [draw.rs:916](file:///Users/bodyking/Desktop/Architecture/screen-ink/crates/ink-app/src/draw.rs#L916) | `"한글·PPT에 {paste_text}"` (`⌘V` / `Ctrl+V`) | **신규**: 키는 `⌘V`로 맞게 바꾸나 맥 사용자에게는 `Keynote·PPT` 등도 일반적이며 영어 지원 시 문구 재구성 필요 | `paste_text()`는 유지하되 카탈로그(ko/en, mac/win)로 이동 |
| 22 | [defaults-v1.json:105,220,235](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/shortcuts/defaults-v1.json#L105) · [cheatSheet.js:39-49](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/cheatSheet.js#L39-L49) | 맥 `Shift+Cmd+Z` / `Shift+Cmd+S` vs Windows `Ctrl+Y` / `Ctrl+Shift+S` | **신규**: Windows는 2키 조합이라 `Ctrl Z Y`로 3키캡 묶음이 되지만, 맥은 3키라 S16에서 `⌘ Z ⇧ ⌘ Z`(5키캡)로 풀려 구분 불가 | `buildGroupedRow`에서 2개 조합 간 구분자 처리(OS 공통 개선) |
| 23 | [template.js:12](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/models/template.js#L12) | `validateFilenameTemplate`에서 `\ / : * ? " < > \|` 금지 | **신규(참고)**: macOS에서도 파일명 안전성을 위해 `:`와 `/`를 막는 것은 좋으나 안내/도움말에 OS 파일명 규칙 차이 설명이 없음 | 양 OS 호환 금지 문자로 유지하되 `naming.rs`와 동일 기준 안내 |
| 24 | [DrawOverlay.svelte:260,283,289](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/lib/screens/DrawOverlay.svelte#L260) | 주석/상수는 Windows 표기(`Ctrl+C`, `Alt+Shift+1`)지만 실제 UI 호출엔 영향 없음(단, `SettingsWindow`에서 `platform` 분기 있음) | **참고**: `DrawOverlay` 내부 코드는 `engine.platform`을 넘기나, `DrawPreview`에는 반영되지 않음 | 아래 미리보기 항목 #34에서 해결 |

### 6.2 미리보기 · 가짜 클라이언트 (`packages/ink-client/src/fake.js` · `packages/ink-ui/src/preview/*`) (10건)

| # | 파일:줄 | 현재 표기 / 동작 | 문제 | 수정 안 |
|---|---|---|---|---|
| 25 | [fake.js:7](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L7) | `SUGGEST = ['Ctrl+Alt+1', ...]` | 맥 미리보기(`?platform=macos`)에서 겹침 발생 시 추천 키가 전부 Windows `Ctrl+Alt+...`로 나옴 | `platform === 'macos'`면 `Option+Cmd+1` 등 맥 조합 배열 사용 |
| 26 | [fake.js:11](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L11) | `'capture.excluded': [{ exe: 'licecap.exe', ... }]` | 맥 미리보기에서도 `licecap.exe` 표시 | 맥이면 `LICEcap.app` 또는 `licecap` |
| 27 | [fake.js:12,580](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L12) | `'output.folder': 'C:\\Users\\…\\Pictures\\Shookshok'`, `'D:\\수업자료\\캡처'` | 맥 미리보기에서도 `C:\Users\...` 역슬래시 Windows 경로 노출 | 맥이면 `~/Pictures/Shookshok`, `~/Desktop/수업자료/캡처` |
| 28 | [fake.js:113](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L113) | `location.path: 'C:\\Users\\…\\AppData\\Roaming\\Shookshok\\settings.json'` | 맥 미리보기에서 Windows `AppData` 경로 노출 | 맥이면 `~/Library/Application Support/com.ssamuniverse.shookshok/settings.json` |
| 29 | [fake.js:123,268](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L123) | `IMPORT_FILE` (`Ctrl+Alt+F2`), `previewImport` (`C:\\Users\\…\\Desktop\\...`) | 맥 미리보기에서 가져오기 시트 열면 Windows 경로·단축키 노출 | `platform === 'macos'` 분기 |
| 30 | [fake.js:183](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L183) | `hints: [..., { key: 'Alt 1', label: '멈추고 쓰기' }]` | 맥 실시간 확대 미리보기 섬에서 `Alt 1` 고정 | `platform === 'macos'`면 `⌥1` |
| 31 | [fake.js:294](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L294) | `detail: 'Alt+\`'` (단축키 끄기 섬) | 맥 미리보기에서도 `Alt+\`` 고정 | `combos.pause`를 `spoken` 변환해 사용 |
| 32 | [fake.js:366,472](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-client/src/fake.js#L366) | `detail: '한글·PPT에 Ctrl+V'`, `buttonKey: 'Ctrl+S'` | 맥 미리보기 복사 섬에서 `Ctrl+V`·`Ctrl+S` 고정 | 맥이면 `⌘V`, `⌘S` |
| 33 | [Preview.svelte:409-495](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/preview/Preview.svelte#L409-L495) | 섬 전시 그리드의 `Ctrl+V`, `Ctrl+S`, `Alt+1`, `Alt F1`, `Alt 2`, `Alt 3`, `Alt Shift D`, `Alt \``, `UAC · 잠금` | `platform` props를 전달하지 않고 Windows 키/용어(`UAC`)만 전시 | `initialPlatform`에 따라 맥 기호·용어로 분기 |
| 34 | [DrawPreview.svelte:158,310-311](file:///Users/bodyking/Desktop/Architecture/screen-ink/packages/ink-ui/src/preview/DrawPreview.svelte#L158) | `'Alt+Shift+1 (데모: Ctrl+Shift+R로 복원)'`, `Ctrl 사각 · Ctrl+Shift 화살표 · +Alt 점선 · Ctrl+휠 굵기 · Ctrl+Z/Y` | `?draw=1&platform=macos`로 띄워도 도움말/안내가 Windows 키 고정 | `platform === 'macos'`일 때 `⌥⇧1`, `⇧⌘R`, `⌘ 사각`, `⌘⇧ 화살표`, `+⌥ 점선`, `⌘+휠`, `⌘Z / ⇧⌘Z`로 분기 |

---

## 7. 촬영한 스크린숏 목록 (`ux-shots/`, 총 39장)

모든 파일은 `deviceScaleFactor: 2` PNG이며 **400 KB(409,600 B) 이하** 기준을 충족한다.

| 파일명 | 크기 | 화면 / 상태 / OS |
|---|---|---|
| [tray-both-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/tray-both-mac.png) | 129.9 KB | S01 트레이 팝오버 · 라이트/다크 · macOS |
| [tray-both-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/tray-both-win.png) | 146.8 KB | S01 트레이 팝오버 · 라이트/다크 · Windows |
| [settings-shortcuts-global-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-shortcuts-global-mac.png) | 95.9 KB | S02 설정·단축키 · 어디서나 · macOS |
| [settings-shortcuts-global-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-shortcuts-global-win.png) | 103.2 KB | S02 설정·단축키 · 어디서나 · Windows |
| [settings-shortcuts-inapp-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-shortcuts-inapp-mac.png) | 88.9 KB | S02 설정·단축키 · 화면 안 · macOS |
| [settings-shortcuts-inapp-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-shortcuts-inapp-win.png) | 90.5 KB | S02 설정·단축키 · 화면 안 · Windows |
| [settings-shortcuts-recording-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-shortcuts-recording-mac.png) | 103.6 KB | S02 설정·단축키 · 녹화 중 겹침 오류 + 추천 칩 · macOS |
| [settings-capture-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-capture-mac.png) | 120.0 KB | S03 설정·캡처·녹화 · 상단 · macOS |
| [settings-capture-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-capture-win.png) | 120.0 KB | S03 설정·캡처·녹화 · 상단 · Windows |
| [settings-capture-bottom-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-capture-bottom-mac.png) | 120.5 KB | S03 설정·캡처·녹화 · 하단 + 제외 앱 입력 · macOS |
| [settings-draw-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-draw-mac.png) | 118.8 KB | S04 설정·판서·확대 · 상단 · macOS |
| [settings-draw-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-draw-win.png) | 119.1 KB | S04 설정·판서·확대 · 상단 · Windows |
| [settings-draw-bottom-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-draw-bottom-mac.png) | 114.9 KB | S04 설정·판서·확대 · 하단(칠판 미리보기·확대) · macOS |
| [settings-general-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-general-mac.png) | 84.6 KB | S05 설정·일반 · 라이트 · macOS |
| [settings-general-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-general-win.png) | 104.6 KB | S05 설정·일반 · 라이트 · Windows |
| [settings-general-dark-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-general-dark-mac.png) | 89.9 KB | S05 설정·일반 · 다크 · macOS |
| [settings-general-import-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-general-import-win.png) | 32.0 KB | S05 설정·일반 · 가져오기 확인 시트 · Windows |
| [settings-info-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/settings-info-mac.png) | 66.2 KB | 설정·정보 탭 · macOS |
| [firstrun-conflict-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/firstrun-conflict-mac.png) | 90.9 KB | S06 첫 실행 · 겹침 있음 · macOS |
| [firstrun-conflict-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/firstrun-conflict-win.png) | 81.1 KB | S06 첫 실행 · 겹침 있음 · Windows |
| [firstrun-clean-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/firstrun-clean-mac.png) | 88.8 KB | S06 첫 실행 · 겹침 없음(F키 안내) · macOS |
| [firstrun-clean-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/firstrun-clean-win.png) | 80.0 KB | S06 첫 실행 · 겹침 없음(F키 안내) · Windows |
| [firstrun-perm-screen-off-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/firstrun-perm-screen-off-mac.png) | 103.1 KB | S06 첫 실행 · 화면 기록 권한 꺼짐(`perm=screen-off`) · macOS |
| [firstrun-perm-revoked-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/firstrun-perm-revoked-mac.png) | 103.4 KB | S06 첫 실행 · 권한 풀림(`perm=screen-revoked`) · macOS |
| [overlay-confirmed-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/overlay-confirmed-mac.png) | 309.4 KB | S07·S08 캡처 오버레이 · 드래그 확정 + 툴바 · macOS |
| [overlay-confirmed-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/overlay-confirmed-win.png) | 309.5 KB | S07·S08 캡처 오버레이 · 드래그 확정 + 툴바 · Windows |
| [overlay-annotate-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/overlay-annotate-mac.png) | 307.2 KB | S07·S08 캡처 오버레이 · 펜 선택(색·굵기 알약) · macOS |
| [overlay-mosaic-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/overlay-mosaic-mac.png) | 305.0 KB | S07·S08 캡처 오버레이 · 모자이크 선택(칸 크기 알약) · macOS |
| [cheatsheet-draw-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/cheatsheet-draw-mac.png) | 84.9 KB | S16 단축키 한 장 · 판서 · macOS |
| [cheatsheet-draw-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/cheatsheet-draw-win.png) | 91.1 KB | S16 단축키 한 장 · 판서 · Windows |
| [cheatsheet-capture-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/cheatsheet-capture-mac.png) | 86.4 KB | S16 단축키 한 장 · 캡처 · macOS |
| [cheatsheet-capture-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/cheatsheet-capture-win.png) | 91.6 KB | S16 단축키 한 장 · 캡처 · Windows |
| [cheatsheet-zoom-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/cheatsheet-zoom-mac.png) | 59.8 KB | S16 단축키 한 장 · 실시간 확대 · macOS |
| [cheatsheet-zoom-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/cheatsheet-zoom-win.png) | 64.4 KB | S16 단축키 한 장 · 실시간 확대 · Windows |
| [island-grid-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/island-grid-mac.png) | 316.2 KB | S17 섬 전시 16종 + 두 줄 안내 · macOS |
| [island-grid-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/island-grid-win.png) | 316.2 KB | S17 섬 전시 16종 + 두 줄 안내 · Windows |
| [draw-canvas-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/draw-canvas-mac.png) | 161.1 KB | 판서 데모(`?draw=1`) · 6색 펼침 + `?` 도움말 · macOS |
| [draw-canvas-win.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/draw-canvas-win.png) | 161.1 KB | 판서 데모(`?draw=1`) · 6색 펼침 + `?` 도움말 · Windows |
| [draw-board-mac.png](file:///Users/bodyking/Desktop/Architecture/screen-ink/docs/plans/2026-11-27/research/ux-shots/draw-board-mac.png) | 50.4 KB | 칠판 데모(`?draw=1&board=1`) · 8색 펼침 · macOS |
