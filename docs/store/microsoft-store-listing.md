# Microsoft Store 등록 자료 — 초안 (진척도 `D4`)

2026-11-26 초안. Partner Center의 칸마다 그대로 붙여 넣을 수 있게 칸 이름 순서로 적었다. 한도는 Microsoft 문서 기준이다: 설명 10,000자, 짧은 설명 1,000자, 기능 20개 × 200자, 검색어 7개(전체 21단어 이내), 저작권 200자, 스크린숏 1~10장(4장 이상 권장).

## 0. 제출 전에 꼭 맞출 것

1. **실제로 들어 있는 기능만 적는다.** 스토어 정책은 설명이 앱과 정확히 맞기를 요구한다.
   - 아래 기능 줄의 표시를 보고, 제출하는 빌드에 없는 줄은 지운다.
     - `[지금]`: 단계 3까지 구현된 것
     - `[3 마무리]`: 단계 3 안이지만 아직 안 된 것(결과 섬 HEX 칩 `3e-chip`, 혼합 배율 확인 `3b-dpi`)
     - `[단계 4~6]`: 아직 없는 것
   - "한글·PPT에 Ctrl+V" 문구는 붙여 넣기 확인(`3e-paste`)이 끝난 뒤에 쓴다.
   - 판서·확대·칠판·녹화가 빠진 첫 제출이면 짧은 설명과 설명 첫 문단에서도 그 말을 뺀다(§2.0의 "첫 제출판" 문구를 쓴다).
2. **스크린숏에 개인정보가 없어야 한다.** 학생 이름·성적·메신저·메일·바탕 화면 파일 이름이 보이면 안 된다. 우리가 만든 시험 문서만 띄우고 찍는다(§4).
3. **게시자 표시 이름**은 스토어 앱 페이지에서 앱 이름 아래 "만든 이"로 공개되는 **계정 이름**이다(앱 이름과 다름). **개발자 계정을 등록할 때 정하며, 등록 뒤에는 바꿀 수 없다고 Microsoft FAQ에 적혀 있다**(바꿀 수 있다는 문서도 있어 엇갈림). 바뀌더라도 이미 낸 앱은 다른 앱으로 취급된다. 그러니 등록 화면에서 처음부터 정한다. **결정(2026-11-26, 사용자): `찰떡쌤`**. 개인 계정은 "자기 이름으로 앱을 내는 계정"이라는 안내가 있다. 실명이 아닌 이름(`쌤유니버스` 등)을 받아 주는지는 공식 규정을 찾지 못했다 **[확인 필요 — 등록 화면에서]**.
4. **패키지 버전**은 네 번째 자리가 0이어야 한다(예: `0.1.0.0`). 스토어 MSIX 규칙이다.

## 1. 기본 정보

| 칸 | 값 |
|---|---|
| 게시자 표시 이름 | **`찰떡쌤`**(계정 등록 때 넣음) |
| 앱 이름(예약) | 한국어 **`슉쇽`** · 영어 **`shookshok`**(2026-11-26 결정, `docs/brand/README.md`). 예약은 Partner Center 계정이 승인되면 |
| 가격 | 무료 |
| 시장 | 모든 시장(대한민국 포함) |
| 범주 | 생산성(Productivity). 보조 범주 교육(Education). 하위 범주는 제출 화면 목록에서 고른다 |
| 개인정보 처리방침 | `https://ink.ssamuniverse.com/privacy/` |
| 웹사이트 | `https://ink.ssamuniverse.com/` |
| 지원 연락처 | `https://ink.ssamuniverse.com/support/` |
| 저작권·상표 | `© 2026 찰떡쌤. MIT License.` |
| 추가 사용 조건 | 비움(MIT 라이선스는 저장소의 LICENSE) |
| 시스템 요구 사항 | 최소: Windows 10 버전 2004(빌드 19041) 이상, x64, 키보드·마우스 / 권장: Windows 11, 모니터 2대까지 시험함 |
| 제품 선언 | "접근성 지침 시험 통과"는 **체크하지 않음**(아직 시험 안 함) · "대체 드라이브에 설치 허용"은 체크 |
| 로고 | 1:1 상자 그림(필수, 1080×1080 또는 2160×2160), 2:3 포스터(권장, 720×1080 또는 1440×2160) — 앱 아이콘 원본(0.4c)으로 만든다 |

## 2. 한국어

### 2.0 짧은 설명

**완성판**(단계 6까지 들어간 뒤):

> 단축키 한 번으로 화면을 캡처하고, 확대하고, 그 위에 바로 판서하는 수업용 도구입니다. 인터넷 연결 없이 동작하고, 어떤 정보도 모으지 않습니다.

**첫 제출판**(캡처만 있을 때):

> F1 한 번이면 커서 아래 표·그림·단추를 알아서 잡아 주는 화면 캡처 도구입니다. 한글·PPT에 바로 붙여 넣고, 인터넷 연결 없이 동작하며 어떤 정보도 모으지 않습니다.

### 2.1 설명

```text
슉쇽은 교실에서 화면을 보여 주며 수업하는 선생님을 위해 만든 도구입니다. 단축키 하나로 화면을 캡처하고, 필요한 곳을 크게 보여 주고, 그 위에 바로 판서합니다.

■ 알아서 잡아 주는 캡처 (F1)
F1을 누르고 커서를 올리면 그 아래의 표·그림·단추·문단을 알아서 강조합니다. 마우스 휠로 더 크게, 더 작게 고르고, 클릭하면 끝. 결과는 클립보드에 바로 들어가서 한글·PowerPoint·채팅 창에 Ctrl+V로 붙여 넣으면 됩니다. 직접 드래그할 때도 창과 칸의 경계에 자석처럼 붙습니다.

■ 수업에 맞춘 작은 배려
· F1 안에서 R을 누르면 방금 잡은 범위, 쉼표·마침표로 최근 범위를 앞뒤로
· 교탁 모니터와 TV·전자칠판처럼 모니터가 둘이어도 정확하게
· 마우스 커서도 함께 찍을지 고르기
· [3 마무리] 모니터 배율이 서로 달라도 정확하게 · 한 가지 색만 잡히면 색 코드(HEX)를 바로 복사
· [단계 6] 방금 잡은 범위를 단축키 하나로 다시 캡처(Shift+F1), 모니터 전체 바로 캡처(Ctrl+F1)

■ 판서·확대·칠판 (Alt+1 · Alt+2 · Alt+3)
슬라이드 쇼 중에도 Alt+1로 화면 위에 바로 쓰고, 휠로 문제 부분을 크게 보여 줍니다. Alt+2는 커서를 따라가는 실시간 확대, Alt+3은 빈 칠판입니다.

■ 학교 컴퓨터에서도 편하게
· 인터넷 연결이 필요 없습니다. 학교망에서도 그대로 동작합니다.
· 개인정보를 모으지 않습니다. 캡처는 클립보드와 선생님이 고른 폴더에만 남습니다.
· 관리자 권한 없이 설치됩니다.
· 단축키 59개를 모두 바꿀 수 있고, 다른 프로그램(Snipaste·PowerToys 등)과 키가 겹치면 알려 줍니다.

■ 무료·오픈소스
MIT 라이선스로 공개된 무료 프로그램입니다. 소스 코드: github.com/djqanrudgns12/screen-ink
```

첫 제출판에서는 "■ 판서·확대·칠판" 문단을 통째로 지우고, 첫 문단을 "단축키 하나로 화면을 캡처해 바로 붙여 넣습니다."로 줄인다.

### 2.2 기능 (최대 20개, 줄마다 200자 이내)

| 표시 | 기능 |
|---|---|
| `[지금]` | F1 한 번으로 커서 아래 표·그림·단추·문단을 알아서 강조 |
| `[지금]` | 마우스 휠로 더 크게·더 작게 범위 고르기, 고른 깊이를 기억 |
| `[지금]` | 드래그할 때 창·칸 경계에 붙는 자석과 1px 방향키 조정 |
| `[지금]` | 결과를 클립보드에 바로 — 한글·PowerPoint에 Ctrl+V |
| `[지금]` | 저장 폴더·파일 이름 규칙 지정 |
| `[지금]` | F1 안에서 R로 방금 잡은 범위, 쉼표·마침표로 최근 범위 넘기기 |
| `[지금]` | 모니터 2대에서도 정확한 범위 |
| `[3 마무리]` | 배율이 서로 다른 모니터에서도 정확한 범위 |
| `[3 마무리]` | 단색 범위는 색 코드(HEX) 바로 복사 |
| `[단계 6]` | Shift+F1 직전 범위 바로 캡처, Ctrl+F1 모니터 바로 캡처 |
| `[지금]` | 마우스 커서 함께 찍기 선택 |
| `[지금]` | 단축키 59개 모두 바꾸기, 다른 앱과 겹치면 안내 |
| `[지금]` | 인터넷 연결 없이 동작, 개인정보 수집 없음 |
| `[단계 4]` | Alt+1 화면 위 판서 — 휠로 확대하며 쓰기 |
| `[단계 4]` | Alt+3 빈 칠판 |
| `[단계 5]` | Alt+2 커서를 따라가는 실시간 확대 |
| `[단계 6]` | F2 창 캡처(가려진 부분까지), F3 화면 녹화(MP4) |
| `[단계 6]` | 지연 캡처(Alt+F1) 3·5·10초 |

### 2.3 이번 버전의 새 기능(첫 제출)

> 첫 버전입니다. F1 스마트 캡처, 휠로 범위 고르기, 클립보드 복사·저장, 최근 범위 다시 고르기(R)를 담았습니다.

### 2.4 검색어 (7개)

`화면 캡처` · `스크린샷` · `캡처 프로그램` · `판서` · `화면 확대` · `수업 도구` · `전자칠판`

첫 제출판에서는 `판서`·`화면 확대`·`전자칠판` 대신 `한글 붙여넣기`·`범위 캡처`·`교사`를 쓴다.

## 3. English

### 3.0 Short description

Full version:

> Capture, zoom and draw on your screen with a single shortcut — built for teaching. Works offline and collects no data.

First submission (capture only):

> Press F1 and shookshok highlights the table, image or button under your cursor. Paste straight into documents and slides. Works offline and collects no data.

### 3.1 Description

```text
shookshok is a screen tool for teachers who present from their computer. One shortcut captures the screen, zooms where you need it, and lets you draw right on top.

■ Capture that finds what you mean (F1)
Press F1 and hover: shookshok highlights the table, image, button or paragraph under the cursor. Scroll to pick a bigger or smaller area, click, done. The result goes straight to the clipboard, ready to paste with Ctrl+V. Dragging snaps to window and cell edges.

■ Small things that matter in class
· Inside F1, press R for the last area, comma/period to step through recent areas
· Accurate on two monitors
· Choose whether to include the mouse cursor
· [3 finish] Accurate with different scaling per monitor · Copy a color code (HEX) when the area is a single color
· [stage 6] Recapture the last area with one key (Shift+F1), capture the whole monitor (Ctrl+F1)

■ Draw, zoom and whiteboard (Alt+1 · Alt+2 · Alt+3)
Draw on the screen even during a slide show and zoom in with the mouse wheel. Alt+2 is a live magnifier that follows the cursor; Alt+3 opens a blank board.

■ Made for school computers
· No internet connection needed.
· Collects no personal data. Captures stay on the clipboard or in the folder you choose.
· Installs without administrator rights.
· All 59 shortcuts can be changed, with a warning when another app already uses a key.

■ Free and open source
MIT licensed. Source code: github.com/djqanrudgns12/screen-ink
```

For the first submission, delete the "Draw, zoom and whiteboard" paragraph.

### 3.2 Features

`[now]` F1 highlights the table, image, button or paragraph under the cursor · `[now]` Scroll to pick a bigger or smaller area · `[now]` Edge snapping and 1px arrow-key nudging while dragging · `[now]` Result goes straight to the clipboard · `[now]` Save folder and file-name rules · `[now]` R for the last area, comma/period for recent areas · `[now]` Accurate on two monitors · `[3 finish]` Accurate with different scaling · `[3 finish]` One-click HEX color copy · `[stage 6]` Shift+F1 / Ctrl+F1 instant capture · `[now]` Optional mouse cursor in captures · `[now]` 59 customizable shortcuts with conflict warnings · `[now]` Works offline, collects no data · `[stage 4]` Alt+1 draw on screen with wheel zoom · `[stage 4]` Alt+3 blank board · `[stage 5]` Alt+2 live magnifier · `[stage 6]` F2 window capture, F3 screen recording (MP4) · `[stage 6]` Delayed capture

### 3.3 What's new

> First release: F1 smart capture, scroll to choose the area, copy and save, reuse recent areas (R).

### 3.4 Search terms

`screen capture` · `screenshot` · `snipping tool` · `annotate` · `zoom` · `teacher` · `whiteboard`

## 4. 스크린숏 계획 (1366×768 이상 PNG, 4~8장)

모두 **실제 앱 화면**이다. 화면 시안(mockups)은 쓰지 않는다. 우리가 만든 시험 문서(교재 PDF 흉내, 표가 있는 시험 HTML, 시험 슬라이드)만 띄우고, 작업 표시줄·바탕 화면에 개인 파일 이름이 보이지 않게 한다.

| # | 장면 | 있는 단계 |
|---|---|---|
| 1 | F1 — 시험 문서의 표가 주황 테두리로 강조되고, 섬에 "표 › 행" 경로가 보임 | 지금 |
| 2 | 휠로 한 칸 키운 모습(같은 자리, 더 큰 범위) | 지금 |
| 3 | 결과 섬 "복사했어요"와 썸네일, 옆에 한글·PPT에 붙여 넣은 결과 | 지금 |
| 4 | 설정 › 단축키 — 59칸과 겹침 안내 | 지금 |
| 5 | 듀얼 모니터에 걸친 범위 | 지금 |
| 6 | Alt+1 판서 — 슬라이드 위에 빨간 펜 풀이 | 단계 4 |
| 7 | Alt+2 실시간 확대 | 단계 5 |
| 8 | F3 녹화 섬 | 단계 6 |

## 5. 연령 등급(IARC 설문) 답안

| 질문 갈래 | 답 |
|---|---|
| 폭력·공포·성적 내용·욕설·약물·도박 | 없음 |
| 사용자끼리 대화·콘텐츠 공유 | 없음 |
| 사용자 위치 공유 | 없음 |
| 디지털 상품 구매 | 없음 |
| 제한 없는 인터넷 접근(브라우저 기능) | 없음 |
| 개인정보 수집 | 없음 |

예상 등급: **전체 이용가**(GRAC 전체 · PEGI 3 · ESRB Everyone). 실제 등급은 설문을 낸 뒤 IARC가 준다.

## 6. 인증 담당자에게 남기는 말 (Notes for certification, 영어)

```text
shookshok is a full-trust Win32 desktop app (Tauri 2 + WebView2) packaged as MSIX. It needs runFullTrust because it:
- registers global hotkeys (RegisterHotKey) — F1/F2/F3 and Alt+1/2/3 by default;
- captures all monitors and shows topmost overlay windows over other apps;
- reads window bounds and UI Automation element bounds to find the area under the cursor (bounds and control types only; no text or titles are read or stored).
The app has no network code, no account and no telemetry.

How to test:
1. Launch shookshok. It lives in the notification area (tray); the settings window opens on first run.
2. Press F1, move the mouse over any window — the element under the cursor is highlighted. Scroll to change the area, click (or press Enter) to copy. Paste into Paint to check.
3. Press Esc to cancel a capture.
4. If F1 is already taken by another app, the app shows a notice and the key can be changed in Settings > Shortcuts.
```

## 7. 확인 필요

- 범주의 하위 범주 목록(제출 화면에서 고름)
- 검색어 한도: 문서마다 한 단어 30자 또는 40자로 다르게 적혀 있다. 제출 화면에서 확인한다
- `runFullTrust` 승인 요청 칸이 실제로 뜨는지(뜨면 §6 문구를 그대로 쓴다)
- 게시자 표시 이름(§0-3) — 사용자 결정

출처: [스토어 목록 정보 추가·편집](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/add-and-edit-store-listing-info), [MSI/EXE 제출](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/create-app-submission), [범주와 하위 범주](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/categories-and-subcategories), [앱 기능 선언(runFullTrust)](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations), [runFullTrust 승인 요청 Q&A](https://learn.microsoft.com/en-us/answers/questions/1424817/the-following-restricted-capabilities-require-appr)
