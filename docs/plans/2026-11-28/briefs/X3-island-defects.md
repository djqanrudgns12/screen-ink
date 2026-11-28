# X3 — 섬 자리·F2 앱 이름(공통 Rust) — Codex

---

[일감 X3-island-defects — 범위와 겹친 섬은 아래 가운데로 · F2 섬에 앱 이름 (PRD-3차 §5.2, 화면 확인 #26·#17)]
사용자에게 보이는 보고와 모든 출력은 한국어로 쓴다(커밋 메시지·주석·문서 포함, 코드 식별자·명령·경로만 원문). 맡은 범위는 묻지 않고 끝내고, 자기 브랜치에 커밋·푸시까지 한다.

작업 폴더(꼭 따로): 저장소 /Users/bodyking/Desktop/Architecture/screen-ink 에서
  git fetch origin && git worktree add ../screen-ink-wt/x3-island -b r3/x3-island origin/feat/foundation
그 worktree 안에서만 고친다. 끝나면 `git push -u origin r3/x3-island`. feat/foundation에 직접 푸시하지 않는다. 앱을 띄우거나 키·마우스를 보내지 않는다.

먼저 읽기: docs/qa/2026-11-28/win-screen-check.md "공통 결함" #26·#17, docs/prd/04-screens.md S07(섬 자리)·S10.

고칠 것:
1. #26 상단 섬이 고른 범위와 겹치면 **그 모니터 아래 가운데**로 옮긴다(겹침이 풀리면 다시 위). 지금은 주 화면 (1280,65)~(2100,385) 범위를 확정해도 섬 창이 위(1360,24, 1120×176 물리 px)에 남는다. 섬 창 자리를 정하는 곳(ink-app 서비스의 섬 알림 + 플러그인의 섬 창 놓기, 맥 `window::place_top_center`·Windows 대응)에 "피할 사각형"을 넘기고, 자리 계산은 ink-domain 순수 함수(모니터 작업 영역·섬 크기·피할 사각형 → 위/아래 자리)로 + 시험. 캡처 중 범위가 바뀔 때(손보기 포함)도 따라 바뀐다.
2. #17 F2 창 고르기 섬에 창의 **앱 이름**(창 제목 아님)을 보인다. 영어에서 메모장 대신 `Window`가 나왔다 — 앱 이름을 못 찾을 때만 일반 말로. 맥·Windows 둘 다(맥은 실행 파일·번들 이름에서, 개인정보 규칙: 창 제목·파일 이름은 쓰지 않음).
고칠 범위: crates/ink-domain(새 순수 함수·시험), crates/ink-app(섬 알림에 피할 사각형·앱 이름), crates/tauri-plugin-screen-ink(섬 창 놓기 공통·cfg 부분), crates/ink-platform-macos/src/window.rs(놓기 함수), crates/ink-platform-windows의 섬 놓기 부분, 새 문구가 필요하면 packages/ink-ui/src/lib/i18n/areas/island.*.json만(합침은 `node scripts/i18n-build.mjs`). 고치지 않는 곳: packages/ink-ui의 화면 파일(Antigravity G2·총괄이 고치는 중).
검사(이 맥의 자원 규칙): /Users/bodyking/.local/bin/resource-guard run --profile test --wait-seconds 600 -- bash -c 'export PATH=/opt/homebrew/opt/rustup/bin:$PATH; cargo test -p ink-domain -p ink-app -p ink-platform-fake -p tauri-plugin-screen-ink -- --test-threads=1' · 같은 방식으로 clippy --workspace --all-targets -D warnings · sh scripts/check-windows.sh. 반환값 75는 자원 대기.
커밋: 한국어 메시지, 끝에 빈 줄 뒤 `Co-Authored-By: Codex <noreply@openai.com>`. force·--no-verify 금지.
끝 보고: 바꾼 것·새 시험, 검사 결과(안 돌린 것은 "미실행"), 브랜치·커밋 해시, 총괄이 실기로 볼 것.
