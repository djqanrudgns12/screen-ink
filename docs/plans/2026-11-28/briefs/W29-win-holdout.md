# W29 — Windows 새 장면(hold-out) 수집 + 판정 (`3-HOLDOUT`·`3f-docs`·`3f-win-cases`·`R2-CAP`, P3, 혼자)

- 보내기: `sh docs/plans/2026-11-28/briefs/send-win.sh W29-win-holdout complex`
- **혼자 실행**(`C:\agents\status.ps1`로 다른 일감이 없는지 확인, W28 끝난 뒤).

---

[일감 W29-win-holdout — 새 장면 ≥ 60점 수집·판정 + #12 가린 그림 사례 + 3f-docs/3f-win-cases 마무리 확인 (PRD-3차 §2·§5.3, 진척 `3-HOLDOUT`·`3f-docs`·`3f-win-cases`·`R2-CAP`·`7.3`)]
기준: `origin/feat/foundation`.
먼저 읽기:
- `docs/plans/2026-11-28/PRD-3차-마무리.md` §2·§5.3
- `docs/benchmark/2026-11-27-windows-recog.md` · `docs/benchmark/2026-11-28-recog-replay.md`
- `bench/recog/README.md` · `bench/recog/RUNBOOK-windows.md` · `bench/recog/holdout/README.md`
- `docs/qa/2026-11-28/win-screen-check.md` (#12 가린 그림 설명)
- `docs/prd/08-smart-capture.md` §5

절대 규칙:
1. **인식 규칙 수정 금지**: `crates/ink-domain/src/recog.rs`, `crates/ink-vision/`의 규칙·점수·문턱값은 바꾸지 않는다(과적합 방지, 측정만 수행).
2. **287점 기준선과 분리**: 새 장면 사례는 반드시 `bench/recog/holdout/`에만 둔다(`bench/recog/cases/`·`baseline.json`에는 넣지 않고, `baseline.json`도 바꾸지 않는다).
3. **개인정보 0**: 시험 HTML과 시험 앱(`rb04-win32`, 빈 메모장 등)만 띄운다. 창 제목·사용자 화면·알림이 PNG에 들어가지 않게 `RUNBOOK-windows.md` 절차를 따른다.

할 일:
1. **새 장면 사례 ≥ 60점 수집(`bench/recog/holdout/`, 사례 ID `ho-*`)** — 기존 287점에 쓰지 않은 화면·배치로 70~100점 안팎이 되게 뜬다:
   - **(A) 다른 사이트 구조의 새 시험 HTML**: `bench/recog/pages/holdout-web.html`을 새로 만든다(`rb01.html`과 다른 2단/3단 카드·내비·폼·표 배치, `<script type="application/json" id="layout">` + 왼쪽 위 2×2 표지 규칙 동일). Chrome에서 **비최대화·287점과 다른 창 크기/위치**로 열어 수집 뒤 `recog-truth`로 자동 정답 생성.
   - **(B) RB-04 Win32 다른 배치**: `rb04-win32.exe`를 287점과 다른 위치·크기로 띄워 `recog-window`로 수집 뒤 `rb04-win32 merge`로 자동 정답 병합.
   - **(C) RB-12 200% 4K 모니터 다른 자리**: 주 4K 200%(`DISPLAY2`)의 다른 위치·창 크기에 `holdout-web.html`(또는 `rb04-win32`)을 두고 수집 + 자동 정답.
   - **(D) RB-14 캔버스 변형**: `bench/recog/pages/holdout-canvas.html`(2D 캔버스·WebGL·그래프·카드 배치 변형, 표지/layout 규칙 동일) 또는 `rb14.html`을 다른 창 크기/위치로 띄워 수집 + `recog-truth` 자동 정답.
   - **(E) #12 가린 둥근 그림(`R2-CAP`, `win-screen-check.md` #12)**: 둥근 모서리 그림 카드가 포함된 시험 HTML(`holdout-web.html` 또는 `holdout-occluded.html`)의 오른쪽 약 15%를 앞 시험 창(예: 빈 메모장 `T1.txt`)으로 가린 배치. **주의**: `recog-window`는 창 하나만 찍으므로 두 창 겹침·Z-순서가 함께 담기도록 `SCREEN_INK_CASE_DIR="$PWD\bench\recog\holdout"`로 앱 사본을 띄워 F1 → Enter로 수집한다(다른 모든 창 최소화, 바탕 화면 흰 창 또는 시험 영역 확인, 개인정보 0 눈검수 필수). 정답은 가리지 않은 쪽 보이는 그림 영역 + 앞 창 요소로 자동(`recog-truth` 후 가림 경계에 맞춰 검증하거나 시험 페이지가 가림 전후 영역을 구분).
2. **새 장면 판정 실행**:
   - `cargo run -p ink-platform-fake --bin recog-replay -- --holdout` (`bench/results/recog-holdout.csv` 생성)
   - `cargo run -p ink-platform-fake --bin recog-replay -- --holdout --min-top1 80` (관문 80% 통과/미달 여부 그대로 기록)
   - `cargo test -p ink-platform-fake --test replay` (기존 287점 기준선 회귀 유지 + `holdout` 분리 시험 통과 확인)
3. **`3f-docs`·`3f-win-cases` 마무리 상태 확인**:
   - `bench/recog/cases/`에 16/17종(RB-04·RB-14 포함, RB-13 관리자 권한 창만 사람 승인 필요)이 모두 있는지 확인하고, 누락이 있으면 `RUNBOOK-windows.md`대로 보완한다. 누락이 없으면 확인 사실을 보고서에 남긴다.
4. **보고서 작성**: `docs/benchmark/2026-11-28-holdout.md` — 수집 조건, 사례별(총 점 수 ≥ 60, Top-1, Top-3, 경계 오차 px) 표, 80% 관문 판정, #12 가린 그림 관찰 분석, `3f-docs`/`3f-win-cases` 확인 결과.

고치기 및 커밋 범위:
- **커밋 범위**: `bench/recog/holdout/*.json` + PNG, 새 시험 HTML(`bench/recog/pages/holdout-*.html`), `bench/results/recog-holdout.csv`, `docs/benchmark/2026-11-28-holdout.md`.
- 코드 고침은 Windows 전용 수집 도구(`crates/ink-platform-windows/examples/recog-window.rs` 등)에 실행을 막는 버그가 있을 때만 최소 수정한다. 인식 엔진·`baseline.json`·공통/맥 코드는 건드리지 않는다.

끝 보고 형식:
- 새 장면 총 점 수와 사례별 Top-1·Top-3·경계 오차 표
- 관문(≥ 60점, Top-1 ≥ 80%) 판정 및 #12 가린 그림 결과
- `3f-docs`·`3f-win-cases` 확인 결과
- 커밋 해시
- 실행한 검사 결과(안 돌린 것은 "미실행")
