# J. 확인만 남은 항목 마무리 시안 (`0.4`·`0.4b`·`0.4c`·`1.3`·`1.6`·`2.2`·`2.5`·`2.6`·`R-win-regress`)

대상 항목(`progress.json`, 모두 상태 `verify` — 구현·단위 시험 완료, 실측·눈 검사·자동 검사 1회만 남음):
`0.4` 토큰·테마 · `0.4b` 아이콘 24종·부품 14종 · `0.4c` 앱·트레이 아이콘 · `1.3` 트레이 팝오버(S01) · `1.6` 설정 창(S02~S05) · `2.2` 오버레이 투명도 재지정 · `2.5` 진입 계측·M1 · `2.6` SystemEvents(잠금·절전·보안 데스크톱) · `R-win-regress` 맥 변경 뒤 Windows 회귀 확인.

---

## 1. 목표 · 범위 (요구사항 ID)

- **목표**: 이미 구현이 끝난 9개 `verify` 항목의 완료 기준(측정·자동 검사·눈 검사·시나리오 기록)을 빠짐없이 닫아 **관문 G1·G2의 미결 항목을 0으로** 만든다.
- **요구사항 ID**: `R-UI-2` (`0.4`), `R-UI-3·4` (`0.4b`·`0.4c`), `R-SET-0` (`1.3`·`1.6`), `R-SYS-6` (`2.2`), `R-CAP-2`·`R-BENCH-1` (`2.5`), `R-SYS-3` (`2.6`), 회귀(`R-win-regress`).

---

## 2. 정확한 형식 (항목별 확인 도구 · 스크립트 · 기록 위치)

| ID | 남은 확인 내용 (`progress.json` 메모) | 실행할 검사·도구 형식 | 기록 위치 |
|---|---|---|---|
| `0.4` | 하드코딩 색 린트 · F02 대조 확인 | ① `scripts/check-tokens.mjs`: `packages/ink-ui/src/**/*.{svelte,css}`에서 `tokens.css`와 `INK_COLORS` 제외 `#hex`/`rgb(`/`hsl(` 패턴 0건 검사 ② `?screen=tokens` 라이트·다크 캡처 ↔ `docs/mockups/v3/png/F02-tokens.png` 대조 | `bench/results/verify-s0-s2.md` |
| `0.4b` | 대비 4.5:1 자동 검사 | `scripts/check-contrast.mjs`: `packages/ink-ui/src/lib/tokens.css`에서 라이트/다크 `[data-theme]` 변수를 파싱해 WCAG 2.1 상대 휘도 계산 → `(text, text-2, sub, accent-text)` × `(bg, sidebar, surface, raised)` + `(on-invert × invert)` + `(island-text/sub/accent-text × island)` 전 쌍 `≥ 4.5:1`, 아이콘 방울(`accent-icon`) `≥ 3.0:1` 검증 (`npm run check:design`으로 등록) | `bench/results/verify-s0-s2.md` |
| `0.4c` | 트레이 16 px 눈 검사 3명 | 라이트/다크 Windows 작업 표시줄(100% 16px, 125% 20px, 150% 24px)에서 `Ready`·`Recording`·`Off` 3상태 스크린숏 촬영 → 3명에게 라벨 없이 구분 테스트 | `bench/results/verify-s0-s2.md` |
| `1.3` | 트레이 팝오버(S01) 열기 140 ms 측정 | `toggle_popover` 호출 시각(`Instant::now()`) → `TrayPopoverScreen.svelte` 첫 페인트(`rAF`) 시각 계측 로그 `popover open N ms` 1줄 추가(또는 DevTools Performance 트레이스 10회) → 목표 `p50 ≤ 140 ms` | `bench/results/verify-s0-s2.md` |
| `1.6` | 실제 설정 창 S02~S05 = F04~F07 대조 | 실제 앱에서 S02(라이트)·S03(다크)·S04(라이트)·S05(다크) 창 캡처 → `docs/mockups/v3/png/F04~F07`과 나란히 비교(차이 있으면 수정 목록 기록) | `bench/results/verify-s0-s2.md` |
| `2.2` | 대기 1시간·배율 변경 뒤 검은 화면 0 (`R-SYS-6`) | ① 앱 실행 후 1시간 대기(또는 절전 복귀) → F1 진입 ② 디스플레이 배율 100%→125%→100% 변경 → F1 진입 → 검은 화면 0회 확인 | `bench/results/verify-s0-s2.md` |
| `2.5` | `highlight first` 계측 확인 | 이미 구현된 `selection probe: V first … A first …` (`crates/ink-app/src/capture.rs:133`) 로그와 `overlay ready`를 짝지어 Chrome 위 20회 집계 → 첫 강조 ≤ 진입 + 30 ms 확인(별도 `highlight first` 라벨로 출력할지 기존 `selection probe`로 통합할지 확정) | `bench/results/verify-s0-s2.md` |
| `2.6` | `SystemEvents` S12·S13 시나리오 카드 기록 | Windows 실기에서 ① `Win+L` 잠금·해제 중/후 F1 ② 절전·깨우기 후 F1 ③ UAC 프롬프트(보안 데스크톱) 중 F1(조용히 무시 + 1회 안내) ④ 모니터 케이블 뽑기·꽂기 후 F1 | `bench/results/verify-s0-s2.md` |
| `R-win-regress` | 결과 섬 위치 · 혼합 배율(100%+150%) 미확인 | 학교 PC 두 모니터를 100%+150%로 설정 후 ① 둘째 모니터에서 F1 → 결과 섬이 커서 모니터 위 가운데에 뜨는지 ② 100%↔150% 넘어 드래그 끝점이 커서와 ≤ 1 px 일치하는지 확인 | `docs/handoff/2026-11-25-mac-windows.md` 갱신 |

---

## 3. 알고리즘 · 결정 (대안과 고른 이유)

- **자동화 가능한 검사는 스크립트로 고정**: `0.4`(하드코딩 색)와 `0.4b`(대비 4.5:1)는 수동 눈 검사로 끝내지 않고 순수 Node 스크립트(`scripts/check-tokens.mjs`, `scripts/check-contrast.mjs`)로 만들어 `npm test` 또는 `7.7` 디자인 관문에서도 재실행할 수 있게 한다(회귀 방지).
- **`2.5` 계측 통합**: `capture.rs`에 이미 `selection probe: V first … A first … moves …`가 들어 있으므로, 새 로그를 중복 추가하지 않고 기존 `selection probe`에서 `V first`를 `highlight first` 지표로 집계하는 파서를 `bench/` 스크립트에 붙인다.
- **실기 확인 묶음 실행**: 화면을 점유하는 확인(`2.2`·`2.6`·`R-win-regress`)은 사용자 작업을 방해하지 않도록 미리 안내 후 한 번의 짧은 세션(약 15분)에 묶어 처리한다(`handoff §3.0` 원칙).

---

## 4. 시험 · 합격 기준

| ID | 합격 기준 |
|---|---|
| `0.4` | 하드코딩 색 0건, F02 토큰 판과 시각 일치 |
| `0.4b` | 글자 대비 전 쌍 `≥ 4.5:1`, 아이콘 방울 `≥ 3.0:1` 자동 통과 |
| `0.4c` | 16 px 트레이 아이콘 3상태(준비·녹화·꺼짐) 3명 전원 100% 구분 |
| `1.3` | 트레이 팝오버 열기 `p50 ≤ 140 ms` (`n ≥ 10`), 키보드(↑↓·Enter·Esc)만으로 전 항목 동작 |
| `1.6` | S02~S05 실제 창 ↔ F04~F07 차이 0건(또는 의도한 차이 기록) |
| `2.2` | 1시간 대기·배율 변경·절전 복귀 후 F1 첫 진입 검은 화면 0회 |
| `2.5` | M1 `p50 ≤ 80 / p95 ≤ 150 ms` 유지 + `V first` ≤ 진입 + 30 ms |
| `2.6` | 잠금·절전·보안 데스크톱·모니터 변경 4종 모두 크래시/고착 0회, S12·S13 `○` |
| `R-win-regress` | 100%+150% 혼합 배율에서 결과 섬 위치·넘어 끌기 오차 ≤ 1 px |

---

## 5. 위험 · 대응

| 위험 | 발생 신호 | 대응 |
|---|---|---|
| **대비 자동 검사에서 특정 다크/라이트 보조 색이 4.5:1 미세 미달** | 예: `4.38:1` | `10-design-system.md §2` 원칙대로 토큰 값만 1단계 조정(`#6B6B68 → #5E5E5B` 전례) 후 재검증 |
| **1시간 대기 후 WebView2 투명도 상실(`2.2`)** | 첫 진입 시 순간 검은 사각형 깜빡임 | `overlay.rs`의 `show_raw` 직전 `set_background_color(0,0,0,0)` + DWM `DwmExtendFrameIntoClientArea` 재적용 보강 |
| **혼합 배율(100%+150%)에서 결과 섬이 옆 모니터에 걸침(`R-win-regress`)** | `show_island`의 `set_position` 좌표 밀림 | Windows `show_island`에서 `outer_size()`가 이전 모니터 배율 크기일 수 있음 → 먼저 대상 모니터로 대략 이동한 뒤 `outer_size()`를 다시 읽어 중앙 정렬(2-step placement) |

---

## 6. 작업일 · 난이도 · 먼저 끝나야 할 것

| 항목 | 항목일 (환산 남은 일) | 난이도 | 선행 |
|---|---:|---|---|
| `0.4` · `0.4b` · `0.4c` (단계 0 디자인 확인) | 0.75 (0.15) | 하 | 없음 |
| `1.3` · `1.6` (단계 1 UI·속도 확인) | 0.63 (0.125) | 하 | 없음 |
| `2.2` · `2.5` · `2.6` (단계 2 안정성·계측 확인) | 1.05 (0.21) | 하 | Windows 실기 |
| `R-win-regress` (혼합 배율·결과 섬 확인) | 0.25 (0.05) | 하 | Windows 실기 |
| **묶음 합계** | **2.68일 (0.54일)** | **하** | 이미 구현 완료 — 단계 7(**G**) 전 수시 마감 |
