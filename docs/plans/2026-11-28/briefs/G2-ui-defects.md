# G2 — 2단계 화면 결함 고치기(공통 화면) — Antigravity

---

[일감 G2-ui-defects — 두 OS 화면 확인에서 나온 공통 화면 결함 4묶음 (PRD-3차 §5.2)]
사용자에게 보이는 보고와 모든 출력은 한국어로 쓴다(커밋 메시지·주석·문서 포함, 코드 식별자·명령·경로만 원문). 맡은 범위는 묻지 않고 끝내고, 자기 브랜치에 커밋·푸시까지 한다.

작업 폴더(꼭 따로): 저장소 /Users/bodyking/Desktop/Architecture/screen-ink 에서
  git fetch origin && git worktree add ../screen-ink-wt/g2-ui -b r3/g2-ui origin/feat/foundation
그 worktree 안에서만 고친다. 끝나면 `git push -u origin r3/g2-ui`. feat/foundation에 직접 푸시하지 않는다. 앱을 띄우거나 키·마우스를 보내지 않는다(실기는 총괄이 한다).

먼저 읽기: docs/qa/2026-11-28/win-screen-check.md "공통 결함" 표, docs/qa/2026-11-28/mac-screen-check.md, docs/qa/2026-11-28/screen-checklist.md 해당 줄, docs/prd/04-screens.md S08·S10·S14.

고칠 것(각각 순수 모델 함수 + 시험을 먼저, 화면은 그 함수를 쓰게):
1. #13 크기 배지 겹침: 범위가 화면 아래쪽이라 캡처 도구 막대가 범위 **위**로 갈 때 범위 왼쪽 위 크기 배지(`W × H`)를 막대가 덮는다 → 막대가 위로 가면 배지를 범위 안 왼쪽 위(또는 막대와 안 겹치는 자리)로 옮긴다. 배치 계산은 packages/ink-ui/src/lib/models/captureToolbar.js(또는 새 모델)에 순수 함수로.
2. #15 색·굵기 패널이 그릴 자리를 가림: 600×370 안팎 범위에서 패널이 막대 위로 열려 범위 왼쪽 위를 넓게 덮는다 → 범위와 겹치지 않는 자리(범위 옆·아래 바깥 → 그래도 안 되면 겹침이 가장 작은 자리)에 열린다. 모니터 가장자리 8px 안쪽 유지. 순수 함수 + 시험.
3. #17·#27 F2 창 강조 모양(S10): 지금은 F1과 같은 네 귀 꺾쇠 + 왼쪽 위 배지. 명세는 **창 둘레 주황 2px 테두리(창 둥근 모서리를 따라감) + 오른쪽 위 주황 크기 꼬리표**. F2(창 고르기) 세션에서만 그 모양으로 그린다(F1은 그대로). 둥글기는 OS 기본값(맥 약 10pt·Windows 11 8px)을 쓰되 상수로 둔다. packages/ink-ui/src/lib/components/SelectionFrame.svelte에 모양 변형(예: `variant="window"`)을 더하고 CaptureOverlay.svelte가 F2 세션 종류(`sessionKind`)로 고르게.
4. #18 칠판 쪽 넘기기(S14): 칠판(Alt+3/⌥3)에서 PgDn = 새 쪽(빈 칠판)·다음 쪽, PgUp = 이전 쪽, 쪽 표시 `n/m`이 바뀐다. 쪽마다 획을 따로 보관, 다 지우기는 지금 쪽만. 그리기(화면 위)에서는 PgDn/PgUp을 쓰지 않는다(아래 앱 발표 넘기기와 겹치지 않게 — 그리기 모드 키 가로채기는 기존 규칙 그대로). 엔진(packages/ink-ui/src/lib/ink/*)·DrawOverlay.svelte·그 모델 시험. 복사·저장은 지금 쪽만.
고칠 범위: packages/ink-ui/src/lib/screens/CaptureOverlay.svelte·DrawOverlay.svelte, components/SelectionFrame.svelte·ColorWidthPanel.svelte·CaptureToolbar.svelte, models/captureToolbar.js(+ 새 모델 파일), ink/*, 새 문구가 필요하면 i18n/areas/*.ko.json·*.en.json(영역 파일만 고치고 `node scripts/i18n-build.mjs`로 합침). 고치지 않는 곳: crates/**(Codex X3가 고치는 중), packages/ink-ui/src/lib/App.svelte·Island.svelte·IslandHost.svelte·settings 화면·models/settingsSearch.js(총괄이 고치는 중).
검사(이 맥의 자원 규칙): /Users/bodyking/.local/bin/resource-guard run --profile test --wait-seconds 600 -- bash -c 'cd packages/ink-ui && npm test && npm run check' · node scripts/i18n-build.mjs 뒤 git diff가 영역·합본 둘 다인지.
커밋: 한국어 메시지, 끝에 빈 줄 뒤 `Co-Authored-By: Gemini (antigravity) <noreply@google.com>`. force·--no-verify 금지.
끝 보고: 4묶음 각각 바꾼 것·새 시험, 검사 결과(안 돌린 것은 "미실행"), 브랜치·커밋 해시, 총괄이 실기로 볼 것.
