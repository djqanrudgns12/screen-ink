# W28 — Windows 시나리오 카드 S1~S15 확인 (`7.2`, P3, 혼자)

- 보내기: `sh docs/plans/2026-11-28/briefs/send-win.sh W28-win-scenarios standard`
- **혼자 실행**(`C:\agents\status.ps1`로 다른 일감이 없는지 확인, W27 측정이 끝난 뒤).

---

[일감 W28-win-scenarios — 시나리오 카드 S1~S15 중 이 PC로 되는 것 실행 및 판정표 (PRD-3차 §5.3, 진척 `7.2`)]
기준: `origin/feat/foundation`.
먼저 읽기:
- `docs/benchmark/README.md` §6 ("6. 기능 대조 시나리오 카드" — **S1~S15 정의 문서**)
- `docs/PRD.md` §8 · `docs/prd/07-plan.md` (`7.2`) (참고: `docs/prd/04-screens.md`의 S01~S15는 화면 설계 번호이므로 시나리오 카드 S1~S15와 다르다)
- `docs/plans/2026-11-28/PRD-3차-마무리.md` §5.3
- `docs/qa/2026-11-28/win-screen-check.md` (포터블 실행·Snipaste 공존 방식)

실행 조건:
- 앱: `origin/feat/foundation` 최신 릴리스 빌드 사본을 `C:\agents\runs\W28-win-scenarios\app\`에 복사해 포터블 설정으로 실행한다.
- 모니터: 주 4K 200%(`DISPLAY2`, 0,0) + 오른쪽 FHD 100%(`DISPLAY1`, 3840,0).
- 단축키: 이 PC에는 Snipaste 2.11.3이 F1·F3을 사용 중이다. 먼저 겹침 감지 안내가 뜨는지 확인한 뒤, 시험용 사본 설정에서만 영역 찍기·녹화 키를 빈 키(`Ctrl+Alt+F1`, `Ctrl+Alt+F3`)로 바꿔 진행한다. Snipaste나 Windows 시스템 설정(배율 변경, 네트워크, 권한)은 건드리지 않는다.
- 화면에는 시험 HTML(`bench/recog/pages/rb01.html`, `rb14.html`)·빈 메모장 등 개인정보 없는 시험 창만 띄운다. 임시 스크린숏·결과 PNG/MP4는 판정 뒤 지운다.

할 일:
1. `docs/benchmark/README.md` §6의 **S1~S15**를 아래 분류대로 확인한다:
   - **이 PC에서 실행할 카드**:
     - `S1`: 크롬(`rb01.html`) 버튼 위에 커서 → 캡처 키 → 버튼 테두리 강조·클릭 확정
     - `S2`: S1 상태에서 휠 위·아래로 부모·자식 요소 범위 확장/축소
     - `S3`: 탐색기 창(시험용 빈 폴더) 위 캡처 키 → `Tab` 창↔요소 전환, 그림자 제외 창 경계 확인
     - `S4`: 4K 200% ↔ FHD 100% 두 모니터 경계를 넘나드는 드래그 선택 → 복사 PNG 이음매 없음 확인
     - `S5`: 방향키 1 px 이동·색 돋보기(좌표·색 값 표시)·`C`로 색 값 복사
     - `S6`: PowerPoint가 설치돼 있으면 개인정보 없는 시험 슬라이드 쇼 중 F1(또는 시험 키)·Alt+1·Alt+2가 쇼 위에 뜨고 쇼가 안 꺼지는지 확인(PowerPoint 미설치 시 "미설치로 실행 불가" 사유 기록)
     - `S7`: 다른 시험 창에 가려진 시험 창을 F2로 창 캡처 → 가려진 부분까지 해당 창만 찍히는지 확인
     - `S8`: Alt+1 → 휠 확대 → 판서 → Esc → 다시 Alt+1로 확대·판서 함께 동작 및 보존 정책 확인
     - `S9`: Alt+2 실시간 확대에서 `bench/recog/pages/rb14.html`의 움직이는 영상 틀 위 확대가 끊김 없이 커서를 따라가는지 확인
     - `S10`: 이 PC의 혼합 배율(4K 200% + FHD 100%)에서 S1·S4·S8 동작·좌표 오차 없음 확인(100%+150% 배율 변경은 시스템 설정 변경 금지이므로 "부분 통과 · 150% 배율은 사람 필요"로 구분 기록)
     - `S15`: 포터블 폴더에서 관리자 권한 상승 없이 캡처·판서·확대·고정 등 주요 기능 실행 확인(순수 표준 사용자 계정 검증은 "부분 통과 · 표준 계정 확인 사람 필요"로 구분 기록)
   - **실행하지 않고 "사람 필요"로 분류할 카드**:
     - `S11`: 관리자 권한 창(작업 관리자) 위 F1·Alt+1(UIPI)
     - `S12`: 실제 화면 잠금 → 해제, 절전 → 깨우기, 모니터 뽑기 → 꽂기
     - `S13`: UAC 확인 창(보안 데스크톱) 위 단축키
     - `S14`: 학교망(SSL 검사) 환경에서 하루 사용
2. 각 카드마다 **통과 / 실패 / 부분 / 사람 필요**와 기호(`○ / △ / × / —`), 확인한 화면·수치·관찰 근거를 표로 적는다. 실패가 있으면 재현 순서를 적는다.
3. 문서 작성: `docs/benchmark/2026-11-28-win-scenarios.md` (시나리오 카드 정의 문서 경로 `docs/benchmark/README.md` §6, 판 해시, PC 조건, S1~S15 결과 표, 결함 목록, 사람 필요 항목).

고치기 및 커밋 범위:
- **문서(`docs/benchmark/2026-11-28-win-scenarios.md`)만 커밋**한다.
- 단, 실행 중 명백한 **Windows 전용 결함**이 드러나고 범위가 작으면 `crates/ink-platform-windows`·`cfg(windows)`만 고쳐 함께 커밋한다(공통 UI/도메인·맥 코드는 고치지 말고 결함으로만 기록). 고쳤으면 `cargo test -p ink-platform-windows`와 `cargo clippy -p ink-platform-windows --all-targets -- -D warnings`를 돌린다.

끝 보고 형식:
- 시나리오 카드 정의 문서 경로(`docs/benchmark/README.md` §6)
- 통과 / 실패 / 부분 / 사람 필요 개수와 S1~S15 요약 표
- 발견 결함 및 고친 내용(있다면)
- 커밋 해시
- 실행한 검사 결과(안 돌린 것은 "미실행")
