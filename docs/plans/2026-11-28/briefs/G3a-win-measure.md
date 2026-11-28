# G3a — 3단계 Windows 측정 묶음 오케스트레이션(Antigravity 네온)

---

[일감 G3a-win-measure — 1시간 대기 뒤 첫 진입·4K 판서 복사 멈칫·회귀 확인을 Windows 노드로 (PRD-3차 §5.3, 진척 2.2·R-win-regress·4.7)]
사용자에게 보이는 보고와 모든 출력은 한국어로 쓴다(커밋 메시지·주석·문서 포함, 코드 식별자·명령·경로만 원문). 맡은 범위는 묻지 않고 끝내고, 자기 브랜치에 커밋·푸시까지 한다. 착오 없이: 아래 순서를 그대로 지키고, 지어낸 수치·해시를 쓰지 않는다(모두 Windows 보고·파일에서 옮긴다).

작업 폴더: 저장소 /Users/bodyking/Desktop/Architecture/screen-ink 에서
  git fetch origin && git worktree add ../screen-ink-wt/g3a -b r3/g3a origin/feat/foundation
그 worktree 안에서만 일한다. 끝나면 `git push -u origin r3/g3a`. feat/foundation에 직접 푸시하지 않는다. docs/progress/progress.json은 고치지 않는다(총괄이 고친다).

먼저 읽기: docs/plans/2026-11-28/PRD-3차-마무리.md §2·§5.3·§6, docs/handoff/windows-remote-setup.md(§3 일감 받기·§4 화면 도우미·§5 머리말), docs/plans/2026-11-28/briefs/README.md·send-win.sh·W20~W26 브리프(형식 본보기), docs/benchmark/2026-11-27-windows.md(M1·첫 강조 측정법), docs/benchmark/2026-11-28-windows-idle.md, docs/benchmark/2026-11-28-windows-highlight.md.

할 일:
1. Windows 일감 브리프 `docs/plans/2026-11-28/briefs/W27-win-measure.md`를 기존 브리프 형식(맨 위 설명 → `---` → 본문)으로 써서 worktree에 커밋한다. 본문에 담을 것:
   - 앱: origin/feat/foundation(0.3.2 이후) 릴리스 빌드 사본을 C:\agents\runs\W27-win-measure\app\ 에서 포터블 설정으로. Snipaste가 F1·F3을 쥐고 있으니 시험 사본에서만 Ctrl+Alt+F1·Ctrl+Alt+F3로 바꿔 쓴다(W24와 같이). 혼자 측정(C:\agents\status.ps1로 확인).
   - (2.2·R-win-regress) 앱을 켠 뒤 **60분 동안 아무 입력 없이** 둔다 → 첫 캡처 진입의 `overlay ready`·첫 강조·검은 화면 여부를 4K(주)·FHD(오른쪽) 각각 한 번씩(커서를 그 모니터에 미리 두고), 이어서 모니터마다 20회 진입 p95. 목표: 대기 뒤 첫 진입 ≤ 150 ms, p95 ≤ 150 ms, 검은 화면 0. 대기 중 메모리(합산 작업 집합)도 10분마다 기록.
   - (4.7) 4K 모니터 그리기(Alt+1)에 획 몇 개를 그린 뒤 복사(Ctrl+C)·저장(Ctrl+S, 시험 폴더)을 각각 10회 — UI 멈춤(메시지 루프 응답, bench/recog/rb20-watch.ps1 방식)과 결과 파일 크기. 목표: UI 멈춤 ≤ 100 ms.
   - 문서 docs/benchmark/2026-11-28-windows-p3-measure.md(조건·판 해시·n·표), 커밋은 문서만. 끝 보고는 표와 판정.
2. 보낸다(worktree 루트에서): `sh docs/plans/2026-11-28/briefs/send-win.sh W27-win-measure standard` — 끝날 때까지 기다린다(1.5~2시간). 그동안 다른 Windows 일감을 보내지 않는다.
3. 받기: `~/.local/bin/win-agent fetch W27-win-measure` → `git log origin/feat/foundation..refs/remotes/win/W27-win-measure`의 커밋을 r3/g3a에 cherry-pick → 문서를 읽어 수치가 보고와 같은지 대조 → `sh scripts/check-windows.sh`(코드가 바뀐 경우만) → 푸시.
4. Windows 정리(꼭): `ssh win 'git -C C:\dev\screen-ink worktree remove --force C:\dev\wt\W27-win-measure; git -C C:\dev\screen-ink branch -D win/W27-win-measure; Unregister-ScheduledTask -TaskName agent-W27-win-measure -Confirm:$false'` 그리고 맥에서 `git update-ref -d refs/remotes/win/W27-win-measure`. 끝으로 `ssh win 'git -C C:\dev\screen-ink worktree list'`가 원본 한 줄인지 확인.
5. 목표에 못 닿은 항목이 있으면 원인 가설과 다음 일감 제안을 문서 끝에 적는다(고치지는 않는다).
검사: 문서만이면 코드 검사 미실행. 맥에서 무거운 명령을 돌릴 때는 /Users/bodyking/.local/bin/resource-guard run --profile test --wait-seconds 600 -- bash -c '...'.
커밋: 한국어 메시지, 끝에 빈 줄 뒤 `Co-Authored-By: Gemini (antigravity) <noreply@google.com>`(Windows 커밋은 그대로 cherry-pick). force·--no-verify 금지.
끝 보고: 항목별 목표·결과·판정 표, 브랜치·커밋 해시, Windows 정리 결과, 미실행·사람 확인 필요.
