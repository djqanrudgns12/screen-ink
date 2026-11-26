# 맥 첫 세션 프롬프트 (2026-11-26 마감 뒤)

맥에서 저장소 폴더를 열고, 아래 블록 전체를 첫 메시지로 붙여 넣는다.

```text
슉쇽(코드 이름 screen-ink) 맥 작업을 이어서 한다. Windows에서 2026-11-26에 판서·칠판, 캡처 주석, 실시간 확대(Windows), F2 창 캡처·F3 녹화(Windows), 인식 마무리, MSIX 포장을 병렬로 만들어 feat/foundation에 병합했다. 맥 코드는 오늘 바뀌지 않았지만 공통 코드가 크게 늘었다.

[먼저]
1. git fetch origin && git checkout feat/foundation && git pull --rebase origin feat/foundation
   → docs/handoff/2026-11-26-mac-handoff.md §8의 기준 커밋 이후인지 확인한다.
2. docs/handoff/2026-11-26-mac-handoff.md를 끝까지 읽는다. 이 문서가 작업 지시서다.
   이전 문서 docs/handoff/2026-11-25-mac-windows.md의 §1 공통 규칙·§5 파일 지도는 그대로 유효하다.
   결정 기록은 docs/plans/work-streams.md §6, 설계 시안은 docs/plans/2026-11-26/(특히 I-mac.md), 진척은 docs/progress/README.md.
3. crates/ink-app/src/ports.rs(포트 계약 — 오늘 Magnifier·Recorder·show_draw 등이 늘었다)와 packages/ink-client/src/index.js(InkClient 계약)를 읽는다.

[이번 세션 범위 — 인수인계 §2 순서]
- §2.0 받기·빌드: 공통 검사 4개 + cargo clippy --workspace --exclude screen-ink --exclude ink-platform-windows --all-targets -- -D warnings, 그다음 npm run app:dev. 이름이 슉쇽인지, 로그 경로(§1-1)가 실제로 어디인지 확인해 문서를 고친다.
- §2.1 공통 기능 맥 확인(M12 판서·칠판, M16 캡처 주석·핀·칩·잔상, 원본 바이트 IPC가 WKWebView에서 오는지). 화면 조작이 필요하면 나에게 먼저 묻고 짧게 한다.
- §2.2 M9 AX 색인 → M10 권한 흐름 → M17 F키 안내 한 줄 → M-sign.
- 시간이 남으면 §2.3(M14 F2 창 캡처 → M13 실시간 확대 SCStream→Metal → M15 녹화 SCStream+AVAssetWriter) 또는 §2.4 플랫폼 무관 할 일.
- 항목 하나가 끝날 때마다: 완료 기준 확인 → 공통 검사 → docs/progress/progress.json 상태 고치고 npm run progress → 커밋.

[규칙]
- ink-platform-windows와 플러그인의 #[cfg(windows)] 블록은 동작을 바꾸지 않는다(맥 컴파일용 #[cfg] 추가만 예외, 커밋 메시지에 적기).
- 공통 계약(포트, 좌표 = 가상 화면 물리 px·왼쪽 위 원점, ink_app::case::Case, InkClient, nativeFrame)은 바꾸지 않는다. 꼭 바꿔야 하면 멈추고 먼저 보고한다. 새 포트 메서드는 기본 구현을 둔다.
- 앱 이름 규칙: 화면 슉쇽, 영문만 쓰는 곳 ShookShok, 파일·폴더 Shookshok, ID com.ssamuniverse.shookshok(플러그인 APP_NAME·APP_DIR 상수 사용).
- 사용자 결정(work-streams.md §6)을 따른다. 새 결정이 필요하면 맥락을 짧게 붙여 묻는다.
- 커밋 전 공통 검사(전부 통과해야 커밋):
  cargo test -p ink-domain -p ink-app -p ink-vision -p ink-store -p ink-platform-fake
  cargo clippy -p ink-domain -p ink-app -p ink-vision -p ink-store -p ink-platform-fake --all-targets -- -D warnings
  npm test
  npm run check
  npm run progress -- --check
  맥 크레이트·플러그인도 clippy -D warnings로 깨끗하게.
- git add는 경로를 밝혀서(-A 금지). 푸시 전에 git pull --rebase origin feat/foundation. 푸시는 feat/foundation에. CI(ubuntu·Windows·맥·UI)가 푸시마다 돈다 — 실패하면 고친다.
- 병렬 일꾼을 쓰면: 흐름마다 워크트리·브랜치·전용 CARGO_TARGET_DIR(빌드 폴더를 같이 쓰면 산출물을 덮어쓴다).
- 화면 기록·손쉬운 사용 권한 허용, 시스템 설정·키보드 설정 변경은 나에게 부탁한다. 직접 바꾸지 않는다.
- 개인정보: 실제 화면으로 사례(bench/recog/cases)를 만들어 커밋하지 않는다. 로그에 창 제목·파일 이름·사용자 이름을 쓰지 않는다.
- 실제 앱 확인은 SCREEN_INK_LOG=debug로 띄워 로그(highlight·overlay ready·mode enter draw)로 한다. F1 직후에는 커서를 4 px 넘게 움직여야 요소 강조로 바뀐다. 맥 F 키는 fn과 함께일 수 있다.

[끝 보고(한국어)]
- 항목별(M12·M16·M9…)로 완료 / 부분 / 못 함
- 실행한 검사와 결과, 실행하지 않은 것을 나눠서
- 맥 진입 시간(overlay ready·mode enter draw ms)을 쟀다면 그 값
- 새로 정한 결정, 남은 [확인 필요], Windows에서 확인해야 할 것
```
