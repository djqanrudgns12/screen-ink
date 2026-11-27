#!/bin/sh
# Windows 노드에 3차 일감 보내기: 공통 머리말(windows-remote-setup.md §5 코드 블록) + 브리프 → win-agent.
# 사용: sh docs/plans/2026-11-28/briefs/send-win.sh W20-highlight4k deep
#   (끝날 때까지 기다리므로 Bash 백그라운드로 돌린다. 동시에 최대 2개, 측정 일감은 혼자.)
set -eu
cd "$(dirname "$0")/../../../.."
JOB=$1; TIER=${2:-standard}
BRIEF="docs/plans/2026-11-28/briefs/$JOB.md"
[ -f "$BRIEF" ] || { echo "브리프가 없어요: $BRIEF"; exit 1; }
{
  awk '/^## 5\./{f=1} /^## 6\./{f=0} f' docs/handoff/windows-remote-setup.md | awk '/^```$/{c++; next} c==1'
  echo
  sed '1,/^---$/d' "$BRIEF"
} | "$HOME/.local/bin/win-agent" run "$JOB" codex "$TIER"
