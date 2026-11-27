# X1 — 인식 적중률 올리기, 재생 기반 (`3-RECOG`, P1 시작 → P3 관문)

- 보내기: 브리지 `bridge_send`(to `codex`, `wake: true`, `tier: deep` = gpt-6.2-astra high), cwd `/Users/bodyking/Desktop/Architecture/screen-ink`. 몇 시간 걸리는 일이라 `bridge_ask`로 기다리지 않는다.
- Windows가 필요 없다: 실제 Windows 화면 287점이 bench/recog/cases에 PNG + 스냅숏으로 있고, 맥에서 순수 재생으로 돈다.

---

[일감 X1-recog — 실제 창 287점 재생 Top-1 ≥ 85%·Top-3 ≥ 93%, 회귀 없음 (PRD-3차 §2·§5.3, 진척 `3-RECOG`)]
사용자에게 보이는 보고와 모든 출력은 한국어로 쓴다(커밋 메시지·주석·문서 포함, 코드 식별자·명령·경로만 원문). 맡은 범위는 묻지 않고 끝내고, 자기 브랜치에 커밋·푸시까지 한다.

작업 폴더(꼭 따로): 저장소 /Users/bodyking/Desktop/Architecture/screen-ink 에서
  git fetch origin && git worktree add ../screen-ink-wt/x1-recog -b r3/x1-recog origin/feat/foundation
그 worktree 안에서만 고친다. 원래 폴더(다른 에이전트·총괄이 쓰는 곳)는 건드리지 않는다. 끝나면 `git push -u origin r3/x1-recog`. feat/foundation에 직접 푸시하지 않는다(총괄이 diff를 보고 합친다).

먼저 읽기: docs/plans/2026-11-28/PRD-3차-마무리.md §2·§5.3, docs/benchmark/2026-11-27-windows-recog.md(종류별 표·적중 정의: 네 변이 정답에서 물리 2 px 안), docs/prd/08-smart-capture.md §4(근거 다섯 개 융합·경계 맞춤·점수).

지금: `cargo run -p ink-platform-fake --bin recog-replay` → 16종·21장면·287점 Top-1 217(75.6%)·Top-3 251(87.5%). 놓친 점이 RB-12(200% 모니터, 50점 중 28)·RB-04(Win32, 18 중 10)·RB-18(가상 데스크톱 가림, 18 중 10)·RB-14(캔버스·WebGL, 33 중 26)·RB-11(메뉴·콤보, 7 중 2)·RB-10(PDF, 4 중 0)·RB-05(탐색기 행, 3 중 0)에 몰려 있다. 점별 CSV는 bench/results/recog-replay.csv.

할 일:
1. 놓친 점을 원인별로 묶는다(배율 변환·경계 맞춤 실패·점수 순서·가림 판정·후보 없음). 종류 이름이나 좌표에 기대는 규칙은 만들지 않는다 — 일반 규칙만(과적합 금지: 3단계에서 287점에 안 쓴 새 장면으로 다시 잰다).
2. 큰 묶음부터 고친다. 고칠 때마다 재생 전체를 돌려 다른 사례가 떨어지지 않는지 본다.
3. 기준선(bench/recog/cases/baseline.json)은 마지막에 한 번 올린다.

완료 기준(전부):
- 실제 287점 Top-1 ≥ 244(85%), Top-3 ≥ 267(93%). 늘려 잡음 90% / 98%.
- 사례마다 지금 기준선보다 Top-1이 1점 넘게 떨어지지 않음.
- 합성 사례 16/16 유지(`cargo test -p ink-platform-fake --test replay`), G3 통과(`cargo run -p ink-platform-fake --bin recog-replay -- --vision-only --filter "RB-01,RB-03" --min-top1 80`).
- 강조 계산 시간이 늘지 않음: 바뀐 순수 함수의 벤치·시험이 있으면 전후를 적는다(4K 커서 질의 ≤ 4 ms 원칙, 1차 R-CAP-18).
고칠 범위: crates/ink-domain/src/recog.rs, crates/ink-vision/src/**, crates/ink-platform-fake/src/replay.rs·truth.rs, bench/recog/cases/baseline.json, 새 문서 docs/benchmark/2026-11-28-recog-replay.md. 고치지 않는 곳: crates/ink-platform-windows·crates/ink-platform-macos(재생으로 못 보는 변경), crates/ink-app/src/capture.rs(Windows W20이 고치는 중 — 꼭 필요하면 보고에 적는다).
검사(이 맥의 자원 규칙 — 무거운 명령은 이렇게): 
  /Users/bodyking/.local/bin/resource-guard run --profile test --wait-seconds 600 -- bash -c 'export PATH=/opt/homebrew/opt/rustup/bin:$PATH; cargo test -p ink-domain -p ink-vision -p ink-platform-fake -- --test-threads=1'
  같은 방식으로 cargo clippy -p ink-domain -p ink-vision -p ink-platform-fake --all-targets -- -D warnings, 재생(recog-replay) 전체.
  반환값 75는 자원 대기(실패 아님) — 잠시 뒤 다시.
문서: docs/benchmark/2026-11-28-recog-replay.md — 종류별 전후 표(점·Top-1·Top-3), 원인 묶음과 고친 규칙, 남은 놓친 점.
커밋: 한국어 메시지, 끝에 빈 줄 뒤 `Co-Authored-By: Codex <noreply@openai.com>`. force·--no-verify 금지.
끝 보고: 전후 Top-1·Top-3(전체·종류별), 고친 규칙, 브랜치·커밋 해시, 검사 결과(안 돌린 것은 "미실행").
