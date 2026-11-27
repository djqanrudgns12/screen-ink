# W20 — 4K 첫 강조 줄이기 (`2.5`, P1)

- 보내기: `sh docs/plans/2026-11-28/briefs/send-win.sh W20-highlight4k deep` · 같이 돌릴 수 있는 일감: W21
- 아래 `---` 뒤가 Windows 에이전트에게 가는 본문이다(머리말은 send-win.sh가 붙인다).

---

[일감 W20-highlight4k — 4K 첫 후보 강조 p95 ≤ 30 ms (PRD-3차 §5.1, 진척 `2.5`)]
기준: origin/feat/foundation. 계획 docs/plans/2026-11-28/PRD-3차-마무리.md §2·§5.1을 먼저 읽는다.

지금: W16 뒤 4K '준비 → 첫 후보 강조' p95 38 ms(목표 30), FHD 25 ms. 측정 방법·정의는 docs/benchmark/2026-11-27-windows.md "M1 캡처 진입과 첫 강조"(`overlay ready`·`highlight first` 로그, bench/m1-overlay.ps1, 커서 쪽 모니터마다 100회).

할 일:
1. 같은 방법으로 4K·FHD 각 100회 기준선을 다시 잰다(혼자인지 C:\agents\status.ps1로 확인 뒤). 첫 강조까지 구간(첫 이동 전달 → 후보 계산 → 네이티브 강조 창 반영)을 나눠 볼 수 있게 필요하면 디버그 로그 줄을 더한다(창 제목·파일 이름은 기록하지 않는다).
2. 가장 긴 구간부터 줄인다. 후보 예: 첫 이동 전에 커서 자리 후보를 미리 계산해 두기, 강조 창 갱신을 다음 그리기 주기까지 기다리지 않기, 영상 분석 결과를 기다리지 않고 창·접근성 후보로 먼저 그리기. 화면 품질(Top-1)을 깎는 단축은 하지 않는다.
3. 고친 뒤 같은 조건으로 다시 100회씩.

완료 기준: 4K p95 ≤ 30 ms(n ≥ 100), FHD p95 ≤ 30 ms 유지, M1 오버레이 준비 p95 ≤ 150 ms 유지, `cargo run -p ink-platform-fake --bin recog-replay` Top-1 217점 이상 유지. 못 닿으면 35 ms 이하에서 구간 표와 원인을 적고 판정을 남긴다.
고칠 범위: crates/ink-platform-windows/src/freeze.rs·screen.rs·gpu.rs, crates/ink-app/src/capture.rs·vision.rs(공통 — 맥 동작을 바꾸지 않게, 바꾸면 시험으로 고정), crates/ink-vision. 다른 일감 몫이라 고치지 않는 곳: uia.rs·msaa.rs·events.rs(W21), crates/tauri-plugin-screen-ink/src/overlay.rs(W23).
검사: cargo test -p ink-app -p ink-vision -p ink-platform-windows, cargo clippy (같은 크레이트) --all-targets -- -D warnings, 인식 재생 1회.
문서: docs/benchmark/2026-11-28-windows-highlight.md(새) — 조건(판 해시·모니터·n), 전후 표, 구간 표.
커밋: 고친 코드 + 문서. 메시지 한국어.
끝 보고: 전후 p95(4K·FHD), 줄인 구간과 방법, 커밋 해시, 검사 결과(안 돌린 것은 "미실행").
