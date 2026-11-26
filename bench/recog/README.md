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

## 재생

```powershell
cargo test -p ink-platform-fake --test replay                    # CI: 기준선보다 떨어지면 실패
cargo run -p ink-platform-fake --bin recog-replay                 # 점별 CSV → bench/results/recog-replay.csv
```

`cases/baseline.json` = `{ "<사례 id>": <Top-1로 맞힌 점 수> }`. 로직을 고쳐 좋아졌으면 숫자를 올린다(내리지 않는다).

## 판정

- 맞음 = 네 변 모두 ±2 px 안(`HIT_TOLERANCE`). 경계 오차 = 맞힌 사각형의 가장 큰 변 오차.
- 칸 수 = 점으로 옮긴 첫 강조가 정답이면 0, 아니면 휠 위·아래 3칸 안에서 처음 맞은 칸. Top-1 = 0칸, Top-3 = 2칸 이내.
- 점마다 새로 진입한다(깊이 기억이 섞이지 않게). UIA 색인은 진입 때 이미 다 와 있는 것으로 친다.
