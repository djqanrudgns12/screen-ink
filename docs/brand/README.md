# 브랜드 — 공식 아이콘 · 앱 이름 · 개발자 표기

2026-11-26 결정(사용자)

| 항목 | 값 | 상태 |
|---|---|---|
| 공식 앱 아이콘 | 최종 시안 **22번 검은 조리개** | **반영 끝**(이 문서 1장) |
| 영문 앱 이름 | **`shookshok`** | 반영 전(3장) |
| 한글 앱 이름 | **`슉쇽`** | 반영 전(3장) |
| 개발자 | **`찰떡쌤`** | 반영 전(2·3장) |
| 라이선스 | **MIT 유지** | 저작권 줄만 반영 전(4장) |

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

## 3. 앱 이름 — 바꿀 자리(반영 전)

| 파일 | 지금 | 바꿀 값 |
|---|---|---|
| `apps/screen-ink/src-tauri/tauri.conf.json` `productName` | `screen-ink` | `shookshok` (설치 폴더·시작 메뉴·exe 이름. 한글 이름은 스토어 표시 이름으로) |
| 같은 파일 `identifier` | `kr.screenink.desktop` | 결정 필요 — 예 `kr.chaltteok.shookshok`. 아직 배포 전이라 지금 바꾸는 게 안전 |
| `apps/screen-ink/index.html` `<title>` | `screen-ink` | `슉쇽` |
| `packages/ink-ui/.../TrayPopoverScreen.svelte` 브랜드·`label` | `screen-ink`, `screen-ink 메뉴` | `슉쇽`, `슉쇽 메뉴` |
| `packages/ink-ui/.../SettingsWindow.svelte` 사이드바 브랜드·`aria-label` | `screen-ink`, `screen-ink 설정` | `슉쇽`, `슉쇽 설정` |
| 같은 파일 정보 페이지 | 버전·오픈소스 고지만 | `만든 이: [마스코트] 찰떡쌤` 줄 + 맨 아래 저작권 줄(2장 표기) |
| `packages/ink-ui/.../FirstRunSheet.svelte` | `screen-ink가 준비됐어요`, `AppIcon label="screen-ink"` | `슉쇽이 준비됐어요` + `Made by. 찰떡쌤` |
| `crates/tauri-plugin-screen-ink/src/lib.rs` 트레이 풍선말·창 제목·파일 대화상자 필터 | `screen-ink`, `screen-ink 설정` | `슉쇽`, `슉쇽 설정` |
| 같은 파일·`adapters.rs` 기본 저장 폴더 | `사진\screen-ink` | `사진\shookshok` |
| 같은 파일 설정 폴더 | `AppData\Roaming\screen-ink` | `AppData\Roaming\shookshok` |
| `crates/ink-domain/src/settings.rs` 파일 이름 기본 틀 | `screen-ink_{yyyy}…` | `shookshok_{yyyy}…` |
| 같은 파일 설정 내보내기 `"app"` | `screen-ink` | `shookshok` (가져오기는 옛 `screen-ink`도 받기) |
| 오류 문구 `이 파일은 screen-ink 설정이 아니에요` (Rust·JS 4곳) | | `이 파일은 슉쇽 설정이 아니에요` |
| `crates/ink-store` 포터블 표시 파일 | `screen-ink.portable` | `shookshok.portable` |
| `docs/store/microsoft-store-listing.md` | 앱 이름 미정, 저작권 `screen-ink contributors. MIT` | 한국어 `슉쇽`·영어 `shookshok`, 저작권은 4장 |

그대로 둘 것(사용자에게 안 보임): 저장소·폴더 이름 `screen-ink`, 크레이트 `tauri-plugin-screen-ink`·`ink-*`, npm `@screen-ink/*`, 플러그인 이름 `screen-ink`(권한 식별자가 바뀌어 손이 많이 감), 스레드 이름·로그 파일 이름. 바꾸려면 따로 정한다.

## 4. 라이선스 — MIT 유지

- MIT를 고른 이유는 SignPath 무료 서명(공개 저장소 + 공인 오픈소스 라이선스가 조건)이다(`docs/prd/11-distribution.md` §1.1·§10). 스토어 등록은 라이선스와 관계없다.
- 그래서 Tidy Task·롤린썬더의 `All rights reserved.` 대신 **`© 2026 [마스코트] 찰떡쌤. MIT License.`** 로 쓴다. 스토어 저작권 칸도 `© 2026 찰떡쌤. MIT License.`
- 이름을 바꿀 때 `LICENSE`의 `Copyright (c) 2026 djqanrudgns12`도 `찰떡쌤`으로 맞춘다.

## 5. 정해야 할 것

- `identifier` 새 값(3장).
