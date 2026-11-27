# 3차 브리프 — 보내는 순서

계획: [PRD-3차-마무리.md](../PRD-3차-마무리.md) §5·§6. 진척: `docs/progress/progress.json`(항목마다 `phase`).

## 1단계(P1) 보내는 순서

| 차례 | 일감 | 받는 곳 | 진척 id | 같이 돌릴 수 있음 | 보내는 법 |
|---|---|---|---|---|---|
| 1 | [W20 4K 첫 강조](W20-highlight4k.md) | Windows 노드 Codex `deep` | `2.5` | W21 | `sh docs/plans/2026-11-28/briefs/send-win.sh W20-highlight4k deep`(Bash 백그라운드) |
| 1 | [W21 RB-20 굳음](W21-rb20.md) | Windows 노드 Codex `deep` | `3f-frozen` | W20 | `… send-win.sh W21-rb20 deep` |
| 1 | [X1 인식 재생](X1-recog.md) | 브리지 Codex `deep`(Astra high), 자기 worktree | `3-RECOG` | 전부 | `bridge_send` wake |
| 1 | [G1 화면 점검표](G1-checklist.md) | 브리지 Antigravity `complex`, 자기 worktree | `3-CHECKLIST` | 전부 | `bridge_send` wake |
| 1 | 맥 실기 14 + 맥 측정 | 총괄(이 세션) | `M3`·`M4`·`M5`·`M6`·`M9`·`M10`·`6.2`·`R2-KEY-REC`·`R2-PDF`·`1.3`·`5.3`·`7.4`·`M13`·`3-MAC-IDLE` | 전부 | — |
| 2 | [W22 꺾쇠·고정 안내](W22-recframe-pin.md) | Windows 노드 Codex `complex` | `6.4`·`6.5.3` | W23 | W20·W21 합친 뒤 |
| 2 | [W23 대기 메모리](W23-idlemem.md) | Windows 노드 Codex `deep` | `3-MEM-W` | W22(끝 측정은 혼자) | W20 합친 뒤 |

## 받기(합치기)

- Windows: `win-agent fetch <일감>` → `git cherry-pick refs/remotes/win/<일감>`(필요한 커밋만) → `scripts/check-windows.sh` + 바뀐 크레이트 맥 시험 → 푸시 → Windows 정리(worktree·브랜치·예약 작업, `docs/handoff/windows-remote-setup.md` §3-4).
- 브리지(X1·G1): 자기 브랜치 `r3/<일감>`을 `git fetch` → diff 검토 → 바뀐 범위 검사 → `feat/foundation`에 합침 → worktree 정리.
- 합칠 때마다 진척 항목의 `status`·`note`(커밋·수치)를 고치고 `npm run progress`.

## 2단계 이후

W24(Windows 화면 확인) · W25(측정 묶음, 혼자) · W26(새 장면 + 사례) · W27(Snipaste 대조, 혼자) · W28(스토어 스크린숏·설치 실기) · G2(배포 문서·CI 초안) · G3(스토어 글·로고) — 앞 단계 결과를 보고 그때 쓴다(범위가 앞 결과에 달려 있음).
