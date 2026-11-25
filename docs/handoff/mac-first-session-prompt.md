# 맥 첫 세션 프롬프트

맥에서 screen-ink 저장소 폴더를 열고, 아래 블록 전체를 첫 메시지로 붙여 넣는다.

```text
screen-ink 맥 작업을 시작한다. 이 저장소는 Windows에서 단계 3까지 만들었고, 맥 어댑터는 아직 비어 있다.

[먼저]
1. git fetch origin && git checkout feat/foundation && git pull --rebase origin feat/foundation
   → 최신 커밋이 18acdb3 이후인지 확인한다.
2. docs/handoff/2026-11-25-mac-windows.md를 끝까지 읽는다. 이 문서가 작업 지시서다.
   참고 문서는 docs/prd/03-architecture.md(포트), 08-smart-capture.md(인식), 07-plan.md(마스터플랜)이다.
3. crates/ink-app/src/ports.rs(포트 계약)와 crates/ink-platform-windows/src/{screen,uia,clipboard,events}.rs(본보기)를 읽는다.

[이번 세션 범위]
인수인계 문서 §2.0 → §2.1 M1부터 순서대로 진행한다.
- §2.0: 맥 빌드를 확인한다. 공통 검사 4개를 돌리고, 이어서 cargo check -p tauri-plugin-screen-ink -p screen-ink, 그다음 npm run app:dev.
- §2.1: 새 크레이트 crates/ink-platform-macos를 만들고 M1(모니터·좌표) → M2(캡처) → M3(커서) → M4(창 목록) → M5(덮개) → M6(클립보드) → M7(오버레이) → M8(캡처 제외)까지 간다.
  M9(AX)·M10(권한)·M11(사건·시계)은 시간이 남으면 이어서 한다.
- 항목 하나가 끝날 때마다: 그 항목의 완료 기준 확인 → 공통 검사 → 커밋.

[규칙]
- ink-platform-windows와 플러그인의 #[cfg(windows)] 블록은 동작을 바꾸지 않는다.
  예외는 맥에서 컴파일이 안 될 때 #[cfg(windows)]를 붙이는 것뿐이고, 그러면 커밋 메시지에 적는다.
- 공통 계약은 바꾸지 않는다. 계약 = 포트, 좌표(가상 화면 물리 px, 왼쪽 위 원점), ink_app::case::Case, InkClient, nativeFrame.
  꼭 바꿔야 하면 멈추고 이유를 먼저 보고한다.
- 좌표 변환은 한 함수에만 두고 단위 시험을 붙인다.
  모니터마다 배율이 다를 때 배치를 어떻게 정했는지, RawWindow.class에 무엇을 넣었는지는 인수인계 문서 §4 결정 기록에 적는다.
- 접근성 역할 → Control 변환은 ink-domain에 Control::from_ax(role)로 더하고 시험을 붙인다.
- 커밋 전 공통 검사(전부 통과해야 커밋한다):
  cargo test -p ink-domain -p ink-app -p ink-vision -p ink-store -p ink-platform-fake
  cargo clippy -p ink-domain -p ink-app -p ink-vision -p ink-store -p ink-platform-fake --all-targets -- -D warnings
  npm test
  npm run check
  맥 크레이트도 clippy -D warnings로 깨끗하게 둔다.
- 푸시 전에 git pull --rebase origin feat/foundation을 한다. 푸시는 feat/foundation에.
- 화면 기록·손쉬운 사용 권한 허용, 시스템 설정 변경, 키보드 설정 변경은 나에게 부탁한다. 직접 바꾸지 않는다.
- 개인정보: 실제 화면으로 사례(bench/recog/cases)를 만들어 커밋하지 않는다.
  사례는 우리가 만든 시험 문서만 띄운 화면에서만 뜬다. 로그에 창 제목·파일 이름·사용자 이름을 쓰지 않는다.
- 인수인계 문서의 [확인 필요] 항목은 확인할 때마다 문서를 고친다(확인됨/다름 + 실제 값).
- 실제 앱 확인은 SCREEN_INK_LOG=debug로 띄워 로그의 highlight·overlay ready 줄로 한다.
  F1 직후에는 커서를 4 px 넘게 움직여야 요소 강조로 바뀐다. 맥 F 키는 fn과 함께 눌러야 할 수 있다.

[끝 보고(한국어)]
- 항목별(M1…)로 완료 / 부분 / 못 함
- 실행한 검사와 결과, 실행하지 않은 것을 나눠서
- 맥 진입 시간(overlay ready ms)을 쟀다면 그 값
- 새로 정한 결정, 남은 [확인 필요], Windows에서 확인해야 할 것(인수인계 §3.2 회귀 확인 대상)
```
