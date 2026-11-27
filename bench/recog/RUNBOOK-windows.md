# 인식 마무리 — 메인이 실제 화면에서 돌릴 것 (C 묶음, 흐름 win-recog)

일꾼은 화면을 쓰지 않았다. 아래는 **메인 세션이 사용자에게 미리 묻고** 학교 PC(또는 배율 125%·150% PC)에서 돌리는 순서다.
모든 명령은 저장소 뿌리에서, PowerShell 기준.

## 0. 공통 준비 (5분)

- **화면에는 시험 문서·시험 앱만.** 메신저·메일·학생·교직원 정보·실제 업무 화면·사용자 이름이 보이는 창은 닫거나 최소화.
  작업 표시줄 알림 끄기(집중 지원). 사례 PNG는 모든 모니터가 통째로 담긴다 — 커밋 전 눈으로 확인(08 §5.1, C §5).
- 빌드 폴더는 흐름별(예: `$env:CARGO_TARGET_DIR = 'D:\dev-cache\screen-ink\target-main'`), `$env:CARGO_HTTP_CHECK_REVOKE = 'false'`.
- 시험 도구 빌드(저장소 작업 공간 밖 독립 크레이트, 릴리스 패키지에 넣지 않음):
  ```powershell
  foreach ($t in 'rb04-win32','rb20-frozen','snipaste-diff') { cargo build --release --manifest-path "bench\recog\tools\$t\Cargo.toml" }
  $bin = if ($env:CARGO_TARGET_DIR) { "$env:CARGO_TARGET_DIR\release" } else { 'bench\recog\tools\<도구>\target\release' }
  ```
  백신이 막으면 `target\release`에서 직접 실행(C §5).

## 1. `3a-rb1516` RB-15·16 실제 창 시험 (2분)

```powershell
cargo test -p ink-platform-windows --test rb15_16 real:: -- --ignored --test-threads=1 --nocapture
```
- 1초 안쪽으로 시험 창·투명 덮개가 뜨고 최대화 창이 포커스를 잠깐 가져간다(그동안 클릭하지 않기).
- 기대: 2개 통과(`real::` = 실제 창 시험만, 순수 판정은 CI가 돈다). ③ 바탕 화면은 모니터마다 그 작업 영역 후보로 들어가야 한다.
- 실패하면 메시지에 고른 창의 실행 파일·클래스가 나온다(사용자의 항상 위 창이 끼었는지 확인).

## 2. `3b-dpi`·`3b-msaa` 실측 — 배율 125% 또는 150% 모니터가 있는 PC (10분)

스파이크 보고서 §7 [확인 필요] 세 가지를 시험 셋으로 만들어 두었다(`crates/ink-platform-windows/tests/dpi_live.rs`).
이 PC(두 모니터 100%)에서는 보정 경로가 검증되지 않으므로 **배율이 다른 PC**에서 돌린다. 배율을 바꾸는 것은 시스템 변경이라 사용자에게 먼저 묻는다.

1. 시험 창을 배율 125%·150% 모니터에 띄운다.
   ```powershell
   Start-Process "$bin\rb04-win32.exe" -ArgumentList '--dpi','unaware','--out',"$PWD\rb04-truth.json"
   Start-Process "$bin\rb20-frozen.exe" -ArgumentList '--mode','slow-msaa','--slow-ms','0','--life','300'
   ```
   창을 그 모니터로 끌어 놓은 뒤 1초 기다린다(rb04가 정답 파일을 다시 씀).
2. 시험:
   ```powershell
   cargo test -p ink-platform-windows --test dpi_live -- --ignored --test-threads=1 --nocapture
   ```
   | 시험 | 확인하는 것 | 기대 |
   |---|---|---|
   | `dpi_창_경계는_물리_px` | `GetWindowRect`·DWM 경계(PMv2 호출자) = 물리 | 창 경계 = rb04 정답 ±1 px |
   | `dpi_uia_사각형은_물리_px` | DPI 모르는 창의 UIA `BoundingRectangle` = 물리 | 컨트롤 80% 이상 ±2 px |
   | `dpi_자체_msaa_서버의_논리_좌표를_물리로` | 자체 IAccessible(`accLocation`)이 논리 px → `msaa::Msaa`가 물리로 | 칸 크기 = 16 × 배율 px, 클라이언트 안 |
   출력의 `인식 Unaware · 창 DPI 96 · 모니터 DPI 144 · 비 1.500` 줄을 보고서에 옮긴다.
3. 같은 것을 `--dpi system`(rb04)으로 한 번 더 — System-aware 창은 시스템 DPI ≠ 모니터 DPI인 모니터에서만 가상화된다.
4. 앱에서 눈으로: `capture.evidence: true`(설정 파일)로 `npm run app:dev` → F1 → RB-20 위에서 회색 칸과 근거 테두리(MSAA만 잡은 요소는 청록 `M`)가 겹치는지.
5. 스파이크 도구(`spikes\dpi`·`spikes\msaa`)도 같은 창으로 다시 돌려 숫자를 맞춰 본다.

## 3. `3f-frozen` RB-20 멈춘 앱 (모드마다 2분, 합 8분)

2026-11-27 실제 창 측정은 [Windows 인식 기준선](../../docs/benchmark/2026-11-27-windows-recog.md)에 있다.
UIA에서 MSAA로 이어지는 경로를 재현하려면 시험 앱에 `--empty-uia`를 추가한다.
Snipaste가 F1을 쓰는 노드에서는 시험용 슉쇽 설정에 빈 키를 지정한다. F13이면 감시 스크립트에
`-HotkeyVk 124 -ExitWithHotkey`를 준다. 같은 단축키로 캡처를 끝내면 포커스와 관계없이 종료 때 이동 통계를 읽을 수 있다.

```powershell
Start-Process "$bin\rb20-frozen.exe" -ArgumentList '--mode','hang-location','--life','90'
npm run app:dev            # 다른 창
powershell -ExecutionPolicy Bypass -File bench\recog\rb20-watch.ps1 -Moves 50
```
- 스크립트가 커서를 RB-20 창으로 옮기고 F1 → 50번 이동 → Esc, 그동안 screen-ink 창들에 `WM_NULL`(100 ms 제한)을 16 ms마다 보낸다.
- 합격: `100 ms 초과 0회`. 앱 로그에서 `selection probe … moves max` < 100 ms, `hang-location`이면 `msaa deadline 100 ms exceeded` 한 번.
- 모드 `hung` · `infinite-msaa` · `slow-msaa --slow-ms 5`도 같은 명령으로. `hung`은 3초마다 10초 멈추니 `-Moves 50`이면 약 4초.
- 끝나면 `--life`초 뒤 RB-20이 스스로 꺼진다(급하면 `Stop-Process -Name rb20-frozen`).

## 4. `3f-docs`·`3f-win-cases` 사례 뜨기 17종 + 기준선 (45~60분)

17종 = RB-01~14 · 17 · 18 · 19(RB-15·16은 1번 Win32 시험, RB-20은 3번 멈춤 시험으로 따로). 절차는 [README.md](README.md) "사례 뜨기"·"자동 정답".

| RB | 띄울 것 | 정답 |
|---|---|---|
| 01·02·03·19 | `pages\rb01.html` — Chrome·Edge·WebView2·Firefox, 확대 100%, 최대화 | `recog-truth … rb01.html "RB-0x …"` |
| 04 | `rb04-win32.exe`(기본 pmv2) | `rb04-win32 merge <사례.json> rb04-truth.json "RB-04 Win32"` |
| 12 | 배율 다른 모니터에서 RB-01·RB-04 한 번 더 | 위와 같음 |
| 14 | `pages\rb14.html` — Chrome 확대 100%, 최대화 | `recog-truth … rb14.html "RB-14 캔버스·WebGL"` |
| 05~11·13·17·18 | 탐색기·설정/계산기·PPT·워드/엑셀·한글 시험 문서·Edge PDF·메뉴/툴팁·작업 관리자·시작 메뉴·다른 가상 데스크톱 | 사람이 F1에서 정답 범위를 확정(README) |

- `note`에 `RB-xx …`를 꼭 적는다(G3 `--filter`가 이것으로 고른다).
- 다 뜬 뒤: `cargo run -p ink-platform-fake --bin recog-replay` → `bench\results\recog-replay.csv`, 사례별 Top-1 맞힌 점 수를 `cases\baseline.json`에 적는다(내리지 않음). `cargo test -p ink-platform-fake --test replay`가 통과하는지.

## 5. `3c-g3` 관문 G3 — V 혼자 RB-01·03 (2분)

```powershell
cargo run -p ink-platform-fake --bin recog-replay -- --vision-only --filter RB-01,RB-03 --min-top1 80
```
- A(UIA)·H(자식 창)를 지우고 V만(창 목록 W는 남김). `판정: … 통과`면 G3 통과, 미달이면 종료 코드 1.
- 미달 시(C §3): V 문턱(`GRAD_THRESHOLD`·`MIN_SEGMENT_LEN`·`MIN_SUPPORT`)을 재생으로 최대 1일 조정, 그래도 미달이면 웹/WebView2 목표 75%로 재조정(사용자 결정).

## 6. `3f-snipaste` Snipaste 대조 + G4 보고서 (보정 2분 + 사례당 약 1분)

screen-ink는 끄고 Snipaste 2.11.3 기본 설정(창·요소 감지 켬). **사례를 뜬 화면을 그대로** 둔 채:
```powershell
# ① 테두리 보정: RB-04 사례(창 정답 점이 있음)로 한 번
powershell -ExecutionPolicy Bypass -File bench\recog\compare-snipaste.ps1 -Case bench\recog\cases\<RB-04 사례>.json -Calibrate
# ② 사례마다(보정 값을 -Inset으로)
powershell -ExecutionPolicy Bypass -File bench\recog\compare-snipaste.ps1 -Case bench\recog\cases\<사례>.json -Inset <보정 값>
```
- 결과: `bench\results\recog-<날짜>-snipaste-<사례>.csv` + 같은 이름 `.md`(Snipaste Top-1 · screen-ink Top-1/Top-3 · Snipaste 실패 점 중 screen-ink가 맞힌 비율).
- G4: RB 전체 Top-1 ≥ 90% · Top-3 ≥ 98% · 경계 ≤ 1 px(재생) + Snipaste 이상 + Snipaste 실패 점 ≥ 70% 맞춤 → 사례별 `.md`를 모아 `bench\results\<날짜>-g4.md`로.
- 점 하나에 약 1.3초(커서 이동·F1·Esc를 스크립트가 넣음), 도는 동안 손대지 않기. 전·후 PNG는 `%TEMP%\screen-ink-snipaste`에만 쓰고 끝나면 지운다.

## 7. `3e-paste` 한글·PowerPoint 붙여 넣기 (10분)

1. `pages\paste-pattern.html`을 Chrome 확대 100%로 연다(배율과 상관없이 400×200 물리 px).
2. screen-ink F1 → 패턴의 검은 테두리에 맞춰 강조(휠·드래그로 400×200 확인) → Enter(복사).
3. 한글 새 문서에 Ctrl+V → 그림을 PNG로 저장(그림 위 오른쪽 클릭 → 그림 저장). PowerPoint 빈 슬라이드에 Ctrl+V → 그림으로 저장(PNG).
4. 확인:
   ```powershell
   powershell -ExecutionPolicy Bypass -File bench\recog\check-paste.ps1 -Png <한글 PNG> -App 한글
   powershell -ExecutionPolicy Bypass -File bench\recog\check-paste.ps1 -Png <PPT PNG> -App PowerPoint
   ```
   크기(400×200 또는 같은 비율)·세 색 차 0·위아래 테두리 검정(뒤집힘 없음). 문서 안 표시 크기(PPT 10.58 × 5.29 cm = 96 DPI)도 적는다.
5. 결과를 `bench\results\<날짜>-paste-check.md`에. 뒤집힘·색 차가 나오면 `clipboard.rs`의 `CF_DIB` `biHeight` 부호·알파를 본다(C §5).

## 걸리는 시간 합

| 순서 | 일 | 시간 |
|---|---|---:|
| 0 | 준비·도구 빌드 | 5분 |
| 1 | RB-15·16 | 2분 |
| 2 | DPI·MSAA 실측(배율 PC) | 10분 |
| 3 | RB-20 네 모드 | 8분 |
| 4 | 사례 17종·기준선 | 45~60분 |
| 5 | G3 | 2분 |
| 6 | Snipaste 대조(사례 17개) | 20~25분 |
| 7 | 붙여 넣기 | 10분 |
| | **합** | **약 1.7~2시간** |
