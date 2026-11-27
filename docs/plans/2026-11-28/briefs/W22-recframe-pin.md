# W22 — 녹화 꺾쇠 창 추적(Windows) + 창 고정 다시 켬 안내 (`6.4`·`6.5.3`, P1)

- 보내기: W20·W21을 합친 뒤 `sh docs/plans/2026-11-28/briefs/send-win.sh W22-recframe-pin complex`
- 맥 쪽 같은 기능은 7b9569f(맥 실기는 총괄)

---

[일감 W22-recframe-pin — 창 녹화 꺾쇠가 창을 따라감 + 창 고정이 풀렸을 때 다시 켠 직후 안내 (PRD-3차 §5.1, 진척 `6.4`·`6.5.3`)]
기준: origin/feat/foundation(W20·W21 합친 뒤). 계획 docs/plans/2026-11-28/PRD-3차-마무리.md §5.1을 먼저 읽는다.

A. 녹화 꺾쇠(6.4): 맥은 창 추적 녹화 중 대상 창이 움직이거나 크기가 바뀌면 녹화기의 자리 변화 알림으로 꺾쇠 창을 옮기고, 최소화·다른 Space로 숨으면 꺾쇠를 숨긴다(crates/tauri-plugin-screen-ink/src/rec_frame.rs, crates/ink-platform-macos/src/recorder.rs의 창 추적, 커밋 7b9569f). Windows 창 녹화(crates/ink-platform-windows/src/recorder.rs, WGC CreateForWindow)도 같은 알림을 내게 한다: 이동·크기 변경 → 새 사각형, 최소화·다른 가상 데스크톱(cloaked) → 숨김, 되돌아오면 다시 보임. 시험 창으로 실제 확인(이동 5회·크기 3회·최소화/복원·가상 데스크톱 이동)하고 스크린숏으로 꺾쇠 자리를 본다(시험 창만 보이게, 확인 뒤 지움).
B. 창 고정 안내(6.5.3): 창 고정 중 앱이 비정상 종료되면 다시 켰을 때 "창 고정이 풀렸어요" 안내가 한 번 뜨게 한다(지금 정리는 되지만 안내가 없음 — 진척 note). 정상 종료·모두 풀기 뒤에는 뜨지 않는다. 기존 문구 키가 있으면 쓰고, 없으면 ko·en 두 언어로 더한다(packages/ink-ui/src/lib/i18n).

완료 기준: A 시나리오 전부 꺾쇠가 창 경계와 물리 2 px 안, 숨김·다시 보임 정상, 녹화 파일 정상. B 비정상 종료(작업 관리자 없이 taskkill /F로 시험 사본만) → 재실행 안내 1회, 정상 종료 뒤 0회.
고칠 범위: crates/ink-platform-windows/src/recorder.rs·topmost.rs, crates/ink-app/src/record.rs·topmost.rs(공통 — 맥 동작 유지, 시험 추가), crates/tauri-plugin-screen-ink/src/rec_frame.rs의 cfg(windows) 부분, i18n 문구.
검사: cargo test -p ink-app -p ink-platform-windows -p tauri-plugin-screen-ink, clippy 같은 크레이트 -D warnings, npm test(문구를 더했으면).
커밋: 코드 + docs/qa/2026-11-28/win-recframe-pin.md(새, 시나리오 표). 메시지 한국어.
끝 보고: 시나리오별 결과, 커밋 해시, 검사 결과(안 돌린 것은 "미실행").
