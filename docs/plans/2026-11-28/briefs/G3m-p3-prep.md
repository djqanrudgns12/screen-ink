# G3m — 3단계 맥 준비(새 장면 판정 도구·W28~W30 브리프) — Antigravity 네온

---

[일감 G3m-p3-prep — 새 장면(과적합 확인) 판정 도구 + Windows 일감 브리프 W28·W29·W30 (PRD-3차 §5.3, 진척 3-HOLDOUT·3f-docs·3f-win-cases·7.3·7.2·3f-snipaste·R2-CAP)]
사용자에게 보이는 보고와 모든 출력은 한국어로 쓴다(커밋 메시지·주석·문서 포함, 코드 식별자·명령·경로만 원문). 맡은 범위는 묻지 않고 끝내고, 자기 브랜치에 커밋·푸시까지 한다. 착오 없이: 지어낸 수치·경로를 쓰지 않는다 — 저장소에 있는 것만 근거로.

작업 폴더: 저장소 /Users/bodyking/Desktop/Architecture/screen-ink 에서
  git fetch origin && git worktree add ../screen-ink-wt/g3m -b r3/g3m origin/feat/foundation
그 worktree 안에서만 일한다. 끝나면 `git push -u origin r3/g3m`. feat/foundation에 직접 푸시하지 않는다. Windows 노드에 일감을 보내지 않는다(지금 G3a가 Windows를 쓰는 중). 앱을 띄우거나 키·마우스를 보내지 않는다. docs/progress/progress.json은 고치지 않는다.

먼저 읽기: docs/plans/2026-11-28/PRD-3차-마무리.md §2·§5.3, docs/benchmark/2026-11-27-windows-recog.md, docs/benchmark/2026-11-28-recog-replay.md, bench/recog/README.md·RUNBOOK-windows.md·compare-snipaste.ps1·pages/·tools/, crates/ink-platform-fake/src/replay.rs·bin/recog-replay.rs·truth.rs, docs/qa/2026-11-28/win-screen-check.md(#12 가린 그림), docs/prd/08-smart-capture.md §5, 시나리오 카드 S1~S15가 정의된 문서(docs/prd 안에서 찾아 경로를 적는다).

할 일:
1. **새 장면(hold-out) 판정 도구**: 287점 기준선(bench/recog/cases/baseline.json)과 섞이지 않게, 새로 뜨는 사례를 `bench/recog/holdout/`(새 폴더) 같은 따로 된 자리에 두고 `recog-replay -- --holdout`(또는 폴더 인자)로 Top-1·Top-3·경계만 재게 한다. 새 장면은 규칙 고칠 때 보지 않는 시험용(문서에 규칙 명시). CI(.github/workflows/ci.yml)의 기존 재생 검사는 그대로 두고, 새 장면은 기준선 없이 보고만(나중에 7.3에서 고정). 순수 Rust + 시험. 검사: resource-guard로 `cargo test -p ink-platform-fake` · clippy -D warnings.
2. **W29 브리프**(docs/plans/2026-11-28/briefs/W29-win-holdout.md, 기존 형식): Windows에서 새 장면 ≥ 60점을 뜬다 — 287점에 안 쓴 화면·배치(다른 창 크기·위치, 다른 사이트 구조의 시험 HTML, RB-04 Win32 다른 배치, RB-12 200% 다른 자리, RB-14 캔버스 변형, #12 둥근 그림을 앞 창이 15% 가린 장면 포함). 정답은 기존 도구(recog-truth·시험 앱 정답 파일)로 자동. 개인정보 0(시험 문서만). 결과를 bench/recog/holdout/에 커밋. 그 PC의 사례 뜨기 절차는 RUNBOOK-windows.md를 따른다. 같은 김에 3f-docs(RB-14·RB-04 시험 문서 사례)·3f-win-cases(17종 중 RB-13 관리자 권한만 사람)를 닫는 데 필요한 사례도 함께. 혼자 실행.
3. **W28 브리프**(…/W28-win-scenarios.md): 시나리오 카드 S1~S15 중 이 PC(4K 200% + FHD 100%, Snipaste 있음)로 되는 것을 실행해 카드마다 통과·실패·사람 필요(잠금·절전·학교 PC·관리자 권한 등) 표. 카드 정의 문서 경로를 적는다. 혼자 실행.
4. **W30 브리프**(…/W30-win-snipaste.md): bench/recog/compare-snipaste.ps1로 같은 커서 점 집합(287점 + 새 장면)에서 슉쇽 vs Snipaste 첫 강조 적중을 잰다 — Snipaste가 틀린 점 중 슉쇽이 맞힌 비율, 결함 D1~D22 재발 여부. 결과 docs/benchmark/2026-11-28-snipaste.md. 혼자 실행. 시스템 설정 변경·설치 금지(Snipaste는 이미 설치됨).
5. 각 브리프는 send-win.sh가 앞에 붙이는 Windows 머리말(windows-remote-setup.md §5)과 겹치지 않게 일감 본문만 쓰고, 끝 보고 형식·커밋 범위(문서·사례만, 코드 고침은 Windows 전용 결함만)를 적는다.
커밋: 한국어 메시지, 끝에 빈 줄 뒤 `Co-Authored-By: Gemini (antigravity) <noreply@google.com>`. force·--no-verify 금지.
끝 보고: 새 장면 도구(명령·시험), 세 브리프 경로와 요점, 시나리오 카드 정의 문서 경로, 검사 결과(안 돌린 것은 "미실행"), 브랜치·커밋 해시.
