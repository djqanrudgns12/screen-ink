# W24 — Windows 화면 확인 + 범위 손보기 실기 (P2)

- 보내기: `sh docs/plans/2026-11-28/briefs/send-win.sh W24-win-screen-check complex` · 혼자(측정 없음, 다른 일감과 같이 돌려도 됨)
- 맥 몫은 총괄이 같은 점검표로 따로 한다.

---

[일감 W24-win-screen-check — Windows 실제 창 화면 확인(점검표 30줄) + F1 범위 손보기 실기 (PRD-3차 §5.2, 진척 2단계 항목·`3-SEL-ADJUST`)]
기준: origin/feat/foundation(7aaaf00 이후). 먼저 읽기: docs/plans/2026-11-28/PRD-3차-마무리.md §5.2, docs/qa/2026-11-28/screen-checklist.md(점검표 — 이 일의 기준), docs/prd/04-screens.md.

할 일:
1. 릴리스 빌드 사본(C:\agents\runs\W24-win-screen-check\app\)을 포터블 설정으로 띄우고, 점검표 #1~#30의 **Windows 열**을 한국어 → 영어(설정 › 일반 › 언어) 순서로 확인한다. 시험 내용은 점검표의 T1~T4(메모장 빈 문서, 시험 HTML, 단색 HTML — bench/recog/pages 등 개인정보 없는 것)만 띄운다. 판정은 스크린숏(C:\agents\tools\shot.ps1)·화면 픽셀·접근성·앱 기록으로 하고, 줄마다 통과 / 결함(원인 메모) / 미실행(까닭) 중 하나로 적는다. 스크린숏은 판정 뒤 지운다.
2. **F1 범위 손보기(새 기능, 4efbe9e)**: F1로 범위를 확정한 뒤 ① 범위 안을 끌면 통째로 옮겨짐(크기 그대로), ② 오른쪽 변·아래 변·네 귀를 끌면 그쪽만 늘어나고 줄어듦(근처 창 경계에 자석, Alt 누르면 안 붙음), ③ 반대편 너머로 끌면 뒤집힘, ④ 범위 밖을 끌면 새로 그림, ⑤ 커서 모양(옮기기·양쪽 화살표)과 변 가운데 손잡이 4개가 보임, ⑥ 4K 200%와 FHD 100% 두 모니터 모두와 두 모니터에 걸친 범위에서 확인. 각 결과 크기는 복사한 그림 크기로 재서 적는다.
3. 필수 실기 줄(#16 3e-chip, #17 6.1 F2 테두리, #23 5.1 돋보기, #30 창을 맨 위에)은 꼭 한다.
고치기: **Windows 전용 코드**(crates/ink-platform-windows, cfg(windows), Windows 벤치 스크립트)의 결함만 고친다. 공통 화면(packages/ink-ui)·공통 Rust(ink-domain·ink-app·플러그인 공통 부분)의 결함은 고치지 말고 결과 문서에 "공통 결함"으로 적는다(맥 총괄이 같은 파일을 고치는 중이라 겹치지 않게).
문서: docs/qa/2026-11-28/win-screen-check.md(새) — 줄 번호·진척 id·판정(ko/en)·메모 표, 범위 손보기 결과 표, 공통 결함 목록(재현 순서·화면·기대/실제), 고친 Windows 결함과 커밋.
검사: 고친 것이 있으면 cargo test -p ink-platform-windows + clippy -D warnings. 없으면 미실행으로 적는다.
커밋: 문서 + Windows 고침. 메시지 한국어.
끝 보고: 줄별 통과/결함/미실행 개수, 범위 손보기 결과, 공통 결함 목록, 커밋 해시, 검사 결과(안 돌린 것은 "미실행").
