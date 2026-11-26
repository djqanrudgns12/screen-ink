# 브랜드 — 공식 아이콘 · 앱 이름 · 개발자 표기

2026-11-26 결정(사용자)

| 항목 | 값 | 상태 |
|---|---|---|
| 공식 앱 아이콘 | 최종 시안 **22번 검은 조리개** | **반영 끝**(이 문서 1장) |
| 영문 앱 이름 | **`shookshok`** | **반영 끝**(3장) |
| 한글 앱 이름 | **`슉쇽`** | **반영 끝**(3장) |
| 앱 고유 ID | **`com.ssamuniverse.shookshok`** (가진 도메인 `ssamuniverse.com`을 거꾸로) | **반영 끝**(3장). 배포 뒤에는 바꾸지 않는다 |
| 개발자 | **`찰떡쌤`** | **반영 끝**(2장) |
| 라이선스 | **MIT 유지** | **반영 끝**(4장) |

## 1. 공식 앱 아이콘

![공식 아이콘](official/shookshok-icon-256.png)

- **모양:** 먹색 무광 6날 카메라 조리개 원반. 날개 사이 빈 공간이 흰 만년필 펜촉 실루엣(음각)이고, 펜촉 숨구멍에만 주황 점 하나. 캡처(조리개)와 판서(펜촉)를 한 기호로 묶었다.
- **형태:** 원형. **원 바깥은 투명**(둥근 사각 타일 없음). 1024 캔버스에 지름 880px, 원 아래 옅은 그림자는 캔버스 안에서 끝난다.
- **색:** 먹색·흑연 명암 + 흰 펜촉 + 주황(`#FF6A1A` 근처) 한 점. 이 밖의 색을 더하지 않는다.
- **원본:** [`official/shookshok-icon-1024.png`](official/shookshok-icon-1024.png) (1024×1024 RGBA). 모든 규격은 이 파일 하나에서 만든다. 고치지 말고, 바꿀 때는 새 원본으로 통째로 갈아 끼운다.
- **만든 과정:** 제미나이(antigravity `gemini-pro-orchestrator-neon`) 이미지 생성. 1차 01~10 → 2차 11~20(08·09 계열) → 최종 21~23(15번을 흑백·원형으로) 중 22번. 시안과 프롬프트는 [`icon-drafts/`](icon-drafts/)(최종은 [`icon-drafts/final/README.md`](icon-drafts/final/README.md)).

### 1.1 규격별 파일

다시 만들 때: `apps/screen-ink`에서 `npx tauri icon src-tauri/icons/app-icon.png -o <임시 폴더>` → 아래 파일만 `src-tauri/icons/`로 복사(android·ios 폴더는 쓰지 않음).

| 쓰는 곳 | 파일 | 크기 |
|---|---|---|
| 원본(입력) | `apps/screen-ink/src-tauri/icons/app-icon.png` | 1024 (공식 원본과 같은 파일) |
| Windows exe·설치본·작업 표시줄 | `icons/icon.ico` | 16 · 24 · 32 · 48 · 64 · 256 |
| macOS 앱 | `icons/icon.icns` | 16 ~ 1024(@2x 포함) |
| Tauri 번들 PNG | `icons/32x32.png` · `64x64.png` · `128x128.png` · `128x128@2x.png` · `icon.png` | 32 · 64 · 128 · 256 · 512 |
| Microsoft Store·MSIX 타일 | `icons/Square30x30Logo.png` … `Square310x310Logo.png`(9종) · `StoreLogo.png` | 30 ~ 310 · 50 |
| 앱 안 아이콘(`AppIcon.svelte`) | `packages/ink-ui/src/lib/icons/app-icon-32.png` · `-64` · `-128` | 보이는 크기의 2배를 고른다(16→32, 24~32→64, 56~64→128) |
| 트레이 원본 | `crates/tauri-plugin-screen-ink/assets/tray-app-32.png` | 32 (`icons/32x32.png`와 같은 파일) |
| 문서·홍보용 | `docs/brand/official/shookshok-icon-256.png` | 256 |

`tauri.conf.json`의 `bundle.icon` 목록(32 · 128 · 128@2x · icns · ico)은 그대로다. 파일만 바뀌었다.

### 1.2 트레이 아이콘

![트레이 세 상태](official/tray-states.png)

왼쪽 = 밝은 작업 표시줄, 오른쪽 = 어두운 작업 표시줄. 위는 32px를 4배로 키운 것, 아래는 실제 크기 32 · 24 · 16px.

| 상태 | 그림 | 어두운 작업 표시줄 |
|---|---|---|
| 준비 | 공식 아이콘 32px 그대로 | 원 둘레에 밝은 고리(55%) — 먹색 원이 바탕에 묻히지 않게 |
| 꺼짐(단축키 잠시 끔) | 회색으로 바랜 아이콘 45% + 대각선 한 줄 | 고리 30% |
| 녹화 중 | 큰 빨간 원 + 바깥 고리(아이콘처럼 원형) | 고리가 밝은 색 |

- 코드: `crates/tauri-plugin-screen-ink/src/tray_art.rs`. 내장 PNG를 풀어 상태별로 덧그린다. 32px 한 벌을 만들고 Windows가 16·20·24로 줄인다.
- 모양만으로도 세 상태가 갈린다(색을 못 보는 사람도). 시험: `cargo test -p tauri-plugin-screen-ink tray_art`.
- macOS 트레이(단색 템플릿 16pt)는 아직 없다. 맥판 때 조리개 외곽선만 남긴 단색형을 따로 만든다.

### 1.3 쓰는 규칙

- 원 그대로 쓴다. 둥근 사각 타일 위에 다시 얹지 않는다(Windows·앱 안·문서). **예외: macOS**. macOS 26(Tahoe)은 둥근 사각이 아닌 아이콘을 회색 타일에 넣어 보여 준다고 알려져 있다 **[맥판 때 확인]**. 맥판에서는 같은 조리개를 둥근 사각에 넣은 맥 전용 원본을 따로 만들어 `icon.icns`만 그것으로 뽑는다.
- 늘이기·회전·색 바꾸기·외곽선 더하기 금지. 어두운 바탕에서 묻힐 때만 원 둘레 밝은 고리를 쓴다(트레이 규칙과 같음).
- 최소 크기 16px. 16px에서는 펜촉이 작은 점으로만 남는다. 정상이다.
- 앱 이름과 함께 놓을 때는 아이콘 왼쪽, 이름 오른쪽(트레이 메뉴·설정 창 사이드바와 같음).

### 1.4 확인한 것과 못 한 것

| 항목 | 결과 |
|---|---|
| ico 안 크기 | 16 · 24 · 32 · 48 · 64 · 256 (파일 머리 읽어 확인) |
| icns 안 크기 | 16 ~ 1024 12종 |
| 트레이 3상태 × 밝기 2 | 그림으로 확인(위 그림), 시험 3개 통과 |
| 앱 안 아이콘 | ink-ui 미리보기 트레이 메뉴·설정 창, 라이트·다크에서 불러와짐 확인 |
| 실제 Windows 작업 표시줄·트레이에 띄워 보기 | **안 함** — 다음 실제 앱 실행 때 |
| 16px 트레이 3상태 사람 눈 구분 검사(07 `0.4c`) | **안 함** |

## 2. 개발자 표기 — 참조한 방식

참조: Tidy Task(`Tidy-Task-Local`), 롤린썬더(`rollinthunder`). 두 앱 모두 같은 찰떡쌤 마스코트 그림을 쓴다(`icon-drafts/ref/chaltteok-mascot.webp`, 두 앱 파일이 바이트까지 같음).

| 자리 | Tidy Task | 롤린썬더 | 슉쇽에 쓸 표기 |
|---|---|---|---|
| 저작권 줄 | `© 2026 [마스코트] 찰떡쌤. All rights reserved.` (설정 아래, 업데이트 안내) | `© 2026.[마스코트]찰떡쌤. All rights reserved.` (대기실 아래) | **`© 2026 [마스코트] 찰떡쌤. MIT License.`** (4장) |
| 앱 이름+버전 | `Tidy Task · v{version}` 아래 줄에 저작권 | — | `슉쇽 · v{version}` 아래 줄에 저작권 |
| 첫 화면 | `[마스코트 32px] Made by. 찰떡쌤` (WelcomeWindow) | — | 첫 실행 안내에 `[마스코트] Made by. 찰떡쌤` |
| 개발자 칸 | 스토어 `개발자: 찰떡쌤`, `© 2026 찰떡쌤` | 푸터 `개발자: 찰떡쌤 · 문의: …` | 스토어 `개발자: 찰떡쌤`, `저작권: © 2026 찰떡쌤. MIT License.` |

마스코트 크기: 줄 안에서는 14~20px(롤린썬더 14px, 글쓰기 길잡이 20px 1x/2x webp). 첫 화면은 32px. 큰 원본(98KB) 대신 작은 1x/2x webp를 만들어 쓴다(`writingguidance/apps/web/src/ui/art/chaltteok@1x.webp`, `@2x.webp`가 이미 있음).

## 3. 앱 이름 — 바꾼 자리(2026-11-26)

이름 상수: `crates/tauri-plugin-screen-ink/src/lib.rs`의 `APP_NAME = "슉쇽"`(화면 문구), `APP_DIR = "shookshok"`(폴더·파일 이름). Rust 화면 문구는 이 둘을 쓴다.

| 자리 | 전 | 후 |
|---|---|---|
| `tauri.conf.json` `productName` | `screen-ink` | `shookshok` (설치 폴더·시작 메뉴·exe 이름 `shookshok.exe`) |
| 같은 파일 `identifier` | `kr.screenink.desktop` | `com.ssamuniverse.shookshok` |
| `apps/screen-ink/index.html` 제목 · 로그 파일 이름 · 시작 실패 문구 | `screen-ink` | `슉쇽` · `shookshok` · `슉쇽을 시작하지 못했어요` |
| 트레이 풍선말 · 팝오버 창 제목 · 트레이 메뉴 머리 · `aria-label` | `screen-ink`, `screen-ink 메뉴` | `슉쇽`, `슉쇽 메뉴` |
| 설정 창 제목 · 사이드바 브랜드 · 파일 대화상자 필터 | `screen-ink 설정`, `screen-ink` | `슉쇽 설정`, `슉쇽` |
| 설정 › 정보 | 버전 · 오픈소스 고지 | + `만든 이 찰떡쌤` · `라이선스 MIT` 줄, 맨 아래 `슉쇽 · v버전` + `© 2026 [마스코트] 찰떡쌤. MIT License.` |
| 첫 실행 안내 | `screen-ink가 준비됐어요` | `슉쇽이 준비됐어요` + `[마스코트] Made by. 찰떡쌤` |
| 기본 저장 폴더 | `사진\screen-ink` | `사진\shookshok` |
| 설정 폴더 | `%APPDATA%\screen-ink` | `%APPDATA%\shookshok`. **새 폴더가 없고 옛 폴더가 있으면 시작할 때 한 번 옮긴다** |
| 포터블 표시 파일 | `screen-ink.portable` | `shookshok.portable`. 옛 이름도 포터블로 본다 |
| 파일 이름 기본 틀 · 이름이 비면 쓰는 이름 | `screen-ink_{yyyy}…`, `screen-ink` | `shookshok_{yyyy}…`, `shookshok` |
| 설정 내보내기 파일 · `"app"` 칸 | `screen-ink-settings-날짜.json`, `screen-ink` | `shookshok-settings-날짜.json`, `shookshok` (가져오기는 `"app"`을 보지 않아 옛 파일도 그대로 읽힌다) |
| 오류 문구 | `이 파일은 screen-ink 설정이 아니에요` | `이 파일은 슉쇽 설정이 아니에요` |
| 섬·오버레이·핀 창 제목(안 보임) | `screen-ink island` 등 | `shookshok island` 등 |
| 안내 사이트(`site/`) | `screen-ink`, 주황 점 | `슉쇽`(한국어)·`shookshok`(영어), 앱 아이콘(`site/icon.png`, 파비콘 겸), 바닥 `슉쇽 shookshok · © 2026 찰떡쌤 · MIT License` |
| 스토어 등록 자료 | 앱 이름 미정, `© 2026 screen-ink contributors` | 한국어 `슉쇽`·영어 `shookshok`, `© 2026 찰떡쌤. MIT License.` |
| `LICENSE` · `README.md` 제목 | `djqanrudgns12`, `screen-ink (가칭)` | `찰떡쌤`, `슉쇽 (shookshok)` |

그대로 둔 것(사용자에게 안 보임): 저장소·폴더 이름 `screen-ink`, 크레이트 `tauri-plugin-screen-ink`·`ink-*`, npm `@screen-ink/*`, 플러그인 이름과 명령 `plugin:screen-ink|…`, 이벤트 `screen-ink://…`, 트레이 ID, 스레드 이름. GitHub 주소(`djqanrudgns12/screen-ink`)도 그대로.

ID를 바꾸면서 달라지는 것: Tauri가 ID로 정하는 폴더(로그 `%LOCALAPPDATA%\com.ssamuniverse.shookshok\logs`, WebView2 데이터)가 새로 생긴다. 예전 폴더의 웹뷰 데이터는 쓰지 않는다(설정은 위처럼 옮김).

## 4. 라이선스 — MIT 유지

- MIT를 고른 이유는 SignPath 무료 서명(공개 저장소 + 공인 오픈소스 라이선스가 조건)이다(`docs/prd/11-distribution.md` §1.1·§10). 스토어 등록은 라이선스와 관계없다.
- 그래서 Tidy Task·롤린썬더의 `All rights reserved.` 대신 **`© 2026 [마스코트] 찰떡쌤. MIT License.`** 로 쓴다. 스토어 저작권 칸도 `© 2026 찰떡쌤. MIT License.`
- `LICENSE`의 저작권자도 `찰떡쌤`으로 맞췄다.

## 5. 남은 일

- 실제 앱에서 이름·아이콘·정보 페이지 확인(트레이 풍선말, 설치 폴더 이름).
- 맥판: 둥근 사각 아이콘 원본, 트레이 단색 템플릿.
- Partner Center 앱 이름 예약(계정 승인 뒤): `슉쇽`, 영어 `shookshok`.
