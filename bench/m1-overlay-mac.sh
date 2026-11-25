#!/bin/sh
# M1 진입 시간 측정(맥판, bench/m1-overlay.ps1과 같은 집계). 목표: p50 <= 80 ms, p95 <= 150 ms.
# F1로 들어가고 Esc로 나오기를 N번 하고(bench/mac/keys.swift, CGEvent), 그동안 앱 로그에 남은
# "overlay ready [N ms] (capture N, show N, windows N ms/…" 줄을 모은다.
#
# 준비(사람이 한 번):
#   - screen-ink가 기본 F1 키로 실행 중(npm run app:dev)이고 화면 기록 권한이 켜져 있다
#   - 이 스크립트를 돌리는 앱(터미널 또는 Claude Code)에 "손쉬운 사용" 권한(CGEvent로 키를 보내기 때문)
#   - F1은 전역 단축키라 운영체제가 screen-ink에만 보낸다. Esc는 키 창인 오버레이가 받는다
#
#   sh bench/m1-overlay-mac.sh            # 100번
#   sh bench/m1-overlay-mac.sh 30         # 30번
set -eu
COUNT="${1:-100}"
HOLD="${HOLD:-0.35}"
GAP="${GAP:-0.35}"
LOG="${LOG:-$HOME/Library/Logs/kr.screenink.desktop/screen-ink.log}"

pgrep -x screen-ink >/dev/null || { echo "screen-ink가 실행 중이 아니에요" >&2; exit 1; }
[ -f "$LOG" ] || { echo "로그가 없어요: $LOG" >&2; exit 1; }

before=$(wc -l < "$LOG")
swift "$(dirname "$0")/mac/keys.swift" "$COUNT" "$HOLD" "$GAP"
sleep 0.5

tail -n +"$((before + 1))" "$LOG" | python3 -c '
import re, sys
rows = [tuple(map(float, m.groups())) for m in (re.search(r"overlay ready \[(\d+) ms\] \(capture (\d+), show (\d+), windows (\d+) ms", l) for l in sys.stdin) if m]
if not rows:
    sys.exit("overlay ready 줄이 없어요 — 권한·단축키를 확인하세요")
def pct(v, p):
    v = sorted(v); k = (len(v) - 1) * p / 100; f = int(k); c = min(f + 1, len(v) - 1)
    return v[f] + (v[c] - v[f]) * (k - f)
names = ["total", "capture", "show", "windows"]
print(f"samples {len(rows)}")
for i, n in enumerate(names):
    col = [r[i] for r in rows]
    print(f"{n:8} p50 {pct(col, 50):6.1f}  p95 {pct(col, 95):6.1f}  max {max(col):6.1f} ms")
t = [r[0] for r in rows]
print("판정:", "통과" if pct(t, 50) <= 80 and pct(t, 95) <= 150 else "미달", "(p50 <= 80, p95 <= 150)")
'
