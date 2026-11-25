# 2026-11-25 학교 PC — 앱별 UI 요소 노출 실측

- 도구: [`bench/uia-probe.ps1`](../uia-probe.ps1)(읽기 전용). 앱마다 맨 위 창 하나를 6×5 격자 30점으로 나눠, 각 점에서 UIA `AutomationElement.FromPoint`(관리 코드 UIA 클라이언트)와 MSAA `AccessibleObjectFromPoint`가 돌려준 요소의 **사각형 크기·컨트롤 종류·걸린 시간**만 적었다. 창 제목·요소 이름·값은 읽지 않았다. 다른 창에 가려진 점은 뺐다.
- "작은 요소 비율" = 돌려받은 요소가 창 넓이의 절반보다 작은 점의 비율. 창 단위보다 작은 조각을 찾을 수 있는지를 대략 본다.
- 대상: 그 시각 떠 있던 앱 중 개인 정보가 없는 것만(Chrome · Tidy Task). 비밀번호 관리자·메신저·보안 에이전트 창은 재지 않았다. 탐색기·설정 앱은 그 시각 보이는 창이 없어 빠졌다.

| 앱 | 창 클래스 | 점 | UIA 작은 요소 | UIA 첫 질의 | UIA 이후 p50 / p95 | MSAA 작은 요소 | MSAA p50 | UIA가 준 종류 |
|---|---|---|---|---|---|---|---|---|
| Chrome | `Chrome_WidgetWin_1` | 30 | 20% | **120.9 ms**(2회차 47.8 ms) | 1.3 / 2.1 ms | 20~23% | 2.0 ms | Group 22~24 · Button 5 · Document 0~2 · ToolBar 1 |
| Tidy Task(Tauri · WebView2) | `Tauri Window` | 30 | **0%** | 1.2 ms | 0.8 / 1.2 ms | **0%** | 1.4 ms | Pane 30(웹 내용 없음) |

같은 시각 창 그림자: `GetWindowRect` 폭 − DWM 확장 프레임 폭 = **16 px**(Chrome·Claude 데스크톱 앱).

## 읽을 점

1. **웹 화면에서는 접근성 트리만으로 조각을 못 찾는다.** Chrome에서 점의 80%가 창 절반 이상을 덮는 큰 Group을 돌려받았다. MSAA도 비슷했다.
2. **WebView2 앱(Tauri·새 Teams·새 Outlook 류)은 UIA·MSAA 모두 창 하나(Pane)만 보인다.** 이런 앱에서는 Snipaste(MSAA만 씀, [08 §1](../../docs/prd/08-smart-capture.md))도, UIA만 쓰는 도구도 창 단위밖에 못 고른다.
3. **Chrome 첫 질의는 100 ms 넘게 걸리고 그 뒤에는 1~2 ms다.** Chromium이 접근성 트리를 처음 요청받을 때 만들기 때문으로 보인다 [추정]. 첫 강조가 늦거나 비는 원인 → 단축키를 받는 순간 미리 깨운다(08 §4.7).
4. 창 경계는 DWM 확장 프레임으로 잡아야 한다(그림자 16 px).

→ 화면 이미지 분석을 1급 근거로 두는 설계의 실측 근거([08-smart-capture](../../docs/prd/08-smart-capture.md)).
