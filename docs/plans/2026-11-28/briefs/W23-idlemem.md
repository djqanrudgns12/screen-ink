# W23 — Windows 대기 메모리 줄이기 (`3-MEM-W`, P1, 끝 측정은 혼자)

- 보내기: W20을 합친 뒤(같은 진입 경로를 재므로) `sh docs/plans/2026-11-28/briefs/send-win.sh W23-idlemem deep`
- 다른 일감과 같이 돌려도 되지만, **마지막 측정은 혼자**(C:\agents\status.ps1)

---

[일감 W23-idlemem — 대기 합산 작업 집합 ≤ 550 MiB, M1 유지 (PRD-3차 §2 D3-1·§5.1, 진척 `3-MEM-W`)]
기준: origin/feat/foundation. 계획 docs/plans/2026-11-28/PRD-3차-마무리.md §2·§3(D3-1)·§5.1을 먼저 읽는다.

지금: docs/benchmark/2026-11-27-windows.md "시작·대기·인식" — 새 프로세스 10분 대기에서 앱 + WebView2 자식 합산 작업 집합 중앙값 785.8 MiB, 전용 656.9 MiB, 핸들 5,459, CPU 0.01%(4K 200% + FHD 100%). 앱 본체만은 약 75 MiB. 모니터별 캡처 오버레이 웹뷰·섬·팝오버를 미리 만들어 두는 구조가 M1 진입 150 ms를 지키는 장치다(없애면 M1이 깨진다).

할 일:
1. 같은 방법(bench/sample-process.ps1, 준비 후 8초 뒤 10분)으로 기준선을 다시 재고, 프로세스별(브라우저·GPU·렌더러별) 표를 만든다.
2. 숨겨 둔 웹뷰의 메모리를 줄인다. 1안: 숨길 때 WebView2 `MemoryUsageTargetLevel = Low`(ICoreWebView2_19), 보일 때 `Normal`. 2안: 숨긴 웹뷰 `TrySuspend`/보일 때 `Resume`(진입 시간을 반드시 재어 본다). 3안: 거의 안 쓰는 화면(설정·도움말 등)만 늦게 만들기. 캡처 오버레이는 미리 만들기를 유지한다. 1안부터, 목표에 닿으면 멈춘다.
3. 고친 뒤 혼자인 상태에서 10분 대기 재측정 + M1(bench/m1-overlay.ps1, 모니터마다 100회) + 첫 진입 검은 화면 0(혼합 배율 첫 진입 표본, W5 방식).

완료 기준: 대기 합산 작업 집합 중앙값 ≤ 550 MiB(늘려 잡음 450), M1 오버레이 준비 p95 ≤ 150 ms(두 모니터), 첫 진입 검은 화면 0, CPU 대기 ≈ 0%. M1이 넘으면 그 안을 되돌리고 다음 안.
고칠 범위: crates/tauri-plugin-screen-ink/src/overlay.rs와 웹뷰를 만드는 곳(cfg(windows) 부분), 필요하면 crates/tauri-plugin-screen-ink/Cargo.toml의 Windows 전용 의존성(webview2-com 등). 다른 일감 몫이라 고치지 않는 곳: freeze.rs·screen.rs·ink-app/capture.rs(W20), uia.rs·msaa.rs·events.rs(W21).
검사: cargo test -p tauri-plugin-screen-ink, cargo clippy -p tauri-plugin-screen-ink --all-targets -- -D warnings, 앱 릴리스 빌드 1회.
문서: docs/benchmark/2026-11-28-windows-idle.md(새) — 조건(판 해시·모니터·n), 전후 표, 프로세스별 표, 고른 안과 버린 안.
커밋: 코드 + 문서. 메시지 한국어.
끝 보고: 전후 대기 메모리·M1 p95·검은 화면, 고른 안, 커밋 해시, 검사 결과(안 돌린 것은 "미실행").
