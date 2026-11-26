# 병렬 작업 운영 규칙 (2026-11-26~)

메인 세션이 작업을 쪼개 여러 일꾼에게 동시에 맡기고, 결과를 합친다. 사용자는 끼어들지 않는다 — 사용자 결정이 필요한 것은 모아 두었다가 최종 보고 때 묻는다.

## 1. 누가 무엇을

| 일꾼 | 맡는 일 | 쓰는 곳 |
|---|---|---|
| **메인 세션**(이 세션) | 작업 쪼개기·배분, 병합, 전체 검사, 커밋·푸시, 진척도·인수인계 갱신, 최종 보고 | `feat/foundation` 원본 폴더 |
| **Claude 일꾼**(서브 에이전트) | Rust·Windows 어댑터·플러그인·순수 엔진처럼 정확도가 중요한 흐름 | 흐름마다 자기 워크트리 |
| **Antigravity 네온**(브리지 tier complex) | 화면(Svelte)·디자인·시험 문서·시험 앱처럼 독립된 흐름 — 대기열로 연달아 | 흐름마다 자기 워크트리 |

## 2. 흐름 하나 = 워크트리 하나 = 브랜치 하나

- 워크트리: `D:\dev-cache\screen-ink\wt\<흐름>`(C 드라이브가 좁음), 브랜치 `stream/<흐름>`(시작점 = 배분 순간의 `feat/foundation`).
- 빌드 폴더: `CARGO_TARGET_DIR=D:\dev-cache\screen-ink\target-<흐름>`(서로 잠그지 않게), `CARGO_HTTP_CHECK_REVOKE=false`. JS는 워크트리에서 `npm ci`.
- **파일 주인**: 배분할 때 흐름마다 고쳐도 되는 파일·폴더를 정한다. 남의 파일을 고쳐야 하면 고치지 말고 보고서에 "필요한 변경"으로 적는다(메인이 합칠 때 반영).
- 공통 계약(포트·좌표·`Case`·`InkClient`·`nativeFrame`)을 바꿔야 하면 멈추고 메인에 보고한다.

## 3. 일꾼이 지킬 것

- 커밋은 자기 브랜치에만. 푸시·병합·PR은 하지 않는다(메인이 한다).
- 끝 기준: 그 흐름의 완료 기준 + `cargo test`(바뀐 크레이트)·`cargo clippy -D warnings`·`npm test`·`npm run check` 통과.
- **화면을 쓰는 자동 조작 금지**: 사용자가 같은 PC에서 일한다. 앱 실행·F1 자동화·전체 화면 창·마우스 이동은 하지 않는다. 화면 확인이 필요하면 "메인이 확인할 것"으로 남긴다.
- 개인정보·비밀값을 쓰지 않는다. PowerShell 5.1은 BOM 없는 UTF-8 한글을 깨뜨린다 — 파일은 편집 도구나 `[IO.File]`(UTF-8)로.
- 보고: 바꾼 파일, 실행한 검사와 결과, 실행하지 않은 것, 메인이 확인할 것, 사용자 결정이 필요한 것.

## 4. 메인 세션의 병합 순서

1. 흐름이 끝나면 그 브랜치를 받아 검토(차이·시험) → `feat/foundation`에 병합(충돌은 메인이 푼다).
2. 병합마다 전체 검사(Rust 전체·clippy·JS·progress --check) → 푸시 → CI(ubuntu·Windows·맥) 통과 확인.
3. 진척도(`docs/progress/progress.json`)와 인수인계 문서를 메인만 고친다(일꾼은 고치지 않음 — 충돌 방지).
4. 화면 확인이 필요한 것은 모아서 한 번에, 사용자에게 미리 묻고 짧게.

## 5. 1차 배분 (2026-11-26, 시안 [2026-11-26/](2026-11-26/index.md) 기준)

| 흐름 | 일꾼 | 범위 | 빌드 폴더 |
|---|---|---|---|
| `ink-engine` | Claude | A 4.1 순수 엔진 → 4.2 렌더러(`canvasRender.js`·`DrawCanvas.svelte`) → 여유 있으면 4.3 엔진 쪽 | `target-ink-engine` |
| `draw-mode` | Claude | A 판서 모드 Rust·연결부: `Mode::Draw`, `ink-app/draw.rs`(보관·복원·내보내기), 플러그인 명령 6개, InkClient, `DrawOverlay.svelte` 껍데기 | `target-ink-engine` |
| `win-recog` | Claude | C 3b-dpi·3b-msaa·RB-15/16 시험·시험 문서/시험 앱 소스·화면 확인 절차(실행은 메인) | `target`(공용) |
| `live-zoom` | Claude | D 5.1·5.2·5.5 + 5.4 측정 준비(5.3은 A 뒤) | `target`(공용) |
| `cap-chip` | 네온 | B 3e-chip(HEX 칩·주황 잔상) | `target-ag` |
| `msix` | 네온(대기열 2번) | H D2 MSIX 포장·D3 패키지 모드·릴리스 CI 틀 | `target-ag` |
| (조사) | Claude | 스파이크: WGC·녹화·확대·MSIX·업데이트 API·MSAA·DPI — 끝, [spikes-windows.md](2026-11-26/spikes-windows.md) | `spikes/target` |
| `window-record` | Claude(2차) | E 6.1 F2 WGC·6.2 바로 캡처 키·6.3 F3 녹화(MF 직접, windows-capture 안 씀)·6.5 자동 중지·복구·마이크·여유 있으면 6.4 | `target-ink-engine` |

병합 기록: `cap-chip` 병합(3e-chip). 잔상은 지금 `overlay.rs`가 오버레이 웹뷰에 `eval`로 그린다 — 2차 캡처 주석 흐름(4.9)이 `CaptureOverlay.svelte`를 맡을 때 `screen-ink://afterimage` 이벤트 + `SelectionFrame state="afterimage"`로 옮기고 `eval`을 지운다. `msix`는 브랜드 커밋(8796250) 전 기준으로 출발했다 — 병합 때 Identity를 `com.ssamuniverse.shookshok`, 표시 이름 `슉쇽 (ShookShok)`, `.appinstaller` 파일 이름 `shookshok.appinstaller`로 맞춘다.

- 빌드 폴더를 같이 쓰는 흐름은 cargo 잠금으로 빌드가 차례로 돈다 — 메모리(16GB 중 여유 4GB대)를 넘기지 않으려는 의도.
- 같은 폴더(원본)에서 사용자의 브랜드 세션이 아이콘·`tokens.css`·`04-screens.md`·`10-design-system.md`·`tray_art.rs`를 고친다 — 모든 흐름에서 손대지 않는다.
- 2차(1차 결과를 본 뒤): E 창 캡처·녹화(스파이크 결과 반영), F 창 항상 위, A 4.3~4.8 화면, B 4.9·이어받기, D 5.3, H D8~D10, G 안정화.

## 6. 사용자 결정 (2026-11-26 확정)

| # | 결정 | 확정 |
|---:|---|---|
| 1 | 캡처 주석 모자이크 블록 크기 | 논리 10 px × 배율, Ctrl+휠로 6~24 px |
| 2 | 판서 0.5초 멈춤 도형 보정 | **제안형이 기본**("원으로 바꿀까요? · Enter", 그대로 쓰면 유지) + 설정 `draw.shapeApply`로 "바로 바꾸기"(즉시 변환 + Ctrl+Z로 원래 획) 선택 — 시안 기본안과 다름 |
| 3 | 확대 중 판서 복사·저장 범위 | **늘 모니터 전체**(확대·이동과 무관, 물리 해상도) — 시안 기본안과 다름 |
| 4 | 결과 섬 버튼 3개가 넘칠 때 | 키캡 생략, 한 줄 유지 |
| 5 | 실시간 확대 중 휠 | 휠 = 배율, Shift+휠은 가로채지 않고 아래 앱으로 |
| 6 | F3 녹화에서 창을 고르면 | 창 추적(WGC CreateForWindow) |
| 7 | 일정이 밀릴 때 6.5 창 항상 위(P1) | 뒤로 미룸(필수 기능 뒤, 여유 있으면 1차) |
| 8 | 앱 이름 | 화면 이름 **슉쇽**, 플랫폼(스토어 등) 표기 **슉쇽 (ShookShok)**, 폴더·파일 `shookshok`, identifier `com.ssamuniverse.shookshok`(브랜드 세션 8796250). 크레이트·npm·플러그인·이벤트 이름은 `screen-ink` 그대로 |
| 9 | 맥 첫 실행 시트 F키 안내 | 첫 실행 시트에 짧은 보조 한 줄로 늘 함께 |

(PRD `R-NET-1` 문구는 v3.0.5에서 이미 고쳤다.)
