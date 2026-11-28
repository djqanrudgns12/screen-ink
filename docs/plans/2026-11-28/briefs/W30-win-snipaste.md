# W30 — Windows Snipaste 대조 + D1~D22 재발 점검 (`3f-snipaste`, P3, 혼자)

- 보내기: `sh docs/plans/2026-11-28/briefs/send-win.sh W30-win-snipaste standard`
- **혼자 실행**(W29를 합친 뒤 단독 실행, `C:\agents\status.ps1` 확인).

---

[일감 W30-win-snipaste — 같은 점 집합(287점 + 새 장면)에서 슉쇽 vs Snipaste 대조 및 D1~D22 재발 여부 (PRD-3차 §2·§5.3, 진척 `3f-snipaste`)]
기준: `origin/feat/foundation`(W29 합친 뒤).
먼저 읽기:
- `docs/plans/2026-11-28/PRD-3차-마무리.md` §2·§5.3
- `docs/prd/08-smart-capture.md` §0·§3(D1~D22)·§5.4
- `bench/recog/compare-snipaste.ps1` · `bench/recog/RUNBOOK-windows.md` §6
- `docs/benchmark/2026-11-27-windows-recog.md` · `docs/benchmark/2026-11-28-recog-replay.md` · `docs/benchmark/2026-11-28-holdout.md`

제약(꼭 지킴):
- **시스템 설정 변경·설치 금지**: Snipaste 2.11.3은 이미 이 PC에 설치돼 있고 기본 설정 상태다. 새로 설치하거나 Snipaste/Windows 설정을 바꾸지 않는다.
- 측정 전 `screen-ink` 프로세스는 모두 종료한다(F1 충돌 방지). 도는 동안 마우스·키보드를 만지지 않는다.
- `%TEMP%\screen-ink-snipaste` 전·후 PNG는 `-Keep` 없이 자동 삭제되게 두고, 저장소에 PNG를 넣지 않는다. 화면에는 시험 문서·시험 앱만 띄운다(개인정보 0).

할 일:
1. **최신 기준 재생 CSV 생성**:
   - `cargo run -p ink-platform-fake --bin recog-replay` (`bench/results/recog-replay.csv`)
   - `cargo run -p ink-platform-fake --bin recog-replay -- --holdout` (`bench/results/recog-holdout.csv`)
2. **테두리 보정(`-Calibrate`)**:
   - `rb04-win32.exe` 시험 창을 띄운 채 `bench/recog/compare-snipaste.ps1 -Case <RB-04 사례 JSON> -Calibrate`로 `-Inset` 값을 한 번 잰다.
3. **같은 커서 점 집합 대조(`287점` + `새 장면`)**:
   - `bench/recog/cases/*.json`과 `bench/recog/holdout/*.json`의 각 사례에 대해, 해당 시험 화면/앱을 사례가 뜬 위치·크기 그대로 화면에 복원한 뒤 아래를 실행한다:
     `powershell -ExecutionPolicy Bypass -File bench\recog\compare-snipaste.ps1 -Case <사례.json> -ReplayCsv <해당 CSV> -Inset <보정값>`
   - 화면 상태를 그대로 재현할 수 없는 사례(예: 관리자 권한이 필요한 RB-13 등)는 억지로 돌리지 말고 "미실행(사유)"로 분리 기록한다.
4. **집계 및 판정**:
   - `287점` / `새 장면` / `합계` 각각에 대해 Snipaste Top-1 vs 슉쇽 Top-1·Top-3를 비교한다.
   - **Snipaste가 틀린 점 중 슉쇽 첫 강조(Top-1)가 맞힌 수와 비율**을 계산한다(3차 관문: 같은 점에서 슉쇽 Top-1 ≥ Snipaste Top-1, 늘려 잡음: Snipaste 실패 점의 ≥ 70% 맞춤).
5. **결함 D1~D22 재발 여부 점검**:
   - `docs/prd/08-smart-capture.md` §3의 D1~D22 각 항목을 기존 증거(RB-15·16·18·20, 287점 재생, 새 장면, 이번 Snipaste 대조)와 연결해 재발 없음/관찰/미실행(예: 관리자 권한)을 표로 정리한다(D23은 맥 2차 항목이므로 대상 외로 표기).
6. **보고서 작성**: `docs/benchmark/2026-11-28-snipaste.md` — PC·Snipaste 조건, `-Inset` 보정값, 287점/새 장면/사례별 대조 표, Snipaste 실패 점 구조 비율, D1~D22 재발 여부 표, 미실행 목록.

고치기 및 커밋 범위:
- **커밋 범위**: 문서(`docs/benchmark/2026-11-28-snipaste.md`)와 대조 결과 CSV(`bench/results/recog-2026-11-28-snipaste-*.csv`)만 커밋한다.
- 코드 고침은 `bench/recog/compare-snipaste.ps1`이나 `bench/recog/tools/snipaste-diff` 등 Windows 전용 측정 도구에 실행을 막는 버그가 있을 때만 최소 수정한다. 인식 엔진·공통·맥 코드는 건드리지 않는다.

끝 보고 형식:
- `-Inset` 보정값과 대조한 총 점 수(실행/미실행)
- 287점 / 새 장면 / 전체 Snipaste vs 슉쇽 Top-1·Top-3 및 Snipaste 실패 점 구조 비율 표
- 관문(Top-1 ≥ Snipaste, 늘려 잡음 ≥ 70%) 판정
- D1~D22 재발 여부 요약
- 커밋 해시
- 실행한 검사 결과(안 돌린 것은 "미실행")
