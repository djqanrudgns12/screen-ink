# 인식 시험 사례 (08 §5.2 스냅숏 재생)

진입 순간의 정지 이미지(모니터별 무손실 PNG)·창 목록·UIA 색인·자식 창을 **창 제목·글자 없이** 떠 둔 것과,
그 화면의 커서 점마다의 정답 사각형. `ink-platform-fake::replay`가 OS 없이 진짜 서비스·고르기·융합·V 엔진으로
다시 돌려 Top-1·Top-3·경계 오차를 잰다.

> **시험 문서 화면에서만 뜬다.** 학생·교직원 정보, 실제 업무 화면, 메신저·메일, 사용자 이름이 보이는 화면 금지(08 §5.1).
> PNG는 화면 전체가 그대로 담긴다 — 넣기 전에 눈으로 확인한다.

## 사례 뜨기 (사람 정답 도구)

1. 시험 문서(우리가 만든 HTML·HWP·PPT 등)만 화면에 띄운다. 다른 창은 최소화.
2. 사례 폴더를 주고 앱을 띄운다.
   ```powershell
   $env:SCREEN_INK_CASE_DIR = "$PWD\bench\recog\cases"; npm run app:dev
   ```
3. F1 → 정답으로 칠 조각에 커서 → 휠·드래그로 **정답 사각형**을 맞춘 뒤 Enter.
   확정할 때마다 `case-<날짜>-<시각>-<n>.json` + `*.m<모니터>.png`가 생긴다(점 1개, 정답 = 확정한 사각형,
   점 = 강조를 고른 커서 자리, 드래그면 가운데).
4. JSON의 `note`에 무엇을 띄운 화면인지(예: `RB-04 Win32 시험 앱`) 적는다. 같은 화면의 점은 한 파일의 `points`로 합쳐도 된다.

## 지정한 시험 창만 수집하는 방법

다른 창이 열려 있는 Windows 작업 노드에서는 시험 창의 PID와 HWND를 지정해 수집할 수 있다.
아래 도구는 WGC로 해당 창만 찍고 실제 모니터의 물리 좌표에 붙인다. 창 밖은 흰색이다.
창 안에서 일부만 저장하려면 메모 뒤에 창 왼쪽 위 기준의 `x y 너비 높이`(물리 px)를 준다.
접근성 색인·자식 창 좌표는 시험 창에서 읽는다. 정답은 수집 후 아래 절차로 붙인다.

```powershell
cargo build -p ink-platform-windows --example recog-window
Copy-Item "$env:CARGO_TARGET_DIR\debug\examples\recog-window.exe" "$작업사본\recog-window.exe"
$env:SCREEN_INK_CASE_DIR = $사례폴더
& "$작업사본\recog-window.exe" rb04 $시험앱.Id $시험앱.MainWindowHandle.ToInt64() "RB-04 Win32 표준 컨트롤"
```

RB-18에서는 메모 뒤에 `--cloaked PID HWND`를 주면 DWM 숨김 속성을 확인하고 두 시험 창을 실제 Z 순서로 보존한다.
수집된 HWND·PID는 재생용 번호로 치환한다. 창 제목·접근성 이름은 기록하지 않는다.
2026-11-27의 화면 범위와 항목별 정답 출처는 [Windows 실제 창 인식 보고서](../../docs/benchmark/2026-11-27-windows-recog.md)에 있다.

## 자동 정답 (RB-01·02·03·12·14·19 — 우리 시험 문서)

시험 문서 `pages/*.html`은 배치(CSS px)를 `<script type="application/json" id="layout">`에 담고, 왼쪽 위에 2×2 색 표지(자홍·청록 / 노랑·검정)를 그린다.
정지 이미지에서 표지를 찾으면 문서 원점과 배율(브라우저 틀·확대·Windows 배율 포함)을 알 수 있어서, 정답을 사람이 그리지 않아도 된다(`ink-platform-fake::truth`).

1. **모든 모니터에 시험 문서 말고는 아무것도 없게 한다**(사례 PNG에 모든 모니터가 담긴다).
2. 브라우저(RB-01 Chrome · 02 Edge · 03 WebView2 · 19 Firefox)로 `pages/rb01.html`을 열고 확대 100%, 창 최대화. 표지가 가려지지 않게 둔다.
3. 사례 폴더를 주고 앱을 띄운 뒤 F1 → 아무 곳 Enter(확정 1번 = 사례 1개).
4. 정답 채우기:
   ```powershell
   cargo run -p ink-platform-fake --bin recog-truth -- bench/recog/cases/<사례>.json bench/recog/pages/rb01.html "RB-01 Chrome 100%"
   ```
   → `모니터 N, 배율 1.000, 정답 점 32개`. 사례의 `points`가 문서의 모든 정답(그릇은 빈 자리 점)으로 바뀐다.
5. RB-12(혼합 배율)는 배율이 다른 모니터에서 2~4를 한 번 더.

## 자동 정답 — RB-14(캔버스·WebGL·영상)와 RB-04(Win32 시험 앱)

- **RB-14**: `pages/rb14.html`도 같은 표지·layout 규칙이다(2D 캔버스·WebGL 가위 상자·움직이는 16:9 영상 틀). 위 2~4를 `rb14.html`로.
  콘솔의 `window.__rb14Truth()`는 정답(CSS px) 확인용.
- **RB-04**: Win32 시험 앱이 자기 컨트롤의 물리 px 정답을 직접 쓴다.
  ```powershell
  cargo build --release --manifest-path bench/recog/tools/rb04-win32/Cargo.toml
  <target>eleaseb04-win32.exe --dpi pmv2 --out rb04-truth.json     # 창을 옮기면 다시 씀
  # F1 → Enter로 사례를 뜬 뒤
  <target>eleaseb04-win32.exe merge bench/recog/cases/<사례>.json rb04-truth.json "RB-04 Win32 100%"
  ```
  `--dpi unaware|system`은 D12(DPI 모르는 창) 시험용 — 정답은 언제나 물리 px.

## 시험 도구 (`tools/`, 저장소 작업 공간 밖 · 릴리스에 넣지 않음)

| 도구 | 하는 일 |
|---|---|
| `rb04-win32` | RB-04 Win32 표준 컨트롤 시험 앱 + 정답 JSON + `merge` |
| `rb20-frozen` | RB-20 멈춘 시험 앱: `--mode hung · infinite-msaa · hang-location · slow-msaa`, `--life`초 뒤 스스로 끝남 |
| `snipaste-diff` | Snipaste F1 전·후 PNG → 강조 사각형(`ink-vision::diff_rect`) — `compare-snipaste.ps1`이 부름 |

화면에서 돌리는 순서·명령·시간(RB-15·16 실제 창, DPI 실측, RB-20, 사례 17종, G3, Snipaste 대조, 붙여 넣기)은
[RUNBOOK-windows.md](RUNBOOK-windows.md).

## 재생

```powershell
cargo test -p ink-platform-fake --test replay                    # CI: 기준선보다 떨어지면 실패
cargo run -p ink-platform-fake --bin recog-replay                 # 점별 CSV → bench/results/recog-replay.csv
cargo run -p ink-platform-fake --bin recog-replay -- --vision-only --filter RB-01,RB-03 --min-top1 80   # 관문 G3(V만)
```

`cases/baseline.json` = `{ "<사례 id>": <Top-1로 맞힌 점 수> }`. 로직을 고쳐 좋아졌으면 숫자를 올린다(내리지 않는다).

### 새 장면(hold-out) 판정

287점 기준선 과적합을 확인하기 위한 시험 전용 사례는 `bench/recog/holdout/`에 따로 둔다([holdout/README.md](holdout/README.md)).

```powershell
cargo run -p ink-platform-fake --bin recog-replay -- --holdout               # → bench/results/recog-holdout.csv
cargo run -p ink-platform-fake --bin recog-replay -- --holdout --min-top1 80 # 80% 관문 판정
```

- **보지 않는 규칙**: 인식 규칙·문턱값을 고칠 때 `holdout/` 사례를 보거나 맞추지 않는다.
- `baseline.json`과 섞지 않고 기준선 검사 없이 Top-1·Top-3·경계 오차만 보고한다. CI(`.github/workflows/ci.yml`) 기존 재생 시험은 `cases/`만 돈다.

## 판정

- 맞음 = 네 변 모두 ±2 px 안(`HIT_TOLERANCE`). 경계 오차 = 맞힌 사각형의 가장 큰 변 오차.
- 칸 수 = 점으로 옮긴 첫 강조가 정답이면 0, 아니면 휠 위·아래 3칸 안에서 처음 맞은 칸. Top-1 = 0칸, Top-3 = 2칸 이내.
- 점마다 새로 진입한다(깊이 기억이 섞이지 않게). UIA 색인은 진입 때 이미 다 와 있는 것으로 친다.
