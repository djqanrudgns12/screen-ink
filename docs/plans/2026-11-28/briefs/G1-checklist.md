# G1 — 두 OS 화면 점검표 (`3-CHECKLIST`, P1에서 미리 → P2에 씀)

- 보내기: 브리지 `bridge_send`(to `antigravity`, `wake: true`, `tier: complex` = 네온 오케스트레이터), cwd `/Users/bodyking/Desktop/Architecture/screen-ink`.

---

[일감 G1-checklist — 실제 창 화면 확인 점검표 (PRD-3차 §5.2, 진척 `3-CHECKLIST`)]
사용자에게 보이는 보고와 모든 출력은 한국어로 쓴다. 맡은 범위는 묻지 않고 끝내고, 자기 브랜치에 커밋·푸시까지 한다.

작업 폴더(꼭 따로): 저장소 /Users/bodyking/Desktop/Architecture/screen-ink 에서
  git fetch origin && git worktree add ../screen-ink-wt/g1-checklist -b r3/g1-checklist origin/feat/foundation
그 worktree 안에서만 쓴다. 원래 폴더는 건드리지 않는다. 끝나면 `git push -u origin r3/g1-checklist`. feat/foundation에 직접 푸시하지 않는다.
이 일은 문서 하나만 만든다: 코드·설정 파일은 고치지 않고, 앱을 띄우거나 시스템 설정을 바꾸는 명령은 실행하지 않는다.

먼저 읽기: docs/plans/2026-11-28/PRD-3차-마무리.md §5.2, docs/prd/04-screens.md(화면 S01~S17), docs/prd/10-design-system.md, docs/plans/2026-11-27/PRD-2차-고도화.md §4·§6, docs/qa/2026-11-27/design-gate.md(미리보기 168장 관문), docs/progress/README.md의 "2단계 — 화면 확인" 목록.

만들 것: docs/qa/2026-11-28/screen-checklist.md
- 2단계 항목(4.3·4.4·4.6·4.8·4.9·5.2·5.5·7.7·R2-ANN·R2-ONB·R2-I18N·R2-CAP·R2-EXTRA·R2-WORD·R2-ICON·R2-UI-TB·R2-FONT·R2-UI-TRAY·R2-HELP·R2-DRAW-TB·R2-OSFIT·3e-chip·6.1·5.1)마다 한 줄 이상: 화면(S 번호) · 여는 방법(맥 키 / Windows 키, 단축키는 docs/prd 단축키 확정본 기준) · 띄울 시험 내용(텍스트 편집기 빈 문서·시험 HTML처럼 개인정보 없는 것) · 볼 것 3~5개(명세에 적힌 위치·크기·문구·OS 표기·영어 전환 잘림·겹침·넘침) · 판정(통과/결함) 칸 · 진척 id.
- 맨 앞에 공통 규칙: 스크린숏은 시험 내용만 띄운 화면에서, 판정 뒤 지운다. 공개 저장소에 그림을 올리지 않는다. 한국어·영어 두 번(영어는 설정 › 언어).
- 두 OS가 다르게 보여야 하는 곳(⌘/Ctrl, 메뉴 막대/작업 표시줄, 시스템 설정/설정)은 칸을 나눈다.
- 한 항목은 5분 안에 볼 수 있게 짧게. 전체 표는 맥·Windows 각각 2시간 안.
완료 기준: 위 24개 id가 모두 들어 있고, 각 줄이 "여는 방법 · 볼 것 · 판정 칸"을 가진다. 근거가 된 명세 절을 줄마다 적는다.
커밋: 한국어 메시지, 끝에 빈 줄 뒤 `Co-Authored-By: Gemini (antigravity) <noreply@google.com>`. force·--no-verify 금지.
끝 보고: 만든 파일, 항목 수, 명세에서 서로 어긋나 보인 곳(있으면), 브랜치·커밋 해시.
